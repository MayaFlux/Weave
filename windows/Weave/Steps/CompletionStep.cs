using System;
using System.Diagnostics;
using System.IO;
using System.Windows.Forms;
using Weave.Shared.Models;
using Weave.Modes;
using Weave.Utils;

namespace Weave.UI.Pages;

public class CompletionStep : IInstallationStep
{
    public Panel CreateUI(InstallationConfig config, Action<string> logCallback, Action nextCallback, InstallationMode parent)
    {
        var panel = new Panel { BackColor = Color.White, Padding = new Padding(20) };

        var titleLabel = new Label
        {
            Text = "Installation Complete!",
            Font = new Font("Segoe UI", 16, FontStyle.Bold),
            AutoSize = true,
            Location = new Point(0, 20),
            ForeColor = Color.FromArgb(16, 124, 16)
        };
        panel.Controls.Add(titleLabel);

        var messageLabel = new Label
        {
            Text = "MayaFlux has been successfully installed.",
            Font = new Font("Segoe UI", 11),
            AutoSize = true,
            Location = new Point(0, 60)
        };
        panel.Controls.Add(messageLabel);

        var instructionsBox = new TextBox
        {
            Multiline = true,
            ReadOnly = true,
            Font = new Font("Consolas", 9),
            BackColor = Color.FromArgb(240, 240, 240),
            Location = new Point(0, 100),
            Width = panel.Width - 40,
            Height = 200,
            Text = $@"Installation Details:
Location: {config.MayaFluxRoot}
Templates: {config.TemplatesDirectory}

Next Steps:
1. Restart your terminal completely
2. Verify installation: cmake --version
3. Create a new project: weave new MyProject
4. Build: cd MyProject && mkdir build && cd build && cmake .. && cmake --build . --config Release

Documentation: https://github.com/MayaFlux/MayaFlux"
        };
        panel.Controls.Add(instructionsBox);

        var exitButton = new Button
        {
            Text = "Exit",
            Width = 100,
            Height = 40,
            Location = new Point(panel.Width - 120, panel.Height - 60),
            BackColor = Color.FromArgb(0, 120, 215),
            ForeColor = Color.White,
            FlatStyle = FlatStyle.Flat
        };
        exitButton.Click += (s, e) => Application.Exit();
        panel.Controls.Add(exitButton);

        var logButton = new Button
        {
            Text = "View Log",
            Width = 100,
            Height = 40,
            Location = new Point(panel.Width - 240, panel.Height - 60),
            BackColor = Color.FromArgb(100, 100, 100),
            ForeColor = Color.White,
            FlatStyle = FlatStyle.Flat
        };
        logButton.Click += (s, e) =>
        {
            var logPath = new Logger().GetLogFile();
            if (File.Exists(logPath))
            {
                System.Diagnostics.Process.Start("notepad", logPath);
            }
        };
        panel.Controls.Add(logButton);

        return panel;
    }
}