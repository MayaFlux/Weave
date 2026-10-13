using System;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using System.Windows.Forms;
using Weave.Modes;
using Weave.Shared;
using Weave.Shared.Models;
using Weave.Theme;
using Weave.UI.Layout;
using Weave.Utils;

namespace Weave.UI.Pages;

public class EnvironmentSetupStep : IInstallationStep
{
    private readonly EnvironmentSetupHost host;
    private readonly Logger logger;
    internal event Action<string>? MessageLogged;
    internal event Action<bool>? ProceedChanged;

    public EnvironmentSetupStep() : this(new EnvironmentSetupHost()) { }
    internal EnvironmentSetupStep(EnvironmentSetupHost host)
    {
        this.host = host;
        logger = host.CreateLogger();
    }
    private bool setupSuccess = false;
    private TextBox? logBox;
    private Button? nextButton;
    private Button? retryButton;
    private int running;

    public void BuildUI(
        LayoutManager layout,
        InstallationConfig config,
        Action<string> logCallback,
        Action nextCallback)
    {
        layout.AddTitle("Step 4: Environment Setup");

        var statusLabel = layout.AddStatusLabel("Configuring environment variables...");

        logBox = layout.AddLogBox(LayoutConstants.LogBoxMaxHeight);

        layout.AddFlexibleSpacer();

        (nextButton, var cancelButton) = layout.AddButtonPair("Next >", "Cancel");
        nextButton.Enabled = false;
        nextButton.Click += (s, e) => nextCallback();
        cancelButton.Click += (s, e) => Application.Exit();
        retryButton = layout.AddButton("Retry", ThemeColors.ButtonSecondary);
        retryButton.Enabled = false;
        retryButton.Click += (s, e) => Task.Run(() => SetupEnvironmentAsync(config, statusLabel));

        Task.Run(() => SetupEnvironmentAsync(config, statusLabel));
    }

    internal async Task SetupEnvironmentAsync(InstallationConfig config, Label? statusLabel = null)
    {
        if (System.Threading.Interlocked.CompareExchange(ref running, 1, 0) != 0) return;
        setupSuccess = false;
        EnableButton();
        try
        {
            RequireDirectory(config.MayaFluxRoot);
            RequireDirectory(config.BinDirectory);
            RequireDirectory(Path.Combine(config.MayaFluxRoot, "include"));
            RequireDirectory(config.LibDirectory);
            await LogAsync("=== Environment Setup ===");
            await LogAsync("");

            await LogAsync("=== MayaFlux Configuration ===");
            await LogAsync("");

            await LogAsync("Setting MAYAFLUX_ROOT...");
            if (ProcessRunner.SetEnvironmentVariable(WeaveConstants.ENV_MAYAFLUX_ROOT, config.MayaFluxRoot, logger, host))
                await LogAsync($"  [OK] MAYAFLUX_ROOT={config.MayaFluxRoot}");
            else
                throw new InvalidOperationException("Failed to set MAYAFLUX_ROOT. Check permissions and retry environment setup.");

            await LogAsync("Adding MayaFlux to PATH...");
            if (ProcessRunner.AddToPath(config.BinDirectory, logger, host))
                await LogAsync($"  [OK] Added to PATH: {config.BinDirectory}");
            else
                throw new InvalidOperationException("Failed to update PATH. Check permissions and retry environment setup.");

            RequireWrite(ProcessRunner.AddIncludeDirectory(Path.Combine(config.MayaFluxRoot, "include"), logger, host), "CPATH");
            await LogAsync($"  [OK] Added to CPATH: {Path.Combine(config.MayaFluxRoot, "include")}");

            RequireWrite(ProcessRunner.AddLibraryDirectory(Path.Combine(config.MayaFluxRoot, "lib"), logger, host), "LIBRARY_PATH");
            await LogAsync($"  [OK] Added to LIBRARY_PATH: {Path.Combine(config.MayaFluxRoot, "lib")}");

            await LogAsync("");
            await LogAsync("=== Dependency Configuration ===");
            await LogAsync("");

            await SetupVulkan(config);
            await SetupLLVM(config);
            await SetupFFmpeg(config);

            await LogAsync("");
            await LogAsync("=== Environment Setup Complete ===");
            await LogAsync("");
            await LogAsync("[WARN] Restart your terminal for environment changes to take effect");
            await LogAsync("");

            setupSuccess = true;
            UpdateStatus(statusLabel, "Environment configured", ThemeColors.Success);
        }
        catch (Exception ex)
        {
            await LogAsync($"[ERROR] {ex.Message}");
            UpdateStatus(statusLabel, "Setup failed", ThemeColors.Error);
        }
        finally
        {
            System.Threading.Interlocked.Exchange(ref running, 0);
            EnableButton();
        }
    }

    private async Task SetupVulkan(InstallationConfig config)
    {
        await LogAsync("Configuring Vulkan SDK...");

        var vulkanBase = @"C:\VulkanSDK";
        var vulkanPath = SelectedDependencyRoot(config, "KhronosGroup.VulkanSDK");
        if (vulkanPath == null && !host.DirectoryExists(vulkanBase))
        {
            if (!config.SkipDependencies) throw new DirectoryNotFoundException("Vulkan SDK is missing. Run dependency setup again.");
            await LogAsync($"  [WARN] Vulkan SDK not found at {vulkanBase}");
            return;
        }

        vulkanPath ??= WindowsInstallationPaths.OrderVersionDirectories(host.GetDirectories(vulkanBase)).FirstOrDefault();
        if (vulkanPath == null)
        {
            if (!config.SkipDependencies) throw new DirectoryNotFoundException("No Vulkan SDK version found. Run dependency setup again.");
            await LogAsync("  [WARN] No Vulkan SDK version found");
            return;
        }

        RequireDirectory(vulkanPath);
        RequireDirectory(Path.Combine(vulkanPath, "Bin"));
        RequireDirectory(Path.Combine(vulkanPath, "Include"));
        RequireWrite(ProcessRunner.SetEnvironmentVariable("VULKAN_SDK", vulkanPath, logger, host), "VULKAN_SDK");
        RequireWrite(ProcessRunner.SetEnvironmentVariable("VK_SDK_PATH", vulkanPath, logger, host), "VK_SDK_PATH");
        RequireWrite(ProcessRunner.AppendToEnvironmentVariable("CPATH", Path.Combine(vulkanPath, "Include"), logger, host), "CPATH");
        RequireWrite(ProcessRunner.AddToPath(Path.Combine(vulkanPath, "Bin"), logger, host), "PATH");

        await LogAsync($"  [OK] VULKAN_SDK={vulkanPath}");
    }

    private async Task SetupLLVM(InstallationConfig config)
    {
        await LogAsync("Configuring LLVM...");

        var llvmRoot = SelectedDependencyRoot(config, "LLVM.LLVM") ?? @"C:\Program Files\LLVM";
        if (!host.DirectoryExists(llvmRoot))
        {
            if (!config.SkipDependencies || config.DependencyExecutablePaths.ContainsKey("LLVM.LLVM"))
                throw new DirectoryNotFoundException($"LLVM is missing at {llvmRoot}. Run dependency setup again.");
            await LogAsync($"  [WARN] LLVM not found at {llvmRoot}");
            return;
        }

        RequireDirectory(llvmRoot);
        RequireDirectory(Path.Combine(llvmRoot, "bin"));
        RequireWrite(ProcessRunner.SetEnvironmentVariable("LLVM_ROOT", llvmRoot, logger, host), "LLVM_ROOT");
        RequireWrite(ProcessRunner.SetEnvironmentVariable("LLVM_DIR", Path.Combine(llvmRoot, "lib", "cmake", "llvm"), logger, host), "LLVM_DIR");
        RequireWrite(ProcessRunner.SetEnvironmentVariable("Clang_DIR", Path.Combine(llvmRoot, "lib", "cmake", "clang"), logger, host), "Clang_DIR");
        RequireWrite(ProcessRunner.AddToPath(Path.Combine(llvmRoot, "bin"), logger, host), "PATH");

        await LogAsync($"  [OK] LLVM_ROOT={llvmRoot}");
    }

    private async Task SetupFFmpeg(InstallationConfig config)
    {
        await LogAsync("Configuring FFmpeg...");

        var ffmpegRoot = SelectedDependencyRoot(config, "Gyan.FFmpeg.Shared") ?? DependenciesStep.FindFFmpegRoot(host);
        if (ffmpegRoot == null)
        {
            throw new InvalidOperationException($"FFmpeg Shared was not found under {DependenciesStep.FFmpegInstallRoot}. Run dependency setup again.");
        }

        RequireDirectory(ffmpegRoot);
        RequireDirectory(Path.Combine(ffmpegRoot, "bin"));
        if (!ProcessRunner.SetEnvironmentVariable("FFMPEG_ROOT", ffmpegRoot, logger, host))
            throw new InvalidOperationException("Failed to set FFMPEG_ROOT. Run Weave as administrator and retry environment setup.");
        if (!ProcessRunner.AddToPath(Path.Combine(ffmpegRoot, "bin"), logger, host))
            throw new InvalidOperationException("Failed to add FFmpeg Shared to PATH. Retry environment setup.");
        await LogAsync($"  [OK] FFMPEG_ROOT={ffmpegRoot}");
    }

    internal static string? SelectedDependencyRoot(InstallationConfig config, string package)
    {
        if (!config.DependencyExecutablePaths.TryGetValue(package, out var executable)) return null;
        var bin = Path.GetDirectoryName(executable);
        return bin != null && Path.GetFileName(bin).Equals("bin", StringComparison.OrdinalIgnoreCase) ? Path.GetDirectoryName(bin) : null;
    }

    private void RequireDirectory(string path)
    {
        if (!WindowsInstallationPaths.IsSafeEnvironmentPath(path)) throw new ArgumentException($"Invalid environment path: {path}");
        if (!host.DirectoryExists(path)) throw new DirectoryNotFoundException($"Required directory is missing: {path}. Restore it and retry setup.");
    }

    private static void RequireWrite(bool success, string name)
    {
        if (!success) throw new InvalidOperationException($"Failed to update {name}. Check permissions and retry environment setup.");
    }

    private Task LogAsync(string message)
    {
        MessageLogged?.Invoke(message);
        return InstallationStepUi.AppendLogAsync(logBox, message);
    }

    private void UpdateStatus(Label? statusLabel, string text, Color color) => InstallationStepUi.UpdateStatus(statusLabel, text, color);

    private void EnableButton()
    {
        ProceedChanged?.Invoke(setupSuccess && running == 0);
        if (nextButton?.Parent != null)
            nextButton.Invoke(new Action(() => nextButton.Enabled = setupSuccess && running == 0));
        if (retryButton?.Parent != null)
            retryButton.Invoke(new Action(() => retryButton.Enabled = !setupSuccess && running == 0));
    }
}
