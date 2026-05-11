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
    private Logger logger = new();
    private bool installSuccess = false;
    private TextBox? logBox;
    private Button? nextButton;
    private Button? skipButton;

    private record WingetPackage(string Name, string Id, string Verify, bool IsPath);

    private static readonly WingetPackage[] Packages =
    [
        new("CMake",      "Kitware.CMake",           "cmake",                                         false),
        new("Git",        "Git.Git",                 "git",                                           false),
        new("Ninja",      "Ninja-build.Ninja",       "ninja",                                         false),
        new("LLVM",       "LLVM.LLVM",               @"C:\Program Files\LLVM\bin\clang.exe",          true),
        new("FFmpeg",     "Gyan.FFmpeg.Shared",         @"Gyan.FFmpeg.Shared",                           false),
        new("Vulkan SDK", "KhronosGroup.VulkanSDK",  @"C:\VulkanSDK",                                 true),
    ];

    public void BuildUI(
        LayoutManager layout,
        InstallationConfig config,
        Action<string> logCallback,
        Action nextCallback)
    {
        layout.AddTitle("Step 3: Install Dependencies");

        var statusLabel = layout.AddStatusLabel("Installing dependencies...");

        logBox = layout.AddLogBox(LayoutConstants.LogBoxMaxHeight);

        layout.AddFlexibleSpacer();

        nextButton = layout.AddButton("Next >", ThemeColors.ButtonPrimary);
        nextButton.Enabled = false;
        nextButton.Click += (s, e) => nextCallback();

        skipButton = layout.AddButton("Skip", ThemeColors.ButtonSecondary);
        skipButton.Click += (s, e) =>
        {
            installSuccess = true;
            nextCallback();
        };

        Task.Run(() => InstallDependenciesAsync(statusLabel));
    }

    private async Task InstallDependenciesAsync(Label statusLabel)
    {
        try
        {
            await LogAsync("=== Dependency Installation ===");
            await LogAsync("");
            await LogAsync("Installing via winget: CMake, Git, Ninja, 7-Zip, LLVM, FFmpeg, Vulkan SDK");
            await LogAsync("");

            string? wingetSource = DetectWingetSource();
            if (wingetSource != null)
                await LogAsync($"Using winget source: {wingetSource}");

            int failed = 0;
            foreach (var pkg in Packages)
            {
                await LogAsync($"[{pkg.Name}] Checking...");

                if (IsInstalled(pkg))
                {
                    await LogAsync($"[{pkg.Name}] Already installed");
                    continue;
                }

                await LogAsync($"[{pkg.Name}] Installing...");
                int code = await RunWingetAsync(pkg.Id, wingetSource);

                if ((code == 0 || code == -1978335189) && IsInstalled(pkg))
                {
                    await LogAsync($"[{pkg.Name}] OK");
                }
                else
                {
                    await LogAsync($"[{pkg.Name}] WARN: install returned {code} or verification failed");
                    failed++;
                }
            }

            await LogAsync("");
            if (failed == 0)
            {
                await LogAsync("OK: All dependencies installed");
                installSuccess = true;
                UpdateStatus(statusLabel, "Dependencies installed", ThemeColors.Success);
            }
            else
            {
                await LogAsync($"WARN: {failed} package(s) may need manual installation");
                installSuccess = true;
                UpdateStatus(statusLabel, "Dependencies installed with warnings", ThemeColors.Warning);
            }
        }
        catch (Exception ex)
        {
            await LogAsync($"ERROR: {ex.Message}");
            installSuccess = true;
            UpdateStatus(statusLabel, "Dependencies skipped - continuing", ThemeColors.Warning);
        }
        finally
        {
            EnableButtons();
        }
    }

    private static string? DetectWingetSource()
    {
        try
        {
            var psi = new ProcessStartInfo
            {
                FileName = "winget",
                Arguments = "source list",
                UseShellExecute = false,
                RedirectStandardOutput = true,
                CreateNoWindow = true
            };
            using var p = Process.Start(psi);
            if (p == null) return null;
            var output = p.StandardOutput.ReadToEnd();
            p.WaitForExit();

            foreach (var name in new[] { "winget", "Microsoft.Winget.Source" })
                if (output.Contains(name, StringComparison.OrdinalIgnoreCase))
                    return name;
        }
        catch { }
        return null;
    }

    private static bool IsInstalled(WingetPackage pkg)
    {
        if (pkg.IsPath)
        {
            if (!pkg.Verify.Contains('\\'))
                return Directory.Exists(pkg.Verify) && Directory.GetFileSystemEntries(pkg.Verify).Length > 0;
            return File.Exists(pkg.Verify) || Directory.Exists(pkg.Verify);
        }

        if (pkg.Verify.StartsWith("Gyan.FFmpeg"))
            return FindFFmpegRoot() != null;

        var machinePath = Environment.GetEnvironmentVariable("PATH", EnvironmentVariableTarget.Machine) ?? "";
        var userPath = Environment.GetEnvironmentVariable("PATH", EnvironmentVariableTarget.User) ?? "";

        foreach (var dir in machinePath.Split(';').Concat(userPath.Split(';')))
        {
            if (string.IsNullOrWhiteSpace(dir)) continue;
            if (File.Exists(Path.Combine(dir, pkg.Verify + ".exe")) ||
                File.Exists(Path.Combine(dir, pkg.Verify)))
                return true;
        }

        return false;
    }

    internal static string? FindFFmpegRoot()
    {
        var wingetPkgs = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            @"Microsoft\WinGet\Packages");

        if (!Directory.Exists(wingetPkgs)) return null;

        return Directory.GetDirectories(wingetPkgs, "Gyan.FFmpeg.Shared*")
            .SelectMany(d => Directory.GetDirectories(d, "ffmpeg-*-full_build-shared"))
            .FirstOrDefault(d => File.Exists(Path.Combine(d, "bin", "ffmpeg.exe")));
    }

    private static async Task<int> RunWingetAsync(string id, string? source)
    {
        var args = new List<string>
        {
            "install",
            "--id", id,
            "--exact",
            "--accept-package-agreements",
            "--accept-source-agreements",
            "--silent"
        };

        if (source != null)
        {
            args.Add("--source");
            args.Add(source);
        }

        var psi = new ProcessStartInfo
        {
            FileName = "winget",
            Arguments = string.Join(" ", args),
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true
        };

        using var p = Process.Start(psi);
        if (p == null) return 1;
        await p.WaitForExitAsync();
        return p.ExitCode;
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

    private void EnableButtons()
    {
        nextButton?.Invoke(new Action(() => nextButton.Enabled = true));
        skipButton?.Invoke(new Action(() => skipButton.Enabled = true));
    }
}