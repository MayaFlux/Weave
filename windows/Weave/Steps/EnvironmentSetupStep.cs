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

        Task.Run(() => SetupEnvironmentAsync(config, statusLabel, logCallback));
    }

    private async Task SetupEnvironmentAsync(InstallationConfig config, Label statusLabel, Action<string> logCallback)
    {
        try
        {
            await LogAsync("=== Environment Setup ===");
            await LogAsync("");

            // ========================================
            // MayaFlux Environment Variables
            // ========================================
            await LogAsync("=== MayaFlux Configuration ===");
            await LogAsync("");

            await LogAsync("Setting MAYAFLUX_ROOT environment variable...");
            if (ProcessRunner.SetEnvironmentVariable(WeaveConstants.ENV_MAYAFLUX_ROOT, config.MayaFluxRoot, logger))
            {
                await LogAsync($"  [OK] MAYAFLUX_ROOT={config.MayaFluxRoot}");
            }
            else
            {
                await LogAsync($"  [WARN] Failed to set MAYAFLUX_ROOT");
            }

            await LogAsync("Adding MayaFlux to PATH...");
            if (ProcessRunner.AddToPath(config.BinDirectory, logger))
            {
                await LogAsync($"  [OK] Added to PATH: {config.BinDirectory}");
            }
            else
            {
                await LogAsync($"  [WARN] Failed to add to PATH");
            }

            await LogAsync("Setting CMAKE_PREFIX_PATH...");
            if (ProcessRunner.SetEnvironmentVariable(WeaveConstants.ENV_CMAKE_PREFIX_PATH, config.MayaFluxRoot, logger))
            {
                await LogAsync($"  [OK] CMAKE_PREFIX_PATH={config.MayaFluxRoot}");
            }
            else
            {
                await LogAsync($"  [WARN] Failed to set CMAKE_PREFIX_PATH");
            }

            var mayaFluxInclude = Path.Combine(config.MayaFluxRoot, "include");
            ProcessRunner.AddIncludeDirectory(mayaFluxInclude, logger);
            await LogAsync($"  [OK] Added to INCLUDE/CPATH: {mayaFluxInclude}");

            var mayaFluxLib = Path.Combine(config.MayaFluxRoot, "lib");
            ProcessRunner.AddLibraryDirectory(mayaFluxLib, logger);
            await LogAsync($"  [OK] Added to LIB/LIBRARY_PATH: {mayaFluxLib}");

            await LogAsync("");

            // ========================================
            // Dependency Environment Variables
            // ========================================
            await LogAsync("=== Dependency Configuration ===");
            await LogAsync("");

            await SetupDiaSDK();
            await SetupLLVM();
            await SetupVulkan();
            await SetupVcpkg();

            await LogAsync("");
            await LogAsync("=== Environment Setup Complete ===");
            await LogAsync("");
            await LogAsync("[WARN] You must restart your terminal/PowerShell for environment changes to take effect");
            await LogAsync("[INFO] Run: $env:MAYAFLUX_ROOT to verify after restart");
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

    private async Task SetupDiaSDK()
    {
        await LogAsync("Configuring DIA SDK...");

        string[] vsBasePaths = new[]
        {
            @"C:\Program Files (x86)\Microsoft Visual Studio",
            @"C:\Program Files\Microsoft Visual Studio"
        };

        string? diaPath = null;

        foreach (var basePath in vsBasePaths)
        {
            if (!Directory.Exists(basePath)) continue;

            foreach (var year in new[] { "2022", "2019", "2017" })
            {
                foreach (var edition in new[] { "Community", "Professional", "Enterprise" })
                {
                    var candidatePath = Path.Combine(basePath, year, edition, "DIA SDK");
                    var libPath = Path.Combine(candidatePath, "lib", "amd64", "diaguids.lib");

                    if (File.Exists(libPath))
                    {
                        diaPath = candidatePath;
                        break;
                    }
                }
                if (diaPath != null) break;
            }
            if (diaPath != null) break;
        }

        if (diaPath != null)
        {
            if (ProcessRunner.SetEnvironmentVariable("DIA_SDK_PATH", diaPath, logger))
            {
                await LogAsync($"  [OK] DIA SDK: {diaPath}");
            }
            else
            {
                await LogAsync($"  [WARN] Failed to set DIA_SDK_PATH");
            }
        }
        else
        {
            await LogAsync("  [WARN] DIA SDK not found - LLVM may have linking issues");
        }
    }

    private async Task SetupLLVM()
    {
        await LogAsync("Configuring LLVM/Clang...");

        var llvmVersion = "21.1.8";
        var llvmRoot = $@"C:\Program Files\LLVM_Libs\{llvmVersion}";

        if (Directory.Exists(llvmRoot))
        {
            ProcessRunner.SetEnvironmentVariable("LLVM_ROOT", llvmRoot, logger);
            ProcessRunner.SetEnvironmentVariable("LLVM_DIR", Path.Combine(llvmRoot, "lib", "cmake", "llvm"), logger);
            ProcessRunner.SetEnvironmentVariable("Clang_DIR", Path.Combine(llvmRoot, "lib", "cmake", "clang"), logger);
            await LogAsync($"  [OK] LLVM v{llvmVersion}: {llvmRoot}");
        }
        else
        {
            await LogAsync($"  [WARN] LLVM v{llvmVersion} not found at {llvmRoot}");
        }
    }

    private async Task SetupVulkan()
    {
        await LogAsync("Configuring Vulkan SDK...");

        var vulkanBase = @"C:\VulkanSDK";

        if (Directory.Exists(vulkanBase))
        {
            var versionDirs = Directory.GetDirectories(vulkanBase).OrderByDescending(d => d).ToArray();

            if (versionDirs.Length > 0)
            {
                var vulkanPath = versionDirs[0];
                var includePath = Path.Combine(vulkanPath, "Include");

                ProcessRunner.SetEnvironmentVariable("VULKAN_SDK", vulkanPath, logger);
                ProcessRunner.SetEnvironmentVariable("VK_SDK_PATH", vulkanPath, logger);

                ProcessRunner.AppendToEnvironmentVariable("CPATH", includePath, logger);

                await LogAsync($"  [OK] Vulkan SDK: {vulkanPath}");
                await LogAsync($"  [OK] Added to CPATH: {includePath}");
            }
            else
            {
                await LogAsync($"  [WARN] No Vulkan SDK version found in {vulkanBase}");
            }
        }
        else
        {
            await LogAsync($"  [WARN] Vulkan SDK not found at {vulkanBase}");
        }
    }

    private async Task SetupVcpkg()
    {
        await LogAsync("Configuring vcpkg...");

        var vcpkgRoot = Environment.GetEnvironmentVariable("VCPKG_ROOT", EnvironmentVariableTarget.Machine);
        if (string.IsNullOrEmpty(vcpkgRoot))
        {
            vcpkgRoot = @"C:\vcpkg";
        }

        if (Directory.Exists(vcpkgRoot))
        {
            await LogAsync($"  [OK] vcpkg root: {vcpkgRoot}");
            
            var installedDir = Path.Combine(vcpkgRoot, "installed", "x64-windows");
            if (Directory.Exists(installedDir))
            {
                await LogAsync($"  [OK] vcpkg packages installed to: {installedDir}");
                await LogAsync("      MayaFluxConfig.cmake handles dependency resolution automatically");
            }
            else
            {
                await LogAsync($"  [WARN] vcpkg packages not found at: {installedDir}");
            }
        }
        else
        {
            await LogAsync($"  [WARN] vcpkg not found at {vcpkgRoot}");
            await LogAsync("         Dependencies may not build correctly");
        }
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
        {
            nextButton.Invoke(new Action(() =>
            {
                nextButton.Enabled = true;
            }));
        }
    }
}
