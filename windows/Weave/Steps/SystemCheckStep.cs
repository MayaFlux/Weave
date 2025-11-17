using System;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Security.Principal;
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
    private Logger logger = new();
    private bool checksPass = false;
    private TextBox? logBox;
    private Button? nextButton;

    public void BuildUI(
        LayoutManager layout,
        InstallationConfig config,
        Action<string> logCallback,
        Action nextCallback)
    {
        layout.AddTitle("Step 1: System Check");

        var statusLabel = layout.AddStatusLabel("Verifying system requirements...");

        logBox = layout.AddLogBox(LayoutConstants.LogBoxMaxHeight);

        layout.AddFlexibleSpacer();

        (nextButton, var cancelButton) = layout.AddButtonPair("Next >", "Cancel");
        nextButton.Enabled = false;
        nextButton.Click += (s, e) => nextCallback();
        cancelButton.Click += (s, e) => Application.Exit();

        Task.Run(() => PerformChecks(
            msg =>
            {
                if (logBox?.Parent != null) // Check if control still exists
                {
                    logBox.Invoke(new Action(() =>
                    {
                        logBox.AppendText(msg + Environment.NewLine);
                        logCallback(msg);
                    }));
                }
            },
            () =>
            {
                if (statusLabel.Parent != null)
                {
                    statusLabel.Invoke(new Action(() =>
                    {
                        nextButton.Enabled = checksPass;
                        statusLabel.Text = checksPass ? "All checks passed!" : "Some checks failed.";
                        statusLabel.ForeColor = checksPass ? ThemeColors.Success : ThemeColors.Error;
                    }));
                }
            }
        ));
    }

    private void PerformChecks(Action<string> log, Action onComplete)
    {
        checksPass = true;

        log("Checking Windows architecture...");
        if (IntPtr.Size == 8)
        {
            log("[OK] 64-bit Windows detected");
        }
        else
        {
            log("[ERROR] 64-bit Windows required");
            checksPass = false;
        }

        log("");
        log("Checking for 7-Zip...");
        var sevenZipPath = Find7zPath();
        if (!string.IsNullOrEmpty(sevenZipPath))
        {
            log($"[OK] 7-Zip found at: {sevenZipPath}");
        }
        else
        {
            log("[WARN] 7-Zip not found - please install from https://www.7-zip.org/");
            checksPass = false;
        }

        log("");
        log("Checking for administrator privileges...");
        if (IsAdministrator())
        {
            log("[OK] Running as administrator");
        }
        else
        {
            log("[WARN] Not running as administrator - installation may fail");
        }

        log("");
        log("[OK] System check complete");
        onComplete?.Invoke();
    }

    private string? Find7zPath()
    {
        var possiblePaths = new[]
        {
            "C:\\Program Files\\7-Zip\\7z.exe",
            "C:\\Program Files (x86)\\7-Zip\\7z.exe"
        };

        return possiblePaths.FirstOrDefault(path => File.Exists(path));
    }

    private bool IsAdministrator()
    {
        try
        {
            var identity = WindowsIdentity.GetCurrent();
            var principal = new WindowsPrincipal(identity);
            return principal.IsInRole(WindowsBuiltInRole.Administrator);
        }
        catch
        {
            return false;
        }
    }
}