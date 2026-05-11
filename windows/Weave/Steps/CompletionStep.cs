using System;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.Windows.Forms;
using Weave.Modes;
using Weave.Shared.Models;
using Weave.Theme;
using Weave.UI.Layout;
using Weave.Utils;

namespace Weave.UI.Pages;

public class CompletionStep : IInstallationStep
{
    private Logger logger = new();

    public void BuildUI(
        LayoutManager layout,
        InstallationConfig config,
        Action<string> logCallback,
        Action nextCallback)
    {
        layout.AddTitle("Installation Complete!");

        layout.AddSubtitle("MayaFlux has been successfully installed.");

        var detailsLabel = new Label
        {
            Text = "Installation Details:",
            Font = new Font("Segoe UI", 10, FontStyle.Bold),
            ForeColor = ThemeColors.TextPrimary,
            BackColor = ThemeColors.BackgroundDark,
            AutoSize = true
        };
        layout.AddToStack(detailsLabel, LayoutConstants.SpacingSmall);

        var detailsBox = new TextBox
        {
            Multiline = true,
            ReadOnly = true,
            Font = new Font("Consolas", 9),
            BackColor = ThemeColors.BackgroundMedium,
            ForeColor = ThemeColors.TextPrimary,
            BorderStyle = BorderStyle.FixedSingle,
            Height = 80,
            Width = layout.GetUsableWidth(),
            Text = $"Location: {config.MayaFluxRoot}\nTemplates: {config.TemplatesDirectory}\nScripts: {config.ScriptsDirectory}"
        };
        layout.AddToStack(detailsBox,  LayoutConstants.SpacingLarge);

        var stepsLabel = new Label
        {
            Text = "Next Steps:",
            Font = new Font("Segoe UI", 10, FontStyle.Bold),
            ForeColor = ThemeColors.TextPrimary,
            BackColor = ThemeColors.BackgroundDark,
            AutoSize = true
        };
        layout.AddToStack(stepsLabel,  LayoutConstants.SpacingSmall);

        var instructionsBox = new TextBox
        {
            Multiline = true,
            ReadOnly = true,
            Font = new Font("Consolas", 9),
            BackColor = ThemeColors.BackgroundMedium,
            ForeColor = ThemeColors.TextPrimary,
            BorderStyle = BorderStyle.FixedSingle,
            Height = 140,
            Width = layout.GetUsableWidth(),
            Text = @"1. Restart your terminal/PowerShell completely

2. Verify installation:
   cmake --version
   weave --version

3. Create a new project:
   weave new MyProject C:\MyProjects

4. Build and run:
   cd MyProject\build
   cmake .. -DCMAKE_BUILD_TYPE=Release
   cmake --build . --config Release

Documentation: https://github.com/MayaFlux/MayaFlux"
        };
        layout.AddToStack(instructionsBox,  LayoutConstants.SpacingLarge);

        layout.AddFlexibleSpacer();

        if (config.NeedsReboot)
        {
            var rebootLabel = new Label
            {
                Text = "⚠ Restart Required",
                Font = new Font("Segoe UI", 10, FontStyle.Bold),
                ForeColor = Color.FromArgb(217, 119, 6),
                BackColor = ThemeColors.BackgroundDark,
                AutoSize = true
            };
            layout.AddToStack(rebootLabel, LayoutConstants.SpacingSmall);

            var rebootBox = new TextBox
            {
                Multiline = true,
                ReadOnly = true,
                Font = new Font("Segoe UI", 9),
                BackColor = Color.FromArgb(255, 251, 235),
                ForeColor = Color.FromArgb(120, 53, 15),
                BorderStyle = BorderStyle.FixedSingle,
                Height = 50,
                Width = layout.GetUsableWidth(),
                Text = "Build tools were installed and require a system restart before\nyou can build projects. Restart when ready."
            };
            layout.AddToStack(rebootBox, LayoutConstants.SpacingLarge);
        }

        layout.AddFlexibleSpacer();

        var viewLogBtn = layout.AddButton("View Log", ThemeColors.ButtonSecondary);
        viewLogBtn.Click += (s, e) =>
        {
            var logPath = logger.GetLogFile();
            if (File.Exists(logPath))
            {
                try
                {
                    Process.Start(new ProcessStartInfo
                    {
                        FileName = "notepad.exe",
                        Arguments = logPath,
                        UseShellExecute = true
                    });
                }
                catch (Exception ex)
                {
                    MessageBox.Show($"Failed to open log: {ex.Message}", "Error", MessageBoxButtons.OK, MessageBoxIcon.Error);
                }
            }
        };

        var exitBtn = layout.AddButton("Exit", ThemeColors.ButtonPrimary);
        exitBtn.Click += (s, e) => Application.Exit();
    }
}