using System;
using System.Collections.Generic;
using System.Drawing;
using System.Globalization;
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
    private ReleaseType releaseType;
    private bool downloadSuccess = false;
    private TextBox? logBox;
    private ProgressBar? progressBar;
    private Button? nextButton;

    private const int DevStaleHours = 24;

    public MayaFluxDownloadStep(ReleaseType releaseType)
    {
        this.releaseType = releaseType;
    }

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
            await LogAsync($"=== MayaFlux Download ({(releaseType == ReleaseType.Stable ? "Stable" : "Development")}) ===");

            Directory.CreateDirectory(config.TempDirectory);

            using var client = new HttpClient();
            client.DefaultRequestHeaders.Add("User-Agent", "Weave-Installer");

            string apiUrl = releaseType == ReleaseType.Stable
                ? "https://api.github.com/repos/MayaFlux/MayaFlux/releases/latest"
                : "https://api.github.com/repos/MayaFlux/MayaFlux/releases";

            await LogAsync("Fetching release info from GitHub...");
            var response = await client.GetAsync(apiUrl);
            response.EnsureSuccessStatusCode();

            var json = await response.Content.ReadAsStringAsync();
            using var doc = JsonDocument.Parse(json);

            JsonElement release;

            if (releaseType == ReleaseType.Stable)
            {
                release = doc.RootElement;
            }
            else
            {
                var root = doc.RootElement;
                if (root.ValueKind != JsonValueKind.Array || root.GetArrayLength() == 0)
                    throw new Exception("No releases found");

                JsonElement? devRelease = null;
                foreach (var rel in root.EnumerateArray())
                {
                    var tagName = rel.GetProperty("tag_name").GetString();
                    if (tagName != null && tagName.Contains("-dev"))
                    {
                        devRelease = rel;
                        break;
                    }
                }

                if (devRelease == null)
                    throw new Exception("No development release found (no tags containing '-dev')");

                release = devRelease.Value;
            }

            var tag = release.GetProperty("tag_name").GetString() ?? throw new Exception("Release has no tag_name");
            var remoteVersion = tag.TrimStart('v').Replace("-dev", "").Trim();
            await LogAsync($"Remote tag: {tag} (version {remoteVersion})");

            if (await ShouldSkipDownload(config.MayaFluxRoot, remoteVersion))
            {
                await LogAsync("=== Download Complete ===");
                downloadSuccess = true;
                UpdateStatus(statusLabel, "Already up to date", ThemeColors.Success);
                EnableNext();
                return;
            }

            var assets = release.GetProperty("assets");
            JsonElement? targetAsset = null;

            foreach (var asset in assets.EnumerateArray())
            {
                var name = asset.GetProperty("name").GetString();
                if (name == null || !name.EndsWith("windows-x64.7z")) continue;

                if (releaseType == ReleaseType.Development)
                {
                    if (name.Contains("-dev-")) { targetAsset = asset; break; }
                }
                else
                {
                    if (!name.Contains("-dev-")) { targetAsset = asset; break; }
                }
            }

            if (targetAsset == null)
            {
                var assetNames = new List<string>();
                foreach (var asset in assets.EnumerateArray())
                    assetNames.Add(asset.GetProperty("name").GetString() ?? "");

                string expected = releaseType == ReleaseType.Development
                    ? "asset containing '-dev-windows-x64.7z'"
                    : "asset matching 'windows-x64.7z' (without -dev)";

                throw new Exception($"No matching Windows asset found. Expected: {expected}\nAvailable: {string.Join(", ", assetNames)}");
            }

            var assetFileName = targetAsset.Value.GetProperty("name").GetString();
            var downloadUrl = targetAsset.Value.GetProperty("browser_download_url").GetString();

            await LogAsync($"Found: {assetFileName}");

            if (Directory.Exists(config.MayaFluxRoot))
            {
                await LogAsync("Removing existing installation...");
                Directory.Delete(config.MayaFluxRoot, recursive: true);
            }

            await LogAsync("Downloading...");
            UpdateStatus(statusLabel, "Downloading...", ThemeColors.TextSecondary);

            var outfile = Path.Combine(config.TempDirectory, "mayaflux.7z");

            using (var downloadResponse = await client.GetAsync(downloadUrl, HttpCompletionOption.ResponseHeadersRead))
            {
                downloadResponse.EnsureSuccessStatusCode();

                var totalBytes = downloadResponse.Content.Headers.ContentLength ?? -1L;
                var canReportProgress = totalBytes != -1;

                using var contentStream = await downloadResponse.Content.ReadAsStreamAsync();
                using var fileStream = new FileStream(outfile, FileMode.Create, FileAccess.Write, FileShare.None, 8192, true);

                var totalRead = 0L;
                var buffer = new byte[8192];
                int read;

                while ((read = await contentStream.ReadAsync(buffer, 0, buffer.Length)) != 0)
                {
                    await fileStream.WriteAsync(buffer, 0, read);
                    totalRead += read;

                    if (canReportProgress)
                        UpdateProgress((int)((totalRead * 100) / totalBytes));
                }
            }

            await LogAsync($"Downloaded: {outfile}");

            await LogAsync("Extracting MayaFlux...");
            UpdateStatus(statusLabel, "Extracting...", ThemeColors.TextSecondary);

            Directory.CreateDirectory(config.MayaFluxRoot);

            if (!FileOperations.Extract7z(outfile, config.MayaFluxRoot, logger))
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
            EnableNext();
        }
        catch (Exception ex)
        {
            await LogAsync($"[ERROR] {ex.Message}");
            UpdateStatus(statusLabel, "Download failed", ThemeColors.Error);
        }
    }

    private async Task<bool> ShouldSkipDownload(string mayafluxRoot, string remoteVersion)
    {
        var versionFile = Path.Combine(mayafluxRoot, "share", "MayaFlux", ".version");

        if (!File.Exists(versionFile))
        {
            await LogAsync("No existing installation found.");
            return false;
        }

        try
        {
            var raw = await File.ReadAllTextAsync(versionFile);
            using var vdoc = JsonDocument.Parse(raw);
            var root = vdoc.RootElement;

            var localVersion = root.GetProperty("version").GetString() ?? "";
            await LogAsync($"Local version: {localVersion}");

            if (IsNewerVersion(remoteVersion, localVersion))
            {
                await LogAsync($"Remote version {remoteVersion} is newer than local {localVersion}. Updating...");
                return false;
            }

            if (releaseType == ReleaseType.Stable)
            {
                await LogAsync("Version matches remote. Nothing to do.");
                return true;
            }

            var buildTimeStr = root.GetProperty("build_time").GetString() ?? "";
            if (!DateTime.TryParseExact(
                buildTimeStr,
                "yyyy-MM-dd HH:mm:ss UTC",
                CultureInfo.InvariantCulture,
                DateTimeStyles.AssumeUniversal | DateTimeStyles.AdjustToUniversal,
                out var buildTime))
            {
                await LogAsync($"[WARN] Could not parse build_time '{buildTimeStr}'. Redownloading.");
                return false;
            }

            var age = DateTime.UtcNow - buildTime;
            await LogAsync($"Build age: {age.TotalHours:F1}h (threshold: {DevStaleHours}h)");

            if (age.TotalHours < DevStaleHours)
            {
                await LogAsync("Dev build is fresh. Nothing to do.");
                return true;
            }

            await LogAsync($"Dev build is stale ({age.TotalHours:F1}h old). Updating...");
            return false;
        }
        catch (Exception ex)
        {
            await LogAsync($"[WARN] Could not read .version file: {ex.Message}. Redownloading.");
            return false;
        }
    }

    private static bool IsNewerVersion(string remote, string local)
    {
        if (Version.TryParse(remote, out var r) && Version.TryParse(local, out var l))
            return r > l;
        return string.Compare(remote, local, StringComparison.Ordinal) > 0;
    }

    private void EnableNext()
    {
        if (nextButton != null && nextButton.Parent != null)
            nextButton.Invoke(new Action(() => nextButton.Enabled = true));
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
            progressBar.Invoke(new Action(() => progressBar.Value = Math.Min(percent, 100)));
    }

    private void UpdateStatus(Label statusLabel, string text, Color color)
    {
        if (statusLabel.Parent != null)
            statusLabel.Invoke(new Action(() =>
            {
                statusLabel.Text = text;
                statusLabel.ForeColor = color;
            }));
    }
}