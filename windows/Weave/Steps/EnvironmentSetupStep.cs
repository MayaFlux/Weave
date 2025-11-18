using System;
using System.Drawing;
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

            await LogAsync("Setting MAYAFLUX_ROOT environment variable...");
            if (ProcessRunner.SetEnvironmentVariable(WeaveConstants.ENV_MAYAFLUX_ROOT, config.MayaFluxRoot, logger))
            {
                await LogAsync($"[OK] MAYAFLUX_ROOT={config.MayaFluxRoot}");
            }
            else
            {
                await LogAsync($"[WARN] Failed to set MAYAFLUX_ROOT");
            }

            await LogAsync("");

            await LogAsync("Adding MayaFlux to PATH...");
            if (ProcessRunner.AddToPath(config.BinDirectory, logger))
            {
                await LogAsync($"[OK] Added to PATH: {config.BinDirectory}");
            }
            else
            {
                await LogAsync($"[WARN] Failed to add to PATH");
            }

            await LogAsync("");

            await LogAsync("Setting CMAKE_PREFIX_PATH...");
            if (ProcessRunner.SetEnvironmentVariable(WeaveConstants.ENV_CMAKE_PREFIX_PATH, config.MayaFluxRoot, logger))
            {
                await LogAsync($"[OK] CMAKE_PREFIX_PATH={config.MayaFluxRoot}");
            }
            else
            {
                await LogAsync($"[WARN] Failed to set CMAKE_PREFIX_PATH");
            }

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