using System;
using System.IO;
using System.Threading.Tasks;
using System.Windows.Forms;
using System.Drawing;
using Weave.Shared.Models;
using Weave.Modes;
using Weave.Utils;
using Weave.Theme;

namespace Weave.UI.Pages;

public class MayaFluxDownloadStep : IInstallationStep
{
    private Logger logger = new();
    private bool downloadSuccess = false;

    public Panel CreateUI(InstallationConfig config, Action<string> logCallback, Action nextCallback, InstallationMode parent)
    {
        var panel = new Panel
        {
            Padding = new Padding(LayoutConstants.MarginMedium),
            AutoScroll = true
        };
        panel.ApplyDarkTheme();

        int currentY = LayoutConstants.MarginMedium;

        currentY = LayoutHelper.AddTitle(panel, "Step 2: Download MayaFlux", currentY);

        var statusLabel = new Label
        {
            Text = "Downloading...",
            Font = new Font("Segoe UI", 10),
            ForeColor = ThemeColors.TextSecondary,
            BackColor = ThemeColors.BackgroundDark,
            AutoSize = true,
            Location = new Point(LayoutConstants.MarginMedium, currentY)
        };
        panel.Controls.Add(statusLabel);
        currentY += statusLabel.Height + LayoutConstants.SpacingLarge;

        var progressBar = new ProgressBar
        {
            Location = new Point(LayoutConstants.MarginMedium, currentY),
            Width = panel.Width - (LayoutConstants.MarginMedium * 2),
            Height = 20
        };
        panel.Controls.Add(progressBar);
        currentY += progressBar.Height + LayoutConstants.SpacingLarge;

        var logBox = new TextBox
        {
            Multiline = true,
            ReadOnly = true,
            Font = new Font("Consolas", 9),
            BackColor = ThemeColors.BackgroundMedium,
            ForeColor = ThemeColors.TextPrimary,
            Location = new Point(LayoutConstants.MarginMedium, currentY),
            Width = panel.Width - (LayoutConstants.MarginMedium * 2),
            Height = LayoutConstants.LogBoxMaxHeight,
            ScrollBars = ScrollBars.Vertical
        };
        panel.Controls.Add(logBox);

        var nextButton = new Button
        {
            Text = "Next >",
            Width = 100,
            Height = LayoutConstants.ButtonHeight,
            BackColor = ThemeColors.ButtonPrimary,
            ForeColor = Color.White,
            FlatStyle = FlatStyle.Flat,
            Enabled = false
        };
        nextButton.Click += (s, e) => nextCallback();

        var cancelButton = new Button
        {
            Text = "Cancel",
            Width = 100,
            Height = LayoutConstants.ButtonHeight,
            BackColor = ThemeColors.ButtonSecondary,
            ForeColor = Color.White,
            FlatStyle = FlatStyle.Flat
        };
        cancelButton.Click += (s, e) => Application.Exit();

        LayoutHelper.PositionButtonsAtBottom(panel, LayoutConstants.ButtonHeight,
            (nextButton, 0),
            (cancelButton, 1)
        );

        panel.Controls.Add(nextButton);
        panel.Controls.Add(cancelButton);

        if (File.Exists(Path.Combine(config.MayaFluxRoot, "lib", "MayaFluxLib.lib")))
        {
            logBox.Text = "[OK] MayaFlux already installed\r\n[INFO] Skipping download";
            statusLabel.Text = "MayaFlux already installed";
            statusLabel.ForeColor = ThemeColors.Success;
            nextButton.Enabled = true;
            config.SkipDownload = true;
            return panel;
        }

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
                    statusLabel.ForeColor = downloadSuccess ? ThemeColors.Success : ThemeColors.Error;
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