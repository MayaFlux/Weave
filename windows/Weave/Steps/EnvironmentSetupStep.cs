using Weave.Shared;
using Weave.Shared.Models;
using Weave.Modes;
using Weave.Utils;

namespace Weave.UI.Pages;

public class EnvironmentSetupStep : IInstallationStep
{
    private Logger logger = new();
    private bool setupSuccess = false;

    public Panel CreateUI(InstallationConfig config, Action<string> logCallback, Action nextCallback, InstallationMode parent)
    {
        var panel = new Panel { BackColor = Color.White, Padding = new Padding(20) };

        var titleLabel = new Label
        {
            Text = "Step 4: Environment Setup",
            Font = new Font("Segoe UI", 14, FontStyle.Bold),
            AutoSize = true,
            Location = new Point(0, 0)
        };
        panel.Controls.Add(titleLabel);

        var statusLabel = new Label
        {
            Text = "Configuring environment variables...",
            Font = new Font("Segoe UI", 10),
            AutoSize = true,
            Location = new Point(0, 40)
        };
        panel.Controls.Add(statusLabel);

        var logBox = new TextBox
        {
            Multiline = true,
            ReadOnly = true,
            Font = new Font("Consolas", 9),
            BackColor = Color.FromArgb(31, 31, 31),
            ForeColor = Color.FromArgb(220, 220, 220),
            Location = new Point(0, 70),
            Width = panel.Width - 40,
            Height = 300,
            ScrollBars = ScrollBars.Vertical
        };
        panel.Controls.Add(logBox);

        var nextButton = new Button
        {
            Text = "Next >",
            Width = 100,
            Height = 40,
            Location = new Point(panel.Width - 120, panel.Height - 60),
            BackColor = Color.FromArgb(0, 120, 215),
            ForeColor = Color.White,
            FlatStyle = FlatStyle.Flat,
            Enabled = false
        };
        nextButton.Click += (s, e) => nextCallback();
        panel.Controls.Add(nextButton);

        Task.Run(() => SetupEnvironmentAsync(
            config,
            msg =>
            {
                panel.Invoke(new Action(() =>
                {
                    logBox.AppendText(msg + Environment.NewLine);
                    logCallback(msg);
                }));
            },
            () =>
            {
                panel.Invoke(new Action(() =>
                {
                    nextButton.Enabled = true;
                    statusLabel.Text = setupSuccess ? "Environment configured!" : "Setup completed with warnings";
                    statusLabel.ForeColor = setupSuccess ? Color.Green : Color.Orange;
                }));
            }
        ));

        return panel;
    }

    private async Task SetupEnvironmentAsync(InstallationConfig config, Action<string> log, Action onComplete)
    {
        try
        {
            log("Setting MAYAFLUX_ROOT environment variable...");
            if (ProcessRunner.SetEnvironmentVariable(WeaveConstants.ENV_MAYAFLUX_ROOT, config.MayaFluxRoot, logger))
            {
                log($"[OK] MAYAFLUX_ROOT={config.MayaFluxRoot}");
            }

            log("");
            log("Adding MayaFlux to PATH...");
            if (ProcessRunner.AddToPath(config.BinDirectory, logger))
            {
                log($"[OK] Added to PATH: {config.BinDirectory}");
            }

            log("");
            log("Setting CMAKE_PREFIX_PATH...");
            if (ProcessRunner.SetEnvironmentVariable(WeaveConstants.ENV_CMAKE_PREFIX_PATH, config.MayaFluxRoot, logger))
            {
                log($"[OK] CMAKE_PREFIX_PATH={config.MayaFluxRoot}");
            }

            log("");
            log("[OK] Environment variables configured");
            log("[WARN] You must restart your terminal for changes to take effect");
            setupSuccess = true;
        }
        catch (Exception ex)
        {
            log($"[ERROR] {ex.Message}");
        }
        finally
        {
            onComplete?.Invoke();
        }
    }
}