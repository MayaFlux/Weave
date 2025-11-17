using System;
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

public class MayaFluxDownloadStep : IInstallationStep
{
    private Logger logger = new();
    private bool downloadSuccess = false;
    private TextBox? logBox;
    private ProgressBar? progressBar;
    private Button? nextButton;

    public void BuildUI(
        LayoutManager layout,
        InstallationConfig config,
        Action<string> logCallback,
        Action nextCallback)
    {
        layout.AddTitle("Step 2: Download MayaFlux");

        var statusLabel = layout.AddStatusLabel("Preparing download...");

        progressBar = layout.AddProgressBar();

        logBox = layout.AddLogBox(LayoutConstants.LogBoxMaxHeight);

        layout.AddFlexibleSpacer();

        (nextButton, var cancelButton) = layout.AddButtonPair("Next >", "Cancel");
        nextButton.Enabled = false;
        nextButton.Click += (s, e) => nextCallback();
        cancelButton.Click += (s, e) => Application.Exit();

        Task.Run(() => DownloadMayaFluxAsync(config, statusLabel, logCallback));
    }

    private async Task DownloadMayaFluxAsync(InstallationConfig config, Label statusLabel, Action<string> logCallback)
    {
        try
        {
            await LogAsync("=== MayaFlux Download ===");
            await LogAsync("Fetching latest release from GitHub API...");

            var api = new GithubApi(logger);
            var release = await api.GetLatestReleaseAsync();

            if (release == null)
            {
                await LogAsync("[ERROR] Failed to fetch latest release");
                UpdateStatus(statusLabel, "Download failed", ThemeColors.Error);
                return;
            }

            await LogAsync($"[OK] Latest release: {release.TagName}");

            var asset = release.GetWindowsAsset();
            if (asset == null)
            {
                await LogAsync("[ERROR] No Windows asset found in release");
                await LogAsync($"Available assets: {string.Join(", ", release.Assets.Select(a => a.Name))}");
                UpdateStatus(statusLabel, "Download failed", ThemeColors.Error);
                return;
            }

            await LogAsync($"[OK] Asset found: {asset.Name}");
            await LogAsync("");
            await LogAsync("Downloading MayaFlux...");
            UpdateStatus(statusLabel, "Downloading...", ThemeColors.TextSecondary);

            Directory.CreateDirectory(config.TempDirectory);
            var downloadPath = Path.Combine(config.TempDirectory, asset.Name);

            var success = await FileOperations.DownloadFileAsync(
                asset.DownloadUrl,
                downloadPath,
                logger,
                (downloaded, total) =>
                {
                    var percent = (int)((downloaded * 100) / total);
                    UpdateProgress(percent);
                    UpdateStatus(statusLabel, $"Downloading... {percent}%", ThemeColors.TextSecondary);
                }
            );

            if (!success)
            {
                await LogAsync("[ERROR] Download failed");
                UpdateStatus(statusLabel, "Download failed", ThemeColors.Error);
                return;
            }

            var fileSize = new FileInfo(downloadPath).Length;
            await LogAsync($"[OK] Downloaded: {FormatBytes(fileSize)}");
            await LogAsync("");
            await LogAsync("Extracting MayaFlux...");
            UpdateStatus(statusLabel, "Extracting...", ThemeColors.TextSecondary);

            Directory.CreateDirectory(config.MayaFluxRoot);

            if (!FileOperations.Extract7z(downloadPath, config.MayaFluxRoot, logger))
            {
                await LogAsync("[ERROR] Extraction failed");
                UpdateStatus(statusLabel, "Extraction failed", ThemeColors.Error);
                return;
            }

            await LogAsync("[OK] Extraction complete");
            await LogAsync("");
            await LogAsync("Verifying installation...");

            if (!FileOperations.VerifyInstallation(config.MayaFluxRoot, logger))
            {
                await LogAsync("[ERROR] Installation verification failed");
                UpdateStatus(statusLabel, "Verification failed", ThemeColors.Error);
                return;
            }

            await LogAsync("[OK] Installation verified");
            await LogAsync("");
            await LogAsync("=== Download Complete ===");
            downloadSuccess = true;
            UpdateStatus(statusLabel, "Download successful", ThemeColors.Success);

            if (nextButton != null)
            {
                if (nextButton.Parent != null)
                {
                    nextButton.Invoke(new Action(() =>
                    {
                        nextButton.Enabled = true;
                    }));
                }
            }
        }
        catch (Exception ex)
        {
            await LogAsync($"[ERROR] {ex.Message}");
            UpdateStatus(statusLabel, "Download failed", ThemeColors.Error);
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

    private void UpdateProgress(int percent)
    {
        if (progressBar?.Parent != null)
        {
            progressBar.Invoke(new Action(() =>
            {
                progressBar.Value = Math.Min(percent, 100);
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

    private string FormatBytes(long bytes)
    {
        string[] sizes = { "B", "KB", "MB", "GB" };
        double len = bytes;
        int order = 0;
        while (len >= 1024 && order < sizes.Length - 1)
        {
            order++;
            len = len / 1024;
        }
        return $"{len:0.##} {sizes[order]}";
    }
}