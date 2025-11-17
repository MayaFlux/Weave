using Weave.Shared.Models;
using Weave.Modes;
using Weave.Utils;

namespace Weave.UI.Pages;

public class SystemCheckStep : IInstallationStep
{
    private Logger logger = new();
    private bool checksPass = false;

    public Panel CreateUI(InstallationConfig config, Action<string> logCallback, Action nextCallback, InstallationMode parent)
    {
        var panel = new Panel { BackColor = Color.White, Padding = new Padding(20) };

        var titleLabel = new Label
        {
            Text = "Step 1: System Check",
            Font = new Font("Segoe UI", 14, FontStyle.Bold),
            AutoSize = true,
            Location = new Point(0, 0)
        };
        panel.Controls.Add(titleLabel);

        var statusLabel = new Label
        {
            Text = "Verifying system requirements...",
            Font = new Font("Segoe UI", 10),
            AutoSize = true,
            Location = new Point(0, 40)
        };
        panel.Controls.Add(statusLabel);

        var logBox = new TextBox
        {
            Multiline = true,
            ReadOnly = true,
            Font = new Font("Consolas", 9),
            BackColor = Color.FromArgb(31, 31, 31),
            ForeColor = Color.FromArgb(220, 220, 220),
            Location = new Point(0, 80),
            Width = panel.Width - 40,
            Height = 300,
            ScrollBars = ScrollBars.Vertical
        };
        panel.Controls.Add(logBox);

        var nextButton = new Button
        {
            Text = "Next >",
            Width = 100,
            Height = 40,
            Location = new Point(panel.Width - 120, panel.Height - 60),
            BackColor = Color.FromArgb(0, 120, 215),
            ForeColor = Color.White,
            FlatStyle = FlatStyle.Flat,
            Enabled = false
        };
        nextButton.Click += (s, e) => nextCallback();
        panel.Controls.Add(nextButton);

        var skipButton = new Button
        {
            Text = "Cancel",
            Width = 100,
            Height = 40,
            Location = new Point(panel.Width - 240, panel.Height - 60),
            BackColor = Color.FromArgb(200, 200, 200),
            ForeColor = Color.Black,
            FlatStyle = FlatStyle.Flat
        };
        skipButton.Click += (s, e) => Application.Exit();
        panel.Controls.Add(skipButton);

        // Run checks asynchronously
        Task.Run(() => PerformChecks(
            msg =>
            {
                panel.Invoke(new Action(() =>
                {
                    logBox.AppendText(msg + Environment.NewLine);
                    logCallback(msg);
                }));
            },
            () =>
            {
                panel.Invoke(new Action(() =>
                {
                    nextButton.Enabled = checksPass;
                    statusLabel.Text = checksPass ? "All checks passed!" : "Some checks failed.";
                    statusLabel.ForeColor = checksPass ? Color.Green : Color.Red;
                }));
            }
        ));

        return panel;
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
            var identity = System.Security.Principal.WindowsIdentity.GetCurrent();
            var principal = new System.Security.Principal.WindowsPrincipal(identity);
            return principal.IsInRole(System.Security.Principal.WindowsBuiltInRole.Administrator);
        }
        catch
        {
            return false;
        }
    }
}
