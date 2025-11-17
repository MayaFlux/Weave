using System;
using System.Collections.Generic;
using System.Drawing;
using System.IO;
using System.Net.Http;
using System.Text.Json;
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
        // Title
        layout.AddTitle("Step 2: Download MayaFlux");

        // Status label
        var statusLabel = layout.AddStatusLabel("Preparing download...");

        // Progress bar
        progressBar = layout.AddProgressBar();

        // Log box
        logBox = layout.AddLogBox(LayoutConstants.LogBoxMaxHeight);

        // Spacer
        layout.AddFlexibleSpacer();

        // Buttons
        (nextButton, var cancelButton) = layout.AddButtonPair("Next >", "Cancel");
        nextButton.Enabled = false;
        nextButton.Click += (s, e) => nextCallback();
        cancelButton.Click += (s, e) => Application.Exit();

        // Start async download
        Task.Run(() => DownloadMayaFluxAsync(config, statusLabel, logCallback));
    }

    private async Task DownloadMayaFluxAsync(InstallationConfig config, Label statusLabel, Action<string> logCallback)
    {
        try
        {
            await LogAsync("=== MayaFlux Download ===");

            if (FileOperations.VerifyInstallation(config.MayaFluxRoot, logger))
            {
                await LogAsync("[OK] MayaFlux already installed at: " + config.MayaFluxRoot);
                await LogAsync("Skipping download...");
                await LogAsync("");
                await LogAsync("=== Download Complete ===");
                downloadSuccess = true;
                UpdateStatus(statusLabel, "Already installed", ThemeColors.Success);

                if (nextButton != null && nextButton.Parent != null)
                {
                    nextButton.Invoke(new Action(() =>
                    {
                        nextButton.Enabled = true;
                    }));
                }
                return;
            }

            await LogAsync("Fetching latest release from GitHub API...");

            Directory.CreateDirectory(config.TempDirectory);

            // Fetch releases
            using (var client = new HttpClient())
            {
                client.DefaultRequestHeaders.Add("User-Agent", "PowerShell/7.0");

                var response = await client.GetAsync("https://api.github.com/repos/MayaFlux/MayaFlux/releases");
                response.EnsureSuccessStatusCode();

                var json = await response.Content.ReadAsStringAsync();
                using (var doc = JsonDocument.Parse(json))
                {
                    var root = doc.RootElement;

                    if (root.ValueKind != JsonValueKind.Array || root.GetArrayLength() == 0)
                    {
                        throw new Exception("No releases found");
                    }

                    var release = root[0];
                    var tag = release.GetProperty("tag_name").GetString();

                    await LogAsync($"Tag: {tag}");

                    // Find windows-x64.7z asset
                    var assets = release.GetProperty("assets");
                    JsonElement? targetAsset = null;

                    foreach (var asset in assets.EnumerateArray())
                    {
                        var name = asset.GetProperty("name").GetString();
                        if (name != null && name.EndsWith("windows-x64.7z"))
                        {
                            targetAsset = asset;
                            break;
                        }
                    }

                    if (targetAsset == null)
                    {
                        var assetNames = new List<string>();
                        foreach (var asset in assets.EnumerateArray())
                        {
                            assetNames.Add(asset.GetProperty("name").GetString() ?? "");
                        }
                        throw new Exception($"No Windows .7z asset found. Available: {string.Join(", ", assetNames)}");
                    }

                    var assetName = targetAsset.Value.GetProperty("name").GetString();
                    var downloadUrl = targetAsset.Value.GetProperty("browser_download_url").GetString();

                    await LogAsync($"Found: {assetName}");

                    // Write tag and URL to files
                    File.WriteAllText(Path.Combine(config.TempDirectory, "tag.txt"), tag);
                    File.WriteAllText(Path.Combine(config.TempDirectory, "url.txt"), downloadUrl);

                    await LogAsync("Downloading...");
                    var outfile = Path.Combine(config.TempDirectory, "mayaflux.7z");

                    using (var downloadResponse = await client.GetAsync(downloadUrl, HttpCompletionOption.ResponseHeadersRead))
                    {
                        downloadResponse.EnsureSuccessStatusCode();

                        var totalBytes = downloadResponse.Content.Headers.ContentLength ?? -1L;
                        var canReportProgress = totalBytes != -1;

                        using (var contentStream = await downloadResponse.Content.ReadAsStreamAsync())
                        using (var fileStream = new FileStream(outfile, FileMode.Create, FileAccess.Write, FileShare.None, 8192, true))
                        {
                            var totalRead = 0L;
                            var buffer = new byte[8192];
                            int read;

                            while ((read = await contentStream.ReadAsync(buffer, 0, buffer.Length)) != 0)
                            {
                                await fileStream.WriteAsync(buffer, 0, read);
                                totalRead += read;

                                if (canReportProgress)
                                {
                                    var progressPercent = (int)((totalRead * 100) / totalBytes);
                                    UpdateProgress(progressPercent);
                                }
                            }
                        }
                    }

                    await LogAsync($"Downloaded: {outfile}");
                    await LogAsync("Success");
                }
            }

            await LogAsync("");
            await LogAsync("Extracting MayaFlux...");
            UpdateStatus(statusLabel, "Extracting...", ThemeColors.TextSecondary);

            // Create installation directory
            Directory.CreateDirectory(config.MayaFluxRoot);

            // Extract archive
            var mayafluxFile = Path.Combine(config.TempDirectory, "mayaflux.7z");
            if (!FileOperations.Extract7z(mayafluxFile, config.MayaFluxRoot, logger))
            {
                await LogAsync("[ERROR] Extraction failed");
                UpdateStatus(statusLabel, "Extraction failed", ThemeColors.Error);
                return;
            }

            await LogAsync("[OK] Extraction complete");
            await LogAsync("");
            await LogAsync("Verifying installation...");

            // Verify installation
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

            if (nextButton != null && nextButton.Parent != null)
            {
                nextButton.Invoke(new Action(() =>
                {
                    nextButton.Enabled = true;
                }));
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
}