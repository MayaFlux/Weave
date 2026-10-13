using System;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using System.Windows.Forms;
using Weave.Modes;
using Weave.Shared.Models;
using Weave.Theme;
using Weave.UI.Layout;
using Weave.Utils;

namespace Weave.UI.Pages;

public class SystemCheckStep : IInstallationStep
{
    private Logger? logger;
    private readonly SystemCheckHost host;
    private bool checksPass = false;
    private TextBox? logBox;
    private Action<string>? logCallback;
    private Label? statusLabel;
    private Button? nextButton;
    private Button? retryButton;
    private int running;
    private volatile ISystemCheckProcess? pendingInstaller;

    public SystemCheckStep() : this(new SystemCheckHost())
    {
        logger = new();
    }

    internal SystemCheckStep(SystemCheckHost host)
    {
        this.host = host;
    }

    internal event Action<string>? MessageLogged;
    internal event Action<bool>? ProceedChanged;

    public void BuildUI(
        LayoutManager layout,
        InstallationConfig config,
        Action<string> logCallback,
        Action nextCallback)
    {
        this.logCallback = logCallback;
        layout.AddTitle("Step 1: System Check");

        statusLabel = layout.AddStatusLabel("Verifying system requirements...");

        logBox = layout.AddLogBox(LayoutConstants.LogBoxMaxHeight);

        layout.AddFlexibleSpacer();

        (nextButton, var cancelButton) = layout.AddButtonPair("Next >", "Cancel");
        nextButton.Enabled = false;
        nextButton.Click += (s, e) => nextCallback();
        cancelButton.Click += (s, e) => Application.Exit();

        retryButton = layout.AddButton("Retry", ThemeColors.ButtonSecondary);
        retryButton.Enabled = false;
        retryButton.Click += (s, e) => Task.Run(() => PerformChecksAsync(config));
        InstallationStepUi.RunWhenReady(logBox, () => PerformChecksAsync(config));
    }

    internal async Task<bool> PerformChecksAsync(InstallationConfig config)
    {
        if (System.Threading.Interlocked.CompareExchange(ref running, 1, 0) != 0) return false;
        try
        {
            if (pendingInstaller != null)
            {
                await LogAsync("[WARN] Microsoft prerequisites are still being installed. Wait for completion before retrying.");
                return false;
            }
            checksPass = false;
            UpdateButtons();
            return await PerformChecksCoreAsync(config);
        }
        catch (Exception error)
        {
            checksPass = false;
            await LogAsync($"[ERROR] {error.Message}. Correct the reported problem and select Retry.");
            UpdateStatus("System checks need attention", ThemeColors.Error);
            return false;
        }
        finally
        {
            System.Threading.Interlocked.Exchange(ref running, 0);
            EnableButton();
        }
    }

    private async Task<bool> PerformChecksCoreAsync(InstallationConfig config)
    {
        checksPass = true;

        await LogAsync("Checking Windows architecture...");
        if (host.PointerSize == 8 && host.ProcessArchitecture == System.Runtime.InteropServices.Architecture.X64)
        {
            await LogAsync("[OK] 64-bit Windows");
        }
        else
        {
            await LogAsync("[ERROR] An x64 Windows process is required");
            checksPass = false;
            UpdateStatus("Unsupported architecture", ThemeColors.Error);
            return false;
        }

        await LogAsync("");
        await LogAsync("Checking for administrator privileges...");
        if (IsAdministrator())
        {
            await LogAsync("[OK] Running as administrator");
        }
        else
        {
            await LogAsync("[ERROR] Not running as administrator. Close Weave, then choose Run as administrator to install system prerequisites.");
            checksPass = false;
            UpdateStatus("Admin privileges required", ThemeColors.Error);
            return false;
        }

        await LogAsync("");
        await LogAsync("Checking for 7-Zip...");

        string? sevenZipPath;
        try
        {
            sevenZipPath = Find7zPath();
        }
        catch (Exception ex)
        {
            await LogAsync($"[ERROR] Cannot check 7-Zip: {ex.Message}");
            checksPass = false;
            UpdateStatus("Cannot check prerequisites", ThemeColors.Error);
            return false;
        }

        if (string.IsNullOrEmpty(sevenZipPath))
        {
            await LogAsync("[INSTALLING] Downloading 7-Zip...");
            bool installed = await Install7ZipAsync();
            if (installed)
            {
                await LogAsync("[OK] 7-Zip installed");
            }
            else
            {
                await LogAsync("[ERROR] Failed to install 7-Zip");
                checksPass = false;
            }
        }
        else
        {
            await LogAsync("[OK] 7-Zip found");
        }

        await LogAsync("");
        await LogAsync("Checking for MSVC build tools...");

        if (!HasMsvcToolchain())
        {
            await LogAsync("[INSTALLING] Visual Studio Build Tools not found");
            await LogAsync("[INSTALLING] Windows requires additional Microsoft C++ libraries and SDK files to build MayaFlux projects and support live C++ compilation, even when LLVM is used. These shared prerequisites account for most of this installation's size. Microsoft's installer will display progress, and Weave will continue when it finishes.");
            await LogAsync("[INSTALLING] Installing Microsoft C++ prerequisites (this may take 10-20 minutes)...");
            int exitCode = await InstallMsvcAsync(config);
            if (exitCode == 3010)
                config.NeedsReboot = true;
            if ((exitCode == 0 || exitCode == 3010) && HasMsvcToolchain())
            {
                await LogAsync("[OK] MSVC build tools installed");
                if (exitCode == 3010)
                {
                    await LogAsync("[WARN] A system restart is required before building");
                }
            }
            else
            {
                await LogAsync($"[ERROR] MSVC installation did not provide a usable x64 toolchain (exit code {exitCode})");
                checksPass = false;
            }
        }
        else
        {
            await LogAsync("[OK] MSVC build tools found");
        }

        await LogAsync("");
        if (checksPass)
        {
            await LogAsync("[OK] All prerequisites met");
            UpdateStatus("Ready to proceed", ThemeColors.Success);
        }
        else
        {
            await LogAsync("[ERROR] Prerequisites need attention. Correct the reported problem, then select Retry.");
            UpdateStatus("Prerequisites missing", ThemeColors.Error);
        }

        return checksPass;
    }

    private async Task<bool> Install7ZipAsync()
    {
        string? installerPath = null;
        bool ownsInstaller = false;
        try
        {
            string tempDir = host.TempDirectory;
            if (!Path.IsPathFullyQualified(tempDir) || tempDir.StartsWith(@"\\") ||
                Path.TrimEndingDirectorySeparator(Path.GetFullPath(tempDir)) == Path.GetPathRoot(tempDir))
                throw new IOException("Installer temporary directory must be a local absolute directory below a drive root");
            installerPath = Path.Combine(Path.GetFullPath(tempDir), "7z-install.exe");
            if (host.FileExists(installerPath))
                throw new IOException("Temporary installer already exists; it will not be overwritten");

            using (var response = await host.DownloadAsync("https://www.7-zip.org/a/7z2301-x64.exe", TimeSpan.FromMinutes(5)))
            {
                if (!response.IsSuccessStatusCode)
                {
                    await LogAsync("[ERROR] Download failed");
                    return false;
                }

                using (var fs = host.CreateFile(installerPath))
                {
                    ownsInstaller = true;
                    await response.Content.CopyToAsync(fs);
                    fs.Position = 0;
                    using var executable = new System.Reflection.PortableExecutable.PEReader(fs, System.Reflection.PortableExecutable.PEStreamOptions.LeaveOpen);
                    if (executable.PEHeaders.PEHeader == null ||
                        executable.PEHeaders.CoffHeader.Machine != System.Reflection.PortableExecutable.Machine.Amd64 ||
                        !executable.PEHeaders.CoffHeader.Characteristics.HasFlag(System.Reflection.PortableExecutable.Characteristics.ExecutableImage))
                        throw new IOException("Downloaded installer is not an x64 Windows executable");
                }
            }

            await LogAsync("[INSTALLING] Running installer...");

            var psi = new System.Diagnostics.ProcessStartInfo
            {
                FileName = installerPath,
                Arguments = "/S",
                UseShellExecute = true,
                CreateNoWindow = true
            };

            using (var process = host.StartProcess(psi))
            {
                if (process == null)
                {
                    await LogAsync("[ERROR] Failed to start installer");
                    return false;
                }

                if (!process.WaitForExit(300000))
                {
                    process.Terminate();
                    await LogAsync("[ERROR] 7-Zip installer timed out");
                    return false;
                }
                if (process.ExitCode != 0)
                {
                    await LogAsync($"[ERROR] 7-Zip installer failed (exit code {process.ExitCode})");
                    return false;
                }
            }

            await host.DelayAsync(TimeSpan.FromSeconds(3));

            bool found = !string.IsNullOrEmpty(Find7zPath());

            return found;
        }
        catch (Exception ex)
        {
            await LogAsync($"[ERROR] {ex.Message}");
            return false;
        }
        finally
        {
            if (ownsInstaller && installerPath != null)
            {
                try { host.DeleteFile(installerPath); }
                catch (Exception ex) { await LogAsync($"[WARN] Temporary installer cleanup failed: {ex.Message}"); }
            }
        }
    }
    private string? Find7zPath()
    {
        var possiblePaths = new[]
        {
            "C:\\Program Files\\7-Zip\\7z.exe",
            "C:\\Program Files (x86)\\7-Zip\\7z.exe"
        };

        return possiblePaths.FirstOrDefault(host.FileExists);
    }

    private bool HasMsvcToolchain()
    {
        string? vswhere;
        try { vswhere = FindVsWhere(); }
        catch { return false; }
        if (vswhere != null)
        {
            try
            {
                var psi = new System.Diagnostics.ProcessStartInfo
                {
                    FileName = vswhere,
                    Arguments = "-latest -prerelease -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -find VC\\Tools\\MSVC\\**\\bin\\Hostx64\\x64\\cl.exe",
                    UseShellExecute = false,
                    RedirectStandardOutput = true,
                    CreateNoWindow = true
                };
                using var p = host.StartProcess(psi);
                if (p != null)
                {
                    if (!p.WaitForExit(30000))
                    {
                        p.Terminate();
                        return false;
                    }
                    var output = p.ReadStandardOutput().Trim();
                    if (p.ExitCode == 0 && output.Split(new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
                        .Any(path => Path.IsPathFullyQualified(path) &&
                            path.EndsWith(@"\bin\Hostx64\x64\cl.exe", StringComparison.OrdinalIgnoreCase) && host.FileExists(path)))
                        return true;
                }
            }
            catch { }
        }

        string[] basePaths = {
            @"C:\Program Files\Microsoft Visual Studio",
            @"C:\Program Files (x86)\Microsoft Visual Studio"
        };
        string[] years = { "2026", "2022" };
        string[] editions = { "BuildTools", "Community", "Professional", "Enterprise" };

        foreach (var b in basePaths)
            foreach (var y in years)
                foreach (var e in editions)
                {
                    try
                    {
                        if (host.EnumerateFiles(Path.Combine(b, y, e, "VC", "Tools", "MSVC"), "cl.exe")
                            .Any(path => path.EndsWith(@"\bin\Hostx64\x64\cl.exe", StringComparison.OrdinalIgnoreCase) && host.FileExists(path)))
                            return true;
                    }
                    catch { }
                }
        return false;
    }

    private string? FindVsWhere()
    {
        var candidates = new[]
        {
            @"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe",
            @"C:\Program Files\Microsoft Visual Studio\Installer\vswhere.exe"
        };
        return candidates.FirstOrDefault(host.FileExists);
    }

    private bool IsWindows11()
    {
        return host.WindowsBuild >= 22000;
    }

    private async Task<int> InstallMsvcAsync(InstallationConfig config)
    {
        ISystemCheckProcess? process = null;
        try
        {
            string wingetId;
            if (IsWindows11())
            {
                await LogAsync("[INSTALLING] Windows 11 detected : installing Visual Studio Build Tools 2026...");
                wingetId = "Microsoft.VisualStudio.BuildTools";
            }
            else
            {
                await LogAsync("[INSTALLING] Windows 10 detected : installing Visual Studio Build Tools 2022...");
                wingetId = "Microsoft.VisualStudio.2022.BuildTools";
            }

            var psi = new System.Diagnostics.ProcessStartInfo
            {
                FileName = "winget",
                Arguments = $"install {wingetId} " +
                            "--override \"--wait --passive --norestart " +
                            "--add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 " +
                            "--add Microsoft.VisualStudio.Component.Windows11SDK.22621 " +
                            "--add Microsoft.VisualStudio.Component.VC.Redist.14.Latest\"",
                UseShellExecute = false,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                CreateNoWindow = true
            };

            process = host.StartProcess(psi);
            if (process == null) return -1;

            if (!process.WaitForExit(3600000))
            {
                var retained = process;
                pendingInstaller = retained;
                process = null;
                _ = ObserveMsvcExitAsync(retained, config);
                await LogAsync("[ERROR] Microsoft prerequisite installation timed out. The Microsoft installer may still be running; let it finish before retrying.");
                return -1;
            }
            return process.ExitCode;
        }
        catch (Exception ex)
        {
            if (process != null && !process.HasExited)
            {
                var retained = process;
                pendingInstaller = retained;
                process = null;
                _ = ObserveMsvcExitAsync(retained, config);
            }
            await LogAsync($"[ERROR] {ex.Message}");
            return -1;
        }
        finally { process?.Dispose(); }
    }

    private async Task ObserveMsvcExitAsync(ISystemCheckProcess process, InstallationConfig config)
    {
        try
        {
            await Task.Run(process.WaitForExit);
            if (!process.HasExited) return;
            await LogAsync("[INFO] Microsoft prerequisite installer has finished. Select Retry to verify the installed toolchain.");
        }
        catch (Exception error)
        {
            await LogAsync($"[WARN] Installer completion could not be observed: {error.Message}. Wait for Microsoft's installer to finish before restarting Weave.");
        }
        finally
        {
            if (process.HasExited)
            {
                if (process.ExitCode == 3010) config.NeedsReboot = true;
                pendingInstaller = null;
                process.Dispose();
                UpdateButtons();
            }
        }
    }

    private Task LogAsync(string message)
    {
        MessageLogged?.Invoke(message);
        logCallback?.Invoke(message);
        return InstallationStepUi.AppendLogAsync(logBox, message, refresh: true);
    }

    private void UpdateStatus(string text, Color color) => InstallationStepUi.UpdateStatus(statusLabel, text, color);

    private void EnableButton()
    {
        UpdateButtons();
        ProceedChanged?.Invoke(checksPass);
    }

    private void UpdateButtons()
    {
        var idle = running == 0 && pendingInstaller == null;
        if (nextButton?.IsDisposed == false && nextButton.IsHandleCreated)
            nextButton.Invoke(new Action(() => nextButton.Enabled = checksPass && idle));
        if (retryButton?.IsDisposed == false && retryButton.IsHandleCreated)
            retryButton.Invoke(new Action(() => retryButton.Enabled = !checksPass && idle));
    }

    private bool IsAdministrator()
    {
        try
        {
            return host.IsAdministrator();
        }
        catch
        {
            return false;
        }
    }
}
