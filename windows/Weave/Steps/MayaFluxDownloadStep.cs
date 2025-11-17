using Weave.Shared.Models;
using Weave.Modes;
using Weave.Utils;

namespace Weave.UI.Pages;

public class MayaFluxDownloadStep : IInstallationStep
{
    private Logger logger = new();
    private bool downloadSuccess = false;

    public Panel CreateUI(InstallationConfig config, Action<string> logCallback, Action nextCallback, InstallationMode parent)
    {
        var panel = new Panel { BackColor = Color.White, Padding = new Padding(20) };

        var titleLabel = new Label
        {
            Text = "Step 2: Download MayaFlux",
            Font = new Font("Segoe UI", 14, FontStyle.Bold),
            AutoSize = true,
            Location = new Point(0, 0)
        };
        panel.Controls.Add(titleLabel);

        var statusLabel = new Label
        {
            Text = "Downloading...",
            Font = new Font("Segoe UI", 10),
            AutoSize = true,
            Location = new Point(0, 40)
        };
        panel.Controls.Add(statusLabel);

        var progressBar = new ProgressBar
        {
            Location = new Point(0, 70),
            Width = panel.Width - 40,
            Height = 20
        };
        panel.Controls.Add(progressBar);

        var logBox = new TextBox
        {
            Multiline = true,
            ReadOnly = true,
            Font = new Font("Consolas", 9),
            BackColor = Color.FromArgb(31, 31, 31),
            ForeColor = Color.FromArgb(220, 220, 220),
            Location = new Point(0, 100),
            Width = panel.Width - 40,
            Height = 270,
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
            Text = "Cancel",
            Width = 100,
            Height = 40,
            Location = new Point(panel.Width - 240, panel.Height - 60),
            BackColor = Color.FromArgb(200, 200, 200),
            ForeColor = Color.Black,
            FlatStyle = FlatStyle.Flat
        };
        skipButton.Click += (s, e) => Application.Exit();
        panel.Controls.Add(skipButton);

        // Check if already installed
        if (File.Exists(Path.Combine(config.MayaFluxRoot, "lib", "MayaFluxLib.lib")))
        {
            logBox.Text = "[OK] MayaFlux already installed\r\n[INFO] Skipping download";
            statusLabel.Text = "MayaFlux already installed";
            statusLabel.ForeColor = Color.Green;
            nextButton.Enabled = true;
            config.SkipDownload = true;
            return panel;
        }

        // Start download
        Task.Run(() => DownloadMayaFluxAsync(
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
                    nextButton.Enabled = downloadSuccess;
                    statusLabel.Text = downloadSuccess ? "Download complete!" : "Download failed";
                    statusLabel.ForeColor = downloadSuccess ? Color.Green : Color.Red;
                }));
            }
        ));

        return panel;
    }

    private async Task DownloadMayaFluxAsync(InstallationConfig config, Action<string> log, Action onComplete)
    {
        try
        {
            var api = new GithubApi(logger);
            var release = await api.GetLatestReleaseAsync();

            if (release == null)
            {
                log("[ERROR] Failed to fetch latest release");
                onComplete?.Invoke();
                return;
            }

            var asset = release.GetWindowsAsset();
            if (asset == null)
            {
                log("[ERROR] No Windows asset found in release");
                onComplete?.Invoke();
                return;
            }

            log($"Asset: {asset.Name}");
            Directory.CreateDirectory(config.TempDirectory);

            var downloadPath = Path.Combine(config.TempDirectory, asset.Name);
            var success = await FileOperations.DownloadFileAsync(asset.DownloadUrl, downloadPath, logger, (downloaded, total) =>
            {
                var percent = (int)((downloaded * 100) / total);
                log($"Progress: {percent}%");
            });

            if (!success)
            {
                log("[ERROR] Download failed");
                onComplete?.Invoke();
                return;
            }

            log("");
            log("Extracting MayaFlux...");
            Directory.CreateDirectory(config.MayaFluxRoot);

            if (!FileOperations.Extract7z(downloadPath, config.MayaFluxRoot, logger))
            {
                log("[ERROR] Extraction failed");
                onComplete?.Invoke();
                return;
            }

            log("");
            if (!FileOperations.VerifyInstallation(config.MayaFluxRoot, logger))
            {
                log("[ERROR] Installation verification failed");
                onComplete?.Invoke();
                return;
            }

            log("[OK] MayaFlux downloaded and extracted successfully");
            downloadSuccess = true;
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