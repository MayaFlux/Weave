using Weave.Shared.Models;
using Weave.Modes;
using Weave.Utils;
using Weave.Theme;

namespace Weave.UI.Pages;

public class SystemCheckStep : IInstallationStep
{
    private Logger logger = new();
    private bool checksPass = false;

    public Panel CreateUI(InstallationConfig config, Action<string> logCallback, Action nextCallback, InstallationMode parent)
    {
        var panel = new Panel
        {
            Dock = DockStyle.Fill,
            BackColor = ThemeColors.BackgroundDark
        };
        panel.ApplyDarkTheme();

        int margin = 20;
        int buttonHeight = 50;
        int usableWidth = panel.Width - (margin * 2);
        int usableHeight = panel.Height - buttonHeight - (margin * 3);

        var titleLabel = new Label
        {
            Text = "Step 1: System Check",
            Font = new Font("Segoe UI", 14, FontStyle.Bold),
            AutoSize = true,
            ForeColor = ThemeColors.TextPrimary,
            BackColor = ThemeColors.BackgroundDark,
            Location = new Point(margin, margin)
        };
        panel.Controls.Add(titleLabel);

        var statusLabel = new Label
        {
            Text = "Verifying system requirements...",
            Font = new Font("Segoe UI", 10),
            ForeColor = ThemeColors.TextSecondary,
            BackColor = ThemeColors.BackgroundDark,
            AutoSize = true,
            Location = new Point(margin, margin + 40)
        };
        panel.Controls.Add(statusLabel);

        var logBox = new TextBox
        {
            Multiline = true,
            ReadOnly = true,
            Font = new Font("Consolas", 9),
            BackColor = ThemeColors.BackgroundMedium,
            ForeColor = ThemeColors.TextPrimary,
            ScrollBars = ScrollBars.Vertical,
            Location = new Point(margin, margin + 70),
            Width = usableWidth,
            Height = usableHeight - 70
        };
        panel.Controls.Add(logBox);

        var nextButton = new Button
        {
            Text = "Next >",
            Width = 100,
            Height = 40,
            BackColor = ThemeColors.ButtonPrimary,
            ForeColor = Color.White,
            FlatStyle = FlatStyle.Flat,
            Enabled = false,
            Location = new Point(usableWidth + margin - 100, panel.Height - 50)
        };
        nextButton.Click += (s, e) => nextCallback();
        panel.Controls.Add(nextButton);

        var cancelButton = new Button
        {
            Text = "Cancel",
            Width = 100,
            Height = 40,
            BackColor = ThemeColors.ButtonSecondary,
            ForeColor = Color.White,
            FlatStyle = FlatStyle.Flat,
            Location = new Point(usableWidth + margin - 210, panel.Height - 50)
        };
        cancelButton.Click += (s, e) => Application.Exit();
        panel.Controls.Add(cancelButton);

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
                    statusLabel.ForeColor = checksPass ? ThemeColors.Success : ThemeColors.Error;
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
