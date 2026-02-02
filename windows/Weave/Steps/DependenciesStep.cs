using System;
using System.Collections.Generic;
using System.Drawing;
using System.IO;
using System.Threading.Tasks;
using System.Windows.Forms;
using Weave.Modes;
using Weave.Shared.Models;
using Weave.Theme;
using Weave.UI.Layout;
using Weave.Utils;

namespace Weave.UI.Pages;

public class DependenciesStep : IInstallationStep
{
    private Logger logger = new();
    private bool installSuccess = false;
    private TextBox? logBox;
    private Button? nextButton;
    private Button? skipButton;

    public void BuildUI(
        LayoutManager layout,
        InstallationConfig config,
        Action<string> logCallback,
        Action nextCallback)
    {
        layout.AddTitle("Step 3: Install Dependencies");

        var statusLabel = layout.AddStatusLabel("Installing dependencies (this may take 10-30 minutes)...");

        logBox = layout.AddLogBox(LayoutConstants.LogBoxMaxHeight);

        layout.AddFlexibleSpacer();

        nextButton = layout.AddButton("Next >", ThemeColors.ButtonPrimary);
        nextButton.Enabled = false;
        nextButton.Click += (s, e) => nextCallback();

        skipButton = layout.AddButton("Skip", ThemeColors.ButtonSecondary);
        skipButton.Click += (s, e) =>
        {
            installSuccess = true;
            nextCallback();
        };

        Task.Run(() => InstallDependenciesAsync(config, statusLabel, logCallback));
    }

    private async Task InstallDependenciesAsync(InstallationConfig config, Label statusLabel, Action<string> logCallback)
    {
        try
        {
            await LogAsync("=== Dependency Installation ===");
            await LogAsync("");

            await LogAsync("This will install:");
            await LogAsync("  • System Tools: CMake, Git, Ninja, 7-Zip");
            await LogAsync("  • Winget Packages: FFmpeg, Vulkan SDK");
            await LogAsync("  • Binary Package: LLVM 21.1.8");
            await LogAsync("  • vcpkg Packages: glm, eigen3, magic-enum, stb, hidapi, glfw3, rtaudio, rtmidi");
            await LogAsync("");
            await LogAsync("Installation time: 10-30 minutes depending on internet speed");
            await LogAsync("");

            await LogAsync("Extracting dependency installer...");
            ResourceExtractor.ExtractAllResources(config.MayaFluxRoot);

            string scriptPath = Path.Combine(config.ScriptsDirectory, "install_package.ps1");
            string packagesFile = Path.Combine(config.ScriptsDirectory, "packages.psd1");

            if (!File.Exists(scriptPath))
            {
                await LogAsync("ERROR: install_package.ps1 not found after extraction");
                await LogAsync("Extracted to: " + config.ScriptsDirectory);
                await LogAsync("Skipping dependency installation");
                UpdateStatus(statusLabel, "Dependencies not found - skipping", ThemeColors.Warning);
                EnableButtons();
                return;
            }

            if (!File.Exists(packagesFile))
            {
                await LogAsync("ERROR: packages.psd1 not found after extraction");
                await LogAsync("Skipping dependency installation");
                UpdateStatus(statusLabel, "Packages file not found - skipping", ThemeColors.Warning);
                EnableButtons();
                return;
            }

            await LogAsync("Found dependency installer at: " + scriptPath);
            await LogAsync("");
            await LogAsync("Running dependency installer...");

            var args = new List<string>
            {
                "-PackagesFile", packagesFile
            };

            int exitCode = await ProcessRunner.RunPowerShellScriptAsync(
                scriptPath,
                args,
                logger,
                msg => LogAsync(msg).GetAwaiter().GetResult()
            );

            if (exitCode == 0)
            {
                await LogAsync("");
                await LogAsync("OK: Dependencies installed successfully");
                installSuccess = true;
                UpdateStatus(statusLabel, "Dependencies installed", ThemeColors.Success);
            }
            else
            {
                await LogAsync("");
                await LogAsync("INFO: Dependency installation exited with code: " + exitCode);
                await LogAsync("INFO: You may need to manually install dependencies");
                await LogAsync("INFO: Continuing with installation anyway...");
                installSuccess = true;
                UpdateStatus(statusLabel, "Dependencies installation completed with warnings", ThemeColors.Warning);
            }
        }
        catch (Exception ex)
        {
            await LogAsync("ERROR: " + ex.Message);
            await LogAsync("INFO: Continuing with installation anyway...");
            installSuccess = true;
            UpdateStatus(statusLabel, "Dependencies skipped - continuing", ThemeColors.Warning);
        }
        finally
        {
            EnableButtons();
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

    private void EnableButtons()
    {
        if (nextButton?.Parent != null)
        {
            nextButton.Invoke(new Action(() =>
            {
                nextButton.Enabled = true;
            }));
        }

        if (skipButton?.Parent != null)
        {
            skipButton.Invoke(new Action(() =>
            {
                skipButton.Enabled = true;
            }));
        }
    }
}
