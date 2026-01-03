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

            await LogAsync("");

            // ========================================
            // Dependency Environment Variables
            // ========================================
            await LogAsync("=== Dependency Configuration ===");
            await LogAsync("");

            await SetupDiaSDK();

            await SetupLLVM();

            await SetupVulkan();

            await SetupGLFW();

            await SetupFFmpeg();

            await SetupRtAudio();

            await SetupHeaderLibraries();

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

        var llvmRoot = @"C:\Program Files\LLVM_Libs";

        if (Directory.Exists(llvmRoot))
        {
            ProcessRunner.SetEnvironmentVariable("LLVM_ROOT", llvmRoot, logger);
            ProcessRunner.SetEnvironmentVariable("LLVM_DIR", Path.Combine(llvmRoot, "lib", "cmake", "llvm"), logger);
            ProcessRunner.SetEnvironmentVariable("Clang_DIR", Path.Combine(llvmRoot, "lib", "cmake", "clang"), logger);
            await LogAsync($"  [OK] LLVM environment configured");
        }
        else
        {
            await LogAsync($"  [WARN] LLVM not found at {llvmRoot}");
        }
    }

    private async Task SetupVulkan()
    {
        await LogAsync("Configuring Vulkan SDK...");

        var vulkanBase = @"C:\VulkanSDK";

        if (Directory.Exists(vulkanBase))
        {
            var versionDirs = Directory.GetDirectories(vulkanBase);
            if (versionDirs.Length > 0)
            {
                var vulkanPath = versionDirs[0];
                ProcessRunner.SetEnvironmentVariable("VULKAN_SDK", vulkanPath, logger);
                ProcessRunner.SetEnvironmentVariable("VK_SDK_PATH", vulkanPath, logger);
                await LogAsync($"  [OK] Vulkan SDK: {vulkanPath}");
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

    private async Task SetupGLFW()
    {
        await LogAsync("Configuring GLFW...");

        var glfwRoot = @"C:\Program Files\GLFW";

        if (Directory.Exists(glfwRoot))
        {
            ProcessRunner.SetEnvironmentVariable("GLFW_ROOT", glfwRoot, logger);

            // Detect lib directory (lib-vc2022, lib-vc2019, etc.)
            string? glfwLibDir = null;
            foreach (var candidate in new[] { "lib-vc2022", "lib-vc2019", "lib-vc2017", "lib-vc2015" })
            {
                var testPath = Path.Combine(glfwRoot, candidate);
                var dllLib = Path.Combine(testPath, "glfw3dll.lib");

                if (File.Exists(dllLib))
                {
                    glfwLibDir = testPath;
                    break;
                }
            }

            if (glfwLibDir != null)
            {
                ProcessRunner.SetEnvironmentVariable("GLFW_LIB_DIR", glfwLibDir, logger);
                await LogAsync($"  [OK] GLFW library: {glfwLibDir}");
            }
            else
            {
                await LogAsync($"  [WARN] GLFW lib directory not found");
            }
        }
        else
        {
            await LogAsync($"  [WARN] GLFW not found at {glfwRoot}");
        }
    }

    private async Task SetupFFmpeg()
    {
        await LogAsync("Configuring FFmpeg...");

        var ffmpegRoot = @"C:\Program Files\FFmpeg";

        if (Directory.Exists(ffmpegRoot))
        {
            ProcessRunner.SetEnvironmentVariable("FFMPEG_ROOT", ffmpegRoot, logger);
            await LogAsync($"  [OK] FFmpeg: {ffmpegRoot}");
        }
        else
        {
            await LogAsync($"  [WARN] FFmpeg not found at {ffmpegRoot}");
        }
    }

    private async Task SetupRtAudio()
    {
        await LogAsync("Configuring RtAudio...");

        var rtaudioRoot = @"C:\Program Files\RtAudio";

        if (Directory.Exists(rtaudioRoot))
        {
            ProcessRunner.SetEnvironmentVariable("RTAUDIO_ROOT", rtaudioRoot, logger);
            await LogAsync($"  [OK] RtAudio: {rtaudioRoot}");
        }
        else
        {
            await LogAsync($"  [WARN] RtAudio not found at {rtaudioRoot}");
        }
    }

    private async Task SetupHeaderLibraries()
    {
        await LogAsync("Configuring header-only libraries...");

        var headerLibs = new (string EnvVar, string Path)[]
        {
            ("EIGEN3_INCLUDE_DIR", @"C:\Program Files\Eigen3"),
            ("GLM_INCLUDE_DIR", @"C:\Program Files\glm\include"),
            ("STB_INCLUDE_DIR", @"C:\Program Files\stb\include"),
            ("MAGIC_ENUM_INCLUDE_DIR", @"C:\Program Files\magic_enum\include"),
            ("LIBXML2_INCLUDE_DIR", @"C:\Program Files\LibXml2\include")
        };

        int foundCount = 0;
        int totalCount = headerLibs.Length;

        foreach (var (envVar, path) in headerLibs)
        {
            if (Directory.Exists(path))
            {
                ProcessRunner.SetEnvironmentVariable(envVar, path, logger);
                foundCount++;
            }
        }

        await LogAsync($"  [OK] Header libraries configured: {foundCount}/{totalCount}");

        if (foundCount < totalCount)
        {
            await LogAsync($"  [WARN] Some header libraries not found - builds may fail");
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