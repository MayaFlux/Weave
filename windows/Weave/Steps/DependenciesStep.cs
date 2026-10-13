using System;
using System.Collections.Generic;
using System.Diagnostics;
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
    private readonly DependenciesHost host;
    private bool installSuccess = false;
    private TextBox? logBox;
    private Action<string>? logCallback;
    private Button? nextButton;
    private Button? skipButton;
    private Button? retryButton;
    private readonly object stateGate = new();
    private bool running;
    private volatile bool skipRequested;
    private volatile IDependencyProcess? pendingInstaller;
    private Action? pendingSkip;

    public DependenciesStep() : this(new DependenciesHost()) { }
    internal DependenciesStep(DependenciesHost host) => this.host = host;
    internal event Action<string>? MessageLogged;
    internal event Action<bool>? ProceedChanged;

    private record WingetPackage(string Name, string Id, string Verify, bool IsPath);
    internal const string FFmpegInstallRoot = @"C:\Program Files\FFmpeg_Shared";

    private static readonly WingetPackage[] Packages =
    [
        new("CMake",      "Kitware.CMake",           "cmake",                                         false),
        new("Git",        "Git.Git",                 "git",                                           false),
        new("Ninja",      "Ninja-build.Ninja",       "ninja",                                         false),
        new("LLVM",       "LLVM.LLVM",               @"C:\Program Files\LLVM\bin\clang.exe",          true),
        new("FFmpeg",     "Gyan.FFmpeg.Shared",       "ffmpeg",                                         false),
        new("Vulkan SDK", "KhronosGroup.VulkanSDK",  @"C:\VulkanSDK",                                 true),
    ];

    public void BuildUI(
        LayoutManager layout,
        InstallationConfig config,
        Action<string> logCallback,
        Action nextCallback)
    {
        this.logCallback = logCallback;
        layout.AddTitle("Step 3: Install Dependencies");

        var statusLabel = layout.AddStatusLabel("Installing dependencies...");

        logBox = layout.AddLogBox(LayoutConstants.LogBoxMaxHeight);

        layout.AddFlexibleSpacer();

        nextButton = layout.AddButton("Next >", ThemeColors.ButtonPrimary);
        nextButton.Enabled = false;
        nextButton.Click += (s, e) => nextCallback();

        retryButton = layout.AddButton("Retry", ThemeColors.ButtonSecondary);
        retryButton.Enabled = false;
        retryButton.Click += (s, e) =>
        {
            retryButton.Enabled = false;
            Task.Run(() => InstallDependenciesAsync(config, statusLabel));
        };

        skipButton = layout.AddButton("Skip", ThemeColors.ButtonSecondary);
        skipButton.Click += (s, e) =>
        {
            skipButton.Enabled = false;
            statusLabel.Text = "Waiting for the current operation before skipping...";
            SkipDependencies(config, () =>
            {
                if (skipButton?.IsDisposed == false) skipButton.Invoke(nextCallback);
            });
        };

        InstallationStepUi.RunWhenReady(logBox, () => InstallDependenciesAsync(config, statusLabel));
    }

    internal void SkipDependencies(InstallationConfig config, Action nextCallback)
    {
        lock (stateGate)
        {
            config.SkipDependencies = true;
            skipRequested = true;
            if (running || pendingInstaller != null)
            {
                pendingSkip ??= nextCallback;
                return;
            }
        }
        nextCallback();
    }

    internal async Task<bool> InstallDependenciesAsync(InstallationConfig config, Label? statusLabel = null)
    {
        lock (stateGate)
        {
            if (running || pendingInstaller != null) return false;
            running = true;
            skipRequested = config.SkipDependencies;
            installSuccess = false;
        }
        EnableButtons();
        try
        {
            if (skipRequested)
            {
                await LogAsync("Dependency installation explicitly skipped");
                installSuccess = true;
                return true;
            }
            config.DependencyExecutablePaths.Clear();
            await LogAsync("=== Dependency Installation ===");
            var source = await DetectWingetSourceAsync();
            if (source != null) await LogAsync($"Using winget source: {source}");
            int failed = 0;
            foreach (var pkg in Packages)
            {
                if (skipRequested || pendingInstaller != null) break;
                bool previouslyUsable = false;
                try
                {
                    await LogAsync($"[{pkg.Name}] Checking...");
                    previouslyUsable = await IsInstalledAsync(pkg, config);
                    if (previouslyUsable && pkg.Id != "Gyan.FFmpeg.Shared")
                    {
                        await LogAsync($"[{pkg.Name}] Already installed");
                        continue;
                    }
                    if (skipRequested) break;
                    await LogAsync($"[{pkg.Name}] Installing...");
                    int code = await RunWingetAsync(pkg.Id, source);
                    if (code != 0 && code != -1978335189)
                        throw new InvalidOperationException($"install returned {code}");
                    if (pkg.Id == "Gyan.FFmpeg.Shared") await NormalizeFFmpegAsync();
                    if (!await IsInstalledAsync(pkg, config)) throw new InvalidOperationException("installation verification failed");
                    await LogAsync($"[{pkg.Name}] OK");
                }
                catch (Exception ex)
                {
                    bool stillUsable = false;
                    if (previouslyUsable)
                    {
                        try { stillUsable = await IsInstalledAsync(pkg, config); }
                        catch { }
                    }
                    if (stillUsable)
                        await LogAsync($"[{pkg.Name}] WARN: {ex.Message}. The existing installation remains usable.");
                    else
                    {
                        await LogAsync($"[{pkg.Name}] ERROR: {ex.Message}");
                        failed++;
                    }
                }
            }
            installSuccess = (skipRequested || failed == 0) && pendingInstaller == null;
            if (pendingInstaller != null)
                await LogAsync("WARN: An installer is still running. Retry and navigation must wait until it finishes.");
            else if (skipRequested)
                await LogAsync("Dependency installation skipped; no further packages will be installed");
            else if (failed == 0 && pendingInstaller == null)
                await LogAsync("OK: All dependencies are usable");
            else
                await LogAsync($"ERROR: {failed} package(s) need attention before continuing. Retry after correcting the reported problem, or explicitly Skip.");
            UpdateStatus(statusLabel, installSuccess ? "Dependencies ready" : "Dependencies need attention", installSuccess ? ThemeColors.Success : ThemeColors.Error);
        }
        catch (Exception ex)
        {
            await LogAsync($"ERROR: {ex.Message}");
            UpdateStatus(statusLabel, "Dependency installation failed", ThemeColors.Error);
        }
        finally
        {
            lock (stateGate) running = false;
            EnableButtons();
            FinishDeferredSkip();
        }
        return installSuccess;
    }

    private async Task<string?> DetectWingetSourceAsync()
    {
        try
        {
            var result = await RunProcessAsync(new ProcessStartInfo
            {
                FileName = "winget", Arguments = "source list", UseShellExecute = false,
                RedirectStandardOutput = true, RedirectStandardError = true, CreateNoWindow = true
            }, false);
            if (result.Code != 0) return null;
            foreach (var line in result.Output.Split(new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries))
            {
                var columns = line.Split(new[] { ' ', '\t' }, StringSplitOptions.RemoveEmptyEntries);
                if (columns.Length >= 2 && (columns[0].Equals("winget", StringComparison.OrdinalIgnoreCase) ||
                    columns[0].Equals("Microsoft.Winget.Source", StringComparison.OrdinalIgnoreCase)) &&
                    Uri.TryCreate(columns[1], UriKind.Absolute, out var uri) && uri.Scheme == Uri.UriSchemeHttps)
                    return columns[0];
            }
        }
        catch (Exception ex) { await LogAsync($"WARN: WinGet source discovery failed: {ex.Message}. Using the configured default sources."); }
        return null;
    }

    private IEnumerable<string> PathDirectories()
    {
        foreach (var target in new[] { EnvironmentVariableTarget.Machine, EnvironmentVariableTarget.User, EnvironmentVariableTarget.Process })
        {
            foreach (var entry in (host.GetEnvironmentVariable("PATH", target) ?? "").Split(';'))
            {
                var directory = entry.Trim();
                if (directory.StartsWith('"') && directory.EndsWith('"') && directory.Length >= 2) directory = directory[1..^1];
                directory = System.Text.RegularExpressions.Regex.Replace(directory, "%([^%]+)%", match =>
                    host.GetEnvironmentVariable(match.Groups[1].Value, EnvironmentVariableTarget.Process) ??
                    host.GetEnvironmentVariable(match.Groups[1].Value, EnvironmentVariableTarget.User) ??
                    host.GetEnvironmentVariable(match.Groups[1].Value, EnvironmentVariableTarget.Machine) ?? match.Value);
                if (Path.IsPathFullyQualified(directory)) yield return directory;
            }
        }
    }

    private IEnumerable<string> ConfiguredRoots(string name)
    {
        foreach (var target in new[] { EnvironmentVariableTarget.Process, EnvironmentVariableTarget.User, EnvironmentVariableTarget.Machine })
        {
            var root = host.GetEnvironmentVariable(name, target)?.Trim().Trim('"');
            if (!string.IsNullOrEmpty(root) && Path.IsPathFullyQualified(root)) yield return root;
        }
    }

    private async Task<bool> IsInstalledAsync(WingetPackage pkg, InstallationConfig config)
    {
        var paths = new List<string>();
        if (pkg.Id == "Gyan.FFmpeg.Shared")
        {
            paths.AddRange(FFmpegRoots(host).Where(root => IsSharedFFmpegRoot(host, root)).Select(root => Path.Combine(root, "bin", "ffmpeg.exe")));
        }
        else if (pkg.Id == "KhronosGroup.VulkanSDK")
        {
            paths.AddRange(ConfiguredRoots("VULKAN_SDK").Concat(ConfiguredRoots("VK_SDK_PATH")).Select(root => Path.Combine(root, "Bin", "glslc.exe")));
            if (host.DirectoryExists(@"C:\VulkanSDK"))
                paths.AddRange(host.GetDirectories(@"C:\VulkanSDK")
                    .OrderByDescending(root => Version.TryParse(Path.GetFileName(root), out var version) ? version : new Version(0, 0))
                    .Select(root => Path.Combine(root, "Bin", "glslc.exe")));
            paths.AddRange(PathDirectories().Select(directory => Path.Combine(directory, "glslc.exe")));
        }
        else
        {
            if (pkg.Id == "LLVM.LLVM")
            {
                paths.AddRange(ConfiguredRoots("LLVM_ROOT").Select(root => Path.Combine(root, "bin", "clang.exe")));
                paths.Add(pkg.Verify);
            }
            var executable = pkg.Id == "LLVM.LLVM" ? "clang.exe" : pkg.Verify + ".exe";
            paths.AddRange(PathDirectories().Select(directory => Path.Combine(directory, executable)));
        }
        foreach (var path in paths.Distinct(StringComparer.OrdinalIgnoreCase))
        {
            if (!host.FileExists(path)) continue;
            try
            {
                var result = await RunProcessAsync(new ProcessStartInfo
                {
                    FileName = path, Arguments = pkg.Id == "Gyan.FFmpeg.Shared" ? "-version" : "--version",
                    UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true, CreateNoWindow = true
                }, false);
                var pattern = pkg.Id switch
                {
                    "Kitware.CMake" => @"\bcmake version (\d+\.\d+)",
                    "Git.Git" => @"\bgit version \d+\.\d+",
                    "Ninja-build.Ninja" => @"^\d+\.\d+(?:\.\d+)?[^\s]*\s*$",
                    "LLVM.LLVM" => @"\bclang version \d+\.\d+",
                    "Gyan.FFmpeg.Shared" => @"\bffmpeg version (?=\S*\d)\S+",
                    "KhronosGroup.VulkanSDK" => @"\bshaderc\s+v?\d+",
                    _ => throw new InvalidOperationException("Unknown dependency")
                };
                var banner = System.Text.RegularExpressions.Regex.Match(result.Output + "\n" + result.Error, pattern,
                    System.Text.RegularExpressions.RegexOptions.IgnoreCase | System.Text.RegularExpressions.RegexOptions.Multiline | System.Text.RegularExpressions.RegexOptions.CultureInvariant);
                bool supported = banner.Success && (pkg.Id != "Kitware.CMake" ||
                    (Version.TryParse(banner.Groups[1].Value, out var cmakeVersion) && cmakeVersion >= new Version(3, 31)));
                if (result.Code == 0 && supported)
                {
                    config.DependencyExecutablePaths[pkg.Id] = path;
                    return true;
                }
                await LogAsync($"[{pkg.Name}] WARN: {path} did not return a usable version (exit code {result.Code})" +
                    (pkg.Id == "Kitware.CMake" ? ". Generated projects require CMake 3.31 or newer." : ""));
            }
            catch (Exception ex) { await LogAsync($"[{pkg.Name}] WARN: Cannot run {path}: {ex.Message}"); }
        }
        return false;
    }

    private static bool IsSharedFFmpegRoot(WindowsInstallationHost host, string root)
    {
        try
        {
            if (!host.FileExists(Path.Combine(root, "bin", "ffmpeg.exe"))) return false;
            var entries = host.GetFileSystemEntries(Path.Combine(root, "bin"));
            return new[] { "avcodec", "avformat", "avutil", "swresample", "swscale", "avdevice" }.All(component =>
                host.FileExists(Path.Combine(root, "lib", component + ".lib")) &&
                host.FileExists(Path.Combine(root, "include", "lib" + component, component + ".h")) &&
                entries.Any(path => Path.GetFileName(path).StartsWith(component + "-", StringComparison.OrdinalIgnoreCase) &&
                    path.EndsWith(".dll", StringComparison.OrdinalIgnoreCase) && host.FileExists(path)));
        }
        catch (IOException) { return false; }
        catch (UnauthorizedAccessException) { return false; }
    }

    internal static string? FindFFmpegRoot() => FindFFmpegRoot(new DependenciesHost());
    internal static string? FindFFmpegRoot(WindowsInstallationHost host)
    {
        return FFmpegRoots(host).FirstOrDefault(root => IsSharedFFmpegRoot(host, root));
    }

    private static IEnumerable<string> FFmpegRoots(WindowsInstallationHost host)
    {
        if (host.DirectoryExists(FFmpegInstallRoot))
        {
            foreach (var root in WindowsInstallationPaths.OrderVersionDirectories(host.GetDirectories(FFmpegInstallRoot)))
                yield return root;
        }
        foreach (var target in new[] { EnvironmentVariableTarget.Process, EnvironmentVariableTarget.User, EnvironmentVariableTarget.Machine })
        {
            var root = host.GetEnvironmentVariable("FFMPEG_ROOT", target)?.Trim().Trim('"');
            if (!string.IsNullOrEmpty(root) && Path.IsPathFullyQualified(root)) yield return root;
        }
    }

    private async Task NormalizeFFmpegAsync()
    {
        if (!host.DirectoryExists(FFmpegInstallRoot)) return;
        foreach (var directory in host.GetDirectories(FFmpegInstallRoot, "ffmpeg-*-full_build-shared"))
        {
            var version = Path.GetFileName(directory).Replace("ffmpeg-", "").Replace("-full_build-shared", "");
            if (!Version.TryParse(version, out _)) continue;
            var destination = Path.Combine(FFmpegInstallRoot, version);
            if (host.DirectoryExists(destination)) continue;
            if (!IsSharedFFmpegRoot(host, directory)) throw new IOException($"FFmpeg wrapper is incomplete: {directory}");
            host.MoveDirectory(directory, destination);
            await LogAsync($"[FFmpeg] Using version {version}");
        }
    }

    private async Task<int> RunWingetAsync(string id, string? source)
    {
        var psi = new ProcessStartInfo
        {
            FileName = "winget", UseShellExecute = false, RedirectStandardOutput = true,
            RedirectStandardError = true, CreateNoWindow = true
        };
        foreach (var argument in new[] { "install", "--id", id, "--exact", "--accept-package-agreements", "--accept-source-agreements", "--silent" })
            psi.ArgumentList.Add(argument);
        if (source != null) { psi.ArgumentList.Add("--source"); psi.ArgumentList.Add(source); }
        if (id == "Gyan.FFmpeg.Shared")
        {
            psi.ArgumentList.Add("--location");
            psi.ArgumentList.Add(FFmpegInstallRoot);
            if (FindFFmpegRoot(host) == null) psi.ArgumentList.Add("--force");
        }
        var result = await RunProcessAsync(psi, true);
        if (!string.IsNullOrWhiteSpace(result.Output)) await LogAsync(result.Output);
        if (!string.IsNullOrWhiteSpace(result.Error)) await LogAsync(result.Error);
        return result.Code;
    }

    private async Task<(int Code, string Output, string Error)> RunProcessAsync(ProcessStartInfo startInfo, bool installer)
    {
        IDependencyProcess? process = host.StartProcess(startInfo);
        if (process == null) throw new InvalidOperationException($"Failed to start {startInfo.FileName}");
        using var timeout = new CancellationTokenSource(installer ? host.InstallTimeout : host.ProcessTimeout);
        try
        {
            var output = process.ReadStandardOutputAsync();
            var error = process.ReadStandardErrorAsync();
            await process.WaitForExitAsync(timeout.Token);
            await Task.WhenAll(output, error).WaitAsync(timeout.Token);
            return (process.ExitCode, await output, await error);
        }
        catch (OperationCanceledException)
        {
            if (installer && !process.HasExited)
            {
                lock (stateGate) pendingInstaller = process;
                _ = ObserveInstallerExitAsync(process);
                process = null;
            }
            else if (!installer && !process.HasExited) process.Terminate();
            throw new TimeoutException($"{startInfo.FileName} timed out. Let any active installer finish before retrying.");
        }
        finally { process?.Dispose(); }
    }

    private async Task ObserveInstallerExitAsync(IDependencyProcess process)
    {
        try { await process.WaitForExitAsync(); }
        catch (Exception ex) { await LogAsync($"WARN: Could not observe installer completion: {ex.Message}"); }
        finally
        {
            if (process.HasExited)
            {
                lock (stateGate) pendingInstaller = null;
                process.Dispose();
                EnableButtons();
                FinishDeferredSkip();
            }
        }
    }

    private void FinishDeferredSkip()
    {
        Action? callback = null;
        lock (stateGate)
        {
            if (!running && pendingInstaller == null) { callback = pendingSkip; pendingSkip = null; }
        }
        callback?.Invoke();
    }
    private Task LogAsync(string message)
    {
        MessageLogged?.Invoke(message);
        logCallback?.Invoke(message);
        return InstallationStepUi.AppendLogAsync(logBox, message);
    }

    private void UpdateStatus(Label? statusLabel, string text, Color color) => InstallationStepUi.UpdateStatus(statusLabel, text, color);

    private void EnableButtons()
    {
        bool ready;
        lock (stateGate) ready = installSuccess && !running && pendingInstaller == null;
        ProceedChanged?.Invoke(ready);
        if (nextButton?.IsDisposed == false && nextButton.IsHandleCreated)
            nextButton.Invoke(new Action(() => nextButton.Enabled = ready));
        if (skipButton?.IsDisposed == false && skipButton.IsHandleCreated)
            skipButton.Invoke(new Action(() => skipButton.Enabled = !skipRequested));
        if (retryButton?.IsDisposed == false && retryButton.IsHandleCreated)
            retryButton.Invoke(new Action(() => retryButton.Enabled = !running && pendingInstaller == null && !installSuccess));
    }
}
