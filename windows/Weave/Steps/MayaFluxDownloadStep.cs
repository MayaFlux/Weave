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
    private Logger? logger;
    private readonly MayaFluxDownloadHost host;
    private ReleaseType releaseType;
    private TextBox? logBox;
    private Action<string>? logCallback;
    private ProgressBar? progressBar;
    private Button? nextButton;
    private Button? retryButton;
    private int running;
    private bool downloadReady;

    private const int DevStaleHours = 24;

    public MayaFluxDownloadStep(ReleaseType releaseType) : this(releaseType, new MayaFluxDownloadHost())
    {
        logger = new();
    }

    internal MayaFluxDownloadStep(ReleaseType releaseType, MayaFluxDownloadHost host)
    {
        this.releaseType = releaseType;
        this.host = host;
        host.Report = message => LogAsync(message);
    }

    internal event Action<string>? MessageLogged;
    internal event Action<bool>? ProceedChanged;
    internal event Action<int>? ProgressChanged;

    public void BuildUI(
        LayoutManager layout,
        InstallationConfig config,
        Action<string> logCallback,
        Action nextCallback)
    {
        this.logCallback = logCallback;
        layout.AddTitle("Step 2: Download MayaFlux");

        var statusLabel = layout.AddStatusLabel("Preparing download...");

        progressBar = layout.AddProgressBar();

        logBox = layout.AddLogBox(LayoutConstants.LogBoxMaxHeight);

        layout.AddFlexibleSpacer();

        (nextButton, var cancelButton) = layout.AddButtonPair("Next >", "Cancel");
        nextButton.Enabled = false;
        nextButton.Click += (s, e) => nextCallback();
        cancelButton.Click += (s, e) => Application.Exit();

        retryButton = layout.AddButton("Retry", ThemeColors.ButtonSecondary);
        retryButton.Enabled = false;
        retryButton.Click += (s, e) => Task.Run(() => DownloadMayaFluxAsync(config, statusLabel, logCallback));
        InstallationStepUi.RunWhenReady(logBox, () => DownloadMayaFluxAsync(config, statusLabel, logCallback));
    }

    internal async Task<bool> DownloadMayaFluxAsync(InstallationConfig config, Label? statusLabel = null, Action<string>? logCallback = null)
    {
        if (System.Threading.Interlocked.CompareExchange(ref running, 1, 0) != 0) return false;
        downloadReady = false;
        UpdateButtons();
        string? archivePath = null;
        string? stagingRoot = null;
        bool ownsArchive = false;
        bool ownsStaging = false;
        SetReadiness(false);
        UpdateProgress(0);
        try
        {
            MayaFluxReleaseValidation.ValidateDirectory(config.MayaFluxRoot, host);
            MayaFluxReleaseValidation.ValidateDirectory(config.TempDirectory, host);
            var installRoot = Path.TrimEndingDirectorySeparator(Path.GetFullPath(config.MayaFluxRoot));
            var tempRoot = Path.TrimEndingDirectorySeparator(Path.GetFullPath(config.TempDirectory));
            if (MayaFluxReleaseValidation.ContainsPath(installRoot, tempRoot) || MayaFluxReleaseValidation.ContainsPath(tempRoot, installRoot))
                throw new IOException("Installation and temporary directories must not overlap");

            await LogAsync($"=== MayaFlux Download ({(releaseType == ReleaseType.Stable ? "Stable" : "Development")}) ===");
            host.CreateDirectory(tempRoot);
            using var client = host.CreateHttpClient();
            client.Timeout = TimeSpan.FromMinutes(2);
            client.DefaultRequestHeaders.Add("User-Agent", "Weave-Installer");
            string apiUrl = releaseType == ReleaseType.Stable
                ? "https://api.github.com/repos/MayaFlux/MayaFlux/releases/latest"
                : "https://api.github.com/repos/MayaFlux/MayaFlux/releases";
            await LogAsync("Fetching release info from GitHub...");
            using var response = await client.GetAsync(apiUrl);
            response.EnsureSuccessStatusCode();
            using var doc = JsonDocument.Parse(await response.Content.ReadAsStringAsync());
            JsonElement release;
            if (releaseType == ReleaseType.Stable)
            {
                release = doc.RootElement;
            }
            else
            {
                if (doc.RootElement.ValueKind != JsonValueKind.Array || doc.RootElement.GetArrayLength() == 0)
                    throw new InvalidDataException("No releases found");
                JsonElement? selected = null;
                foreach (var candidate in doc.RootElement.EnumerateArray())
                {
                    if (candidate.TryGetProperty("draft", out var draft) && draft.GetBoolean()) continue;
                    if (candidate.GetProperty("tag_name").GetString()?.EndsWith("-dev", StringComparison.Ordinal) == true)
                    {
                        selected = candidate;
                        break;
                    }
                }
                release = selected ?? throw new InvalidDataException("No development release found (no tags ending in '-dev')");
            }
            if (release.TryGetProperty("draft", out var isDraft) && isDraft.GetBoolean())
                throw new InvalidDataException("A draft release cannot be installed");
            if (releaseType == ReleaseType.Stable && release.TryGetProperty("prerelease", out var isPrerelease) && isPrerelease.GetBoolean())
                throw new InvalidDataException("A prerelease cannot be installed through the stable channel");
            var tag = release.GetProperty("tag_name").GetString() ?? throw new InvalidDataException("Release has no tag_name");
            var version = MayaFluxReleaseValidation.ParseVersion(tag, out var development);
            if (development != (releaseType == ReleaseType.Development))
                throw new InvalidDataException("Release tag does not match the requested channel");
            await LogAsync($"Remote tag: {tag} (version {version})");
            if (await ShouldSkipDownload(installRoot, version.ToString()))
            {
                UpdateStatus(statusLabel, "Already up to date", ThemeColors.Success);
                downloadReady = true;
                return true;
            }

            var assets = release.GetProperty("assets");
            JsonElement? selectedAsset = null;
            var names = new List<string>();
            foreach (var asset in assets.EnumerateArray())
            {
                var name = asset.GetProperty("name").GetString() ?? "";
                names.Add(name);
                if (!name.EndsWith("windows-x64.7z", StringComparison.Ordinal) || name.Contains("-dev-", StringComparison.Ordinal) != development) continue;
                if (selectedAsset != null) throw new InvalidDataException("Multiple matching Windows assets found");
                selectedAsset = asset;
            }
            if (selectedAsset == null)
                throw new InvalidDataException($"No matching Windows asset found for the requested channel. Available: {string.Join(", ", names)}");
            var assetName = selectedAsset.Value.GetProperty("name").GetString()!;
            var url = selectedAsset.Value.GetProperty("browser_download_url").GetString();
            if (!Uri.TryCreate(url, UriKind.Absolute, out var assetUri) || assetUri.Scheme != Uri.UriSchemeHttps ||
                !assetUri.Host.Equals("github.com", StringComparison.OrdinalIgnoreCase) || !assetUri.IsDefaultPort ||
                assetUri.UserInfo.Length != 0 || assetUri.Fragment.Length != 0 || assetUri.Query.Length != 0 ||
                assetUri.AbsolutePath != $"/MayaFlux/MayaFlux/releases/download/{Uri.EscapeDataString(tag)}/{Uri.EscapeDataString(assetName)}")
                throw new InvalidDataException("Asset URL must be the HTTPS download for this MayaFlux release and asset");
            var expectedSize = selectedAsset.Value.TryGetProperty("size", out var size) ? size.GetInt64() : -1;
            if (expectedSize == 0 || expectedSize < -1) throw new InvalidDataException("Release archive size is invalid");
            archivePath = Path.Combine(tempRoot, $"mayaflux-{Guid.NewGuid():N}.7z");
            await LogAsync($"Found: {assetName}");
            UpdateStatus(statusLabel, "Downloading...", ThemeColors.TextSecondary);
            using var downloadTimeout = new System.Threading.CancellationTokenSource(TimeSpan.FromMinutes(30));
            var cancellation = downloadTimeout.Token;
            using (var downloadResponse = await client.GetAsync(assetUri, HttpCompletionOption.ResponseHeadersRead, cancellation))
            {
                downloadResponse.EnsureSuccessStatusCode();
                var totalBytes = downloadResponse.Content.Headers.ContentLength;
                if (totalBytes is <= 0) throw new InvalidDataException("Archive Content-Length must be positive when provided");
                if (totalBytes.HasValue && expectedSize > 0 && totalBytes.Value != expectedSize)
                    throw new InvalidDataException("Archive Content-Length does not match the release asset size");
                using var contentStream = await downloadResponse.Content.ReadAsStreamAsync(cancellation);
                using var fileStream = host.CreateArchive(archivePath);
                ownsArchive = true;
                var signature = new byte[6];
                var buffer = new byte[8192];
                long totalRead = 0;
                int read;
                while ((read = await contentStream.ReadAsync(buffer, 0, buffer.Length, cancellation)) != 0)
                {
                    if (totalRead < signature.Length)
                        Array.Copy(buffer, 0, signature, (int)totalRead, Math.Min(read, signature.Length - (int)totalRead));
                    totalRead += read;
                    if ((totalBytes.HasValue && totalRead > totalBytes.Value) || (expectedSize > 0 && totalRead > expectedSize))
                        throw new InvalidDataException("Archive exceeds its declared size");
                    await fileStream.WriteAsync(buffer, 0, read, cancellation);
                    if (totalBytes.HasValue) UpdateProgress((int)((double)totalRead / totalBytes.Value * 100));
                }
                if (totalRead < 32 || !signature.SequenceEqual(new byte[] { 0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C }) ||
                    (totalBytes.HasValue && totalRead != totalBytes.Value) || (expectedSize > 0 && totalRead != expectedSize))
                    throw new InvalidDataException("Downloaded archive is invalid or incomplete");
            }
            await LogAsync($"Downloaded: {archivePath}");
            stagingRoot = installRoot + $".weave-staging-{Guid.NewGuid():N}";
            if (host.DirectoryExists(stagingRoot)) throw new IOException("Installation staging directory already exists");
            host.CreateDirectory(stagingRoot);
            ownsStaging = true;
            UpdateStatus(statusLabel, "Extracting...", ThemeColors.TextSecondary);
            if (!host.Extract7z(archivePath, stagingRoot, logger)) throw new IOException("Extraction failed");
            await LogAsync("Verifying staged installation...");
            if (!host.VerifyInstallation(stagingRoot, logger)) throw new IOException("Installation verification failed");

            string? backupRoot = null;
            if (host.DirectoryExists(installRoot))
            {
                backupRoot = installRoot + $".weave-backup-{Guid.NewGuid():N}";
                await LogAsync($"Preserving existing installation at {backupRoot}");
                host.MoveDirectory(installRoot, backupRoot);
            }
            try
            {
                host.MoveDirectory(stagingRoot, installRoot);
                stagingRoot = null;
            }
            catch
            {
                if (backupRoot != null)
                {
                    try { host.MoveDirectory(backupRoot, installRoot); }
                    catch (Exception rollbackError)
                    {
                        await LogAsync($"[ERROR] Could not restore the previous installation: {rollbackError.Message}. It is preserved at {backupRoot}");
                    }
                }
                throw;
            }
            if (backupRoot != null)
            {
                try { host.DeleteDirectory(backupRoot); }
                catch (Exception cleanupError) { await LogAsync($"[WARN] Previous installation remains at {backupRoot}: {cleanupError.Message}"); }
            }
            await LogAsync("=== Download Complete ===");
            UpdateStatus(statusLabel, "Download successful", ThemeColors.Success);
            downloadReady = true;
            return true;
        }
        catch (Exception ex)
        {
            await LogAsync($"[ERROR] {ex.Message}. Correct the reported problem and select Retry.");
            UpdateStatus(statusLabel, "Download failed", ThemeColors.Error);
            return false;
        }
        finally
        {
            if (ownsArchive && archivePath != null)
            {
                try { host.DeleteFile(archivePath); }
                catch (Exception ex) { await LogAsync($"[WARN] Could not clean up temporary archive {archivePath}: {ex.Message}"); }
            }
            if (ownsStaging && stagingRoot != null)
            {
                try { host.DeleteDirectory(stagingRoot); }
                catch (Exception ex) { await LogAsync($"[WARN] Could not clean up staging directory {stagingRoot}: {ex.Message}"); }
            }
            System.Threading.Interlocked.Exchange(ref running, 0);
            if (downloadReady) SetReadiness(true);
            UpdateButtons();
        }
    }
    private async Task<bool> ShouldSkipDownload(string mayafluxRoot, string remoteVersion)
    {
        var versionFile = Path.Combine(mayafluxRoot, "share", "MayaFlux", ".version");

        if (!host.FileExists(versionFile))
        {
            await LogAsync("No existing installation found.");
            return false;
        }

        try
        {
            var raw = await host.ReadAllTextAsync(versionFile);
            using var vdoc = JsonDocument.Parse(raw);
            var root = vdoc.RootElement;

            var localVersion = root.GetProperty("version").GetString() ?? "";
            await LogAsync($"Local version: {localVersion}");

            var localNumber = MayaFluxReleaseValidation.ParseVersion(localVersion, out var localDevelopment);
            var remoteNumber = MayaFluxReleaseValidation.ParseVersion(remoteVersion, out _);
            if (releaseType == ReleaseType.Stable && localDevelopment)
                return false;

            if (remoteNumber > localNumber)
            {
                await LogAsync($"Remote version {remoteVersion} is newer than local {localVersion}. Updating...");
                return false;
            }

            if (releaseType == ReleaseType.Stable)
            {
                await LogAsync("Version matches remote. Nothing to do.");
                return host.VerifyInstallation(mayafluxRoot, logger);
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

            var age = host.UtcNow - buildTime;
            await LogAsync($"Build age: {age.TotalHours:F1}h (threshold: {DevStaleHours}h)");

            if (age.TotalHours < 0)
            {
                await LogAsync("[WARN] Local build_time is in the future. Redownloading.");
                return false;
            }

            if (age.TotalHours < DevStaleHours)
            {
                await LogAsync("Dev build is fresh. Nothing to do.");
                return host.VerifyInstallation(mayafluxRoot, logger);
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

    private void SetReadiness(bool ready)
    {
        ProceedChanged?.Invoke(ready);
        if (nextButton != null && nextButton.Parent != null)
            nextButton.Invoke(new Action(() => nextButton.Enabled = ready));
    }

    private void UpdateButtons()
    {
        if (nextButton?.IsDisposed == false && nextButton.IsHandleCreated)
            nextButton.Invoke(new Action(() => nextButton.Enabled = downloadReady && running == 0));
        if (retryButton?.IsDisposed == false && retryButton.IsHandleCreated)
            retryButton.Invoke(new Action(() => retryButton.Enabled = !downloadReady && running == 0));
    }

    private Task LogAsync(string message)
    {
        MessageLogged?.Invoke(message);
        logCallback?.Invoke(message);
        return InstallationStepUi.AppendLogAsync(logBox, message);
    }

    private void UpdateProgress(int percent)
    {
        ProgressChanged?.Invoke(Math.Min(percent, 100));
        if (progressBar?.Parent != null)
            progressBar.Invoke(new Action(() => progressBar.Value = Math.Min(percent, 100)));
    }

    private void UpdateStatus(Label? statusLabel, string text, Color color) => InstallationStepUi.UpdateStatus(statusLabel, text, color);
}
