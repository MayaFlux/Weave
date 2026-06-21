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
    private Logger logger = new();
    private bool setupSuccess = false;
    private TextBox? logBox;
    private Button? nextButton;

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

        Task.Run(() => SetupEnvironmentAsync(config, statusLabel));
    }

    private async Task SetupEnvironmentAsync(InstallationConfig config, Label statusLabel)
    {
        try
        {
            await LogAsync("=== Environment Setup ===");
            await LogAsync("");

            await LogAsync("=== MayaFlux Configuration ===");
            await LogAsync("");

            await LogAsync("Setting MAYAFLUX_ROOT...");
            if (ProcessRunner.SetEnvironmentVariable(WeaveConstants.ENV_MAYAFLUX_ROOT, config.MayaFluxRoot, logger))
                await LogAsync($"  [OK] MAYAFLUX_ROOT={config.MayaFluxRoot}");
            else
                await LogAsync("  [WARN] Failed to set MAYAFLUX_ROOT");

            await LogAsync("Adding MayaFlux to PATH...");
            if (ProcessRunner.AddToPath(config.BinDirectory, logger))
                await LogAsync($"  [OK] Added to PATH: {config.BinDirectory}");
            else
                await LogAsync("  [WARN] Failed to add to PATH");

            ProcessRunner.AddIncludeDirectory(Path.Combine(config.MayaFluxRoot, "include"), logger);
            await LogAsync($"  [OK] Added to INCLUDE/CPATH: {Path.Combine(config.MayaFluxRoot, "include")}");

            ProcessRunner.AddLibraryDirectory(Path.Combine(config.MayaFluxRoot, "lib"), logger);
            await LogAsync($"  [OK] Added to LIB/LIBRARY_PATH: {Path.Combine(config.MayaFluxRoot, "lib")}");

            await LogAsync("");
            await LogAsync("=== Dependency Configuration ===");
            await LogAsync("");

            await SetupVulkan();
            await SetupLLVM();
            await SetupFFmpeg();

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
            EnableButton();
        }
    }

    private async Task SetupVulkan()
    {
        await LogAsync("Configuring Vulkan SDK...");

        var vulkanBase = @"C:\VulkanSDK";
        if (!Directory.Exists(vulkanBase))
        {
            await LogAsync($"  [WARN] Vulkan SDK not found at {vulkanBase}");
            return;
        }

        var vulkanPath = Directory.GetDirectories(vulkanBase).OrderByDescending(d => d).FirstOrDefault();
        if (vulkanPath == null)
        {
            await LogAsync("  [WARN] No Vulkan SDK version found");
            return;
        }

        ProcessRunner.SetEnvironmentVariable("VULKAN_SDK", vulkanPath, logger);
        ProcessRunner.SetEnvironmentVariable("VK_SDK_PATH", vulkanPath, logger);
        ProcessRunner.AppendToEnvironmentVariable("CPATH", Path.Combine(vulkanPath, "Include"), logger);
        ProcessRunner.AddToPath(Path.Combine(vulkanPath, "Bin"), logger);

        await LogAsync($"  [OK] VULKAN_SDK={vulkanPath}");
    }

    private async Task SetupLLVM()
    {
        await LogAsync("Configuring LLVM...");

        var llvmRoot = @"C:\Program Files\LLVM";
        if (!Directory.Exists(llvmRoot))
        {
            await LogAsync($"  [WARN] LLVM not found at {llvmRoot}");
            return;
        }

        ProcessRunner.SetEnvironmentVariable("LLVM_ROOT", llvmRoot, logger);
        ProcessRunner.SetEnvironmentVariable("LLVM_DIR", Path.Combine(llvmRoot, "lib", "cmake", "llvm"), logger);
        ProcessRunner.SetEnvironmentVariable("Clang_DIR", Path.Combine(llvmRoot, "lib", "cmake", "clang"), logger);
        ProcessRunner.AddToPath(Path.Combine(llvmRoot, "bin"), logger);

        await LogAsync($"  [OK] LLVM_ROOT={llvmRoot}");
    }

    private async Task SetupFFmpeg()
    {
        await LogAsync("Configuring FFmpeg...");

        var ffmpegRoot = DependenciesStep.FindFFmpegRoot();
        if (ffmpegRoot == null)
        {
            await LogAsync("  [WARN] FFmpeg not found in WinGet packages");
            return;
        }

        ProcessRunner.SetEnvironmentVariable("FFMPEG_ROOT", ffmpegRoot, logger);
        ProcessRunner.AddToPath(Path.Combine(ffmpegRoot, "bin"), logger);
        await LogAsync($"  [OK] FFMPEG_ROOT={ffmpegRoot}");
    }

    private async Task LogAsync(string message)
    {
        if (logBox?.Parent != null)
        {
            await logBox.Invoke(new Func<Task>(async () =>
            {
                logBox.AppendText(message + Environment.NewLine);
                await Task.CompletedTask;
            }));
        }
    }

    private void UpdateStatus(Label statusLabel, string text, Color color)
    {
        if (statusLabel.Parent != null)
        {
            statusLabel.Invoke(new Action(() =>
            {
                statusLabel.Text = text;
                statusLabel.ForeColor = color;
            }));
        }
    }

    private void EnableButton()
    {
        if (nextButton?.Parent != null)
            nextButton.Invoke(new Action(() => nextButton.Enabled = true));
    }
}