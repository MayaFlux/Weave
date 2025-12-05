using System;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Net.Http;
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

        Task.Run(() => PerformChecksAsync());
    }

    private async Task PerformChecksAsync()
    {
        checksPass = true;

        await LogAsync("Checking Windows architecture...");
        if (IntPtr.Size == 8)
        {
            await LogAsync("[OK] 64-bit Windows");
        }
        else
        {
            await LogAsync("[ERROR] 64-bit Windows required");
            checksPass = false;
        }

        await LogAsync("");
        await LogAsync("Checking for administrator privileges...");
        if (IsAdministrator())
        {
            await LogAsync("[OK] Running as administrator");
        }
        else
        {
            await LogAsync("[ERROR] Not running as administrator");
            checksPass = false;
            UpdateStatus("Admin privileges required", ThemeColors.Error);
            EnableButton();
            return;
        }

        await LogAsync("");
        await LogAsync("Checking for 7-Zip...");

        if (string.IsNullOrEmpty(Find7zPath()))
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
        if (checksPass)
        {
            await LogAsync("[OK] All prerequisites met");
            UpdateStatus("Ready to proceed", ThemeColors.Success);
        }
        else
        {
            await LogAsync("[ERROR] Cannot proceed");
            UpdateStatus("Prerequisites missing", ThemeColors.Error);
        }

        EnableButton();
    }

    private async Task<bool> Install7ZipAsync()
    {
        try
        {
            string tempDir = Path.GetTempPath();
            string installerPath = Path.Combine(tempDir, "7z-install.exe");

            using (var client = new HttpClient())
            {
                client.Timeout = TimeSpan.FromMinutes(5);
                var response = await client.GetAsync("https://www.7-zip.org/a/7z2301-x64.exe");

                if (!response.IsSuccessStatusCode)
                {
                    await LogAsync("[ERROR] Download failed");
                    return false;
                }

                using (var fs = File.Create(installerPath))
                {
                    await response.Content.CopyToAsync(fs);
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

            using (var process = System.Diagnostics.Process.Start(psi))
            {
                if (process == null)
                {
                    await LogAsync("[ERROR] Failed to start installer");
                    return false;
                }

                process.WaitForExit(300000); 
            }

            await Task.Delay(3000);

            bool found = !string.IsNullOrEmpty(Find7zPath());

            try { File.Delete(installerPath); } catch { }

            return found;
        }
        catch (Exception ex)
        {
            await LogAsync($"[ERROR] {ex.Message}");
            return false;
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

    private void UpdateStatus(string text, Color color)
    {
        if (statusLabel != null && statusLabel.Parent != null)
        {
            statusLabel.Invoke(new Action(() =>
            {
                statusLabel.Text = text;
                statusLabel.ForeColor = color;
            }));
        }
    }

    private void EnableButton()
    {
        if (nextButton != null && nextButton.Parent != null)
        {
            nextButton.Invoke(new Action(() =>
            {
                nextButton.Enabled = checksPass;
            }));
        }
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