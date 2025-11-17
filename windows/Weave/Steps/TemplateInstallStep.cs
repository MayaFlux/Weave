using Weave.Shared.Models;
using Weave.Modes;
using Weave.Utils;

namespace Weave.UI.Pages;

public class TemplatesInstallStep : IInstallationStep
{
    private Logger logger = new();
    private bool extractSuccess = false;

    public Panel CreateUI(InstallationConfig config, Action<string> logCallback, Action nextCallback, InstallationMode parent)
    {
        var panel = new Panel { BackColor = Color.White, Padding = new Padding(20) };

        var titleLabel = new Label
        {
            Text = "Step 5: Install Templates & Tools",
            Font = new Font("Segoe UI", 14, FontStyle.Bold),
            AutoSize = true,
            Location = new Point(0, 0)
        };
        panel.Controls.Add(titleLabel);

        var statusLabel = new Label
        {
            Text = "Extracting project templates and tools...",
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
            Location = new Point(0, 70),
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

        Task.Run(() => ExtractResourcesAsync(
            config,
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
                    nextButton.Enabled = true;
                    statusLabel.Text = extractSuccess ? "Templates extracted!" : "Extraction completed with warnings";
                    statusLabel.ForeColor = extractSuccess ? Color.Green : Color.Orange;
                }));
            }
        ));

        return panel;
    }

    private async Task ExtractResourcesAsync(InstallationConfig config, Action<string> log, Action onComplete)
    {
        await Task.Run(() =>
        {
            try
            {
                log("Extracting embedded resources...");
                ResourceExtractor.ExtractAllResources(config.MayaFluxRoot);

                log($"[OK] Templates extracted to: {config.TemplatesDirectory}");
                log($"[OK] Scripts extracted to: {config.ScriptsDirectory}");

                // Verify
                if (Directory.Exists(config.TemplatesDirectory))
                {
                    var templateFiles = Directory.GetFiles(config.TemplatesDirectory, "*", SearchOption.AllDirectories);
                    log($"[OK] Found {templateFiles.Length} template files");
                }

                extractSuccess = true;
            }
            catch (Exception ex)
            {
                log($"[ERROR] {ex.Message}");
            }
            finally
            {
                onComplete?.Invoke();
            }
        });
    }
}