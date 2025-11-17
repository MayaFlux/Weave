using Weave.Shared.Models;
using Weave.Modes;
using Weave.Utils;

namespace Weave.UI.Pages;

public class DependenciesStep : IInstallationStep
{
    private Logger logger = new();
    private bool installSuccess = false;

    public Panel CreateUI(InstallationConfig config, Action<string> logCallback, Action nextCallback, InstallationMode parent)
    {
        var panel = new Panel { BackColor = Color.White, Padding = new Padding(20) };

        var titleLabel = new Label
        {
            Text = "Step 3: Install Dependencies",
            Font = new Font("Segoe UI", 14, FontStyle.Bold),
            AutoSize = true,
            Location = new Point(0, 0)
        };
        panel.Controls.Add(titleLabel);

        var statusLabel = new Label
        {
            Text = "Installing dependencies (this may take 10-30 minutes)...",
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

        var skipButton = new Button
        {
            Text = "Skip",
            Width = 100,
            Height = 40,
            Location = new Point(panel.Width - 240, panel.Height - 60),
            BackColor = Color.FromArgb(200, 200, 200),
            ForeColor = Color.Black,
            FlatStyle = FlatStyle.Flat
        };
        skipButton.Click += (s, e) => { installSuccess = true; nextCallback(); };
        panel.Controls.Add(skipButton);

        Task.Run(() => InstallDependenciesAsync(
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
                    statusLabel.Text = installSuccess ? "Dependencies installed!" : "Installation failed or skipped";
                    statusLabel.ForeColor = installSuccess ? Color.Green : Color.Orange;
                }));
            }
        ));

        return panel;
    }

    private async Task InstallDependenciesAsync(InstallationConfig config, Action<string> log, Action onComplete)
    {
        try
        {
            var scriptDir = Path.Combine(config.MayaFluxRoot, "share", "weave", "scripts");
            var scriptPath = Path.Combine(scriptDir, "install_package.ps1");
            var packagesFile = Path.Combine(scriptDir, "packages.psd1");

            if (!File.Exists(scriptPath))
            {
                log("[ERROR] Dependency installer not found at: " + scriptPath);
                log("[INFO] Extracted scripts may not be available yet");
                onComplete?.Invoke();
                return;
            }

            log("Running dependency installer...");
            log($"Script: {scriptPath}");

            var args = new List<string>
            {
                "-PackagesFile", packagesFile
            };

            var exitCode = await ProcessRunner.RunPowerShellScriptAsync(scriptPath, args, logger, msg => log(msg));

            if (exitCode == 0)
            {
                log("[OK] Dependencies installed successfully");
                installSuccess = true;
            }
            else
            {
                log($"[WARN] Dependency installation exited with code: {exitCode}");
                log("[INFO] You may need to manually install dependencies");
            }
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