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
    private Label? statusLabel;
    private Button? nextButton;

    public void BuildUI(
        LayoutManager layout,
        InstallationConfig config,
        Action<string> logCallback,
        Action nextCallback)
    {
        layout.AddTitle("Step 1: System Check");

        statusLabel = layout.AddStatusLabel("Verifying system requirements...");

        logBox = layout.AddLogBox(LayoutConstants.LogBoxMaxHeight);

        layout.AddFlexibleSpacer();

        (nextButton, var cancelButton) = layout.AddButtonPair("Next >", "Cancel");
        nextButton.Enabled = false;
        nextButton.Click += (s, e) => nextCallback();
        cancelButton.Click += (s, e) => Application.Exit();

        Task.Delay(500).ContinueWith(_ => PerformChecksAsync());
    }

    private async Task PerformChecksAsync()
    {
        checksPass = true;

        await LogAsync("Checking Windows architecture...");
        await Task.Delay(200);

        if (IntPtr.Size == 8)
        {
            await LogAsync("[OK] 64-bit Windows detected");
        }
        else
        {
            await LogAsync("[ERROR] 64-bit Windows required");
            checksPass = false;
        }

        await LogAsync("");
        await LogAsync("Checking for 7-Zip...");
        await Task.Delay(200);

        var sevenZipPath = Find7zPath();
        if (!string.IsNullOrEmpty(sevenZipPath))
        {
            await LogAsync($"[OK] 7-Zip found at: {sevenZipPath}");
        }
        else
        {
            await LogAsync("[WARN] 7-Zip not found - please install from https://www.7-zip.org/");
            checksPass = false;
        }

        await LogAsync("");
        await LogAsync("Checking for administrator privileges...");
        await Task.Delay(200);

        if (IsAdministrator())
        {
            await LogAsync("[OK] Running as administrator");
        }
        else
        {
            await LogAsync("[WARN] Not running as administrator - installation may fail");
        }

        await LogAsync("");
        await LogAsync("[OK] System check complete");

        if (statusLabel != null && statusLabel.Parent != null)
        {
            statusLabel.Invoke(new Action(() =>
            {
                statusLabel.Text = checksPass ? "All checks passed!" : "Some checks failed.";
                statusLabel.ForeColor = checksPass ? ThemeColors.Success : ThemeColors.Error;
            }));
        }

        if (nextButton != null && nextButton.Parent != null)
        {
            nextButton.Invoke(new Action(() =>
            {
                nextButton.Enabled = true;
            }));
        }
    }

    private async Task LogAsync(string message)
    {
        if (logBox != null && logBox.Parent != null)
        {
            await logBox.Invoke(new Func<Task>(async () =>
            {
                logBox.AppendText(message + Environment.NewLine);
                logBox.Refresh();
                await Task.CompletedTask;
            }));
        }
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