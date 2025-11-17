using System;
using System.Drawing;
using System.Security.Principal;
using System.Windows.Forms;
using Weave.Shared.Models;
using Weave.Theme;
using static Weave.Theme.ThemeExtensions;

namespace Weave;

public class ModeSelector : Form
{
    public WeaveMode? SelectedMode { get; private set; }

    public ModeSelector()
    {
        InitializeUI();
    }

    private void InitializeUI()
    {
        Text = "Weave - Select Mode";
        Width = 500;
        Height = 300;
        StartPosition = FormStartPosition.CenterScreen;
        FormBorderStyle = FormBorderStyle.FixedDialog;
        MaximizeBox = false;
        MinimizeBox = false;
        Icon = SystemIcons.Application;
        ShowInTaskbar = true;

        this.ApplyDarkTheme();

        var titleLabel = new Label
        {
            Text = "What would you like to do?",
            Font = new Font("Segoe UI", 14, FontStyle.Bold),
            Location = new Point(20, 20),
            AutoSize = true,
            BackColor = ThemeColors.BackgroundDark,
            ForeColor = ThemeColors.TextPrimary
        };
        Controls.Add(titleLabel);

        var installButton = new Button
        {
            Text = "Install MayaFlux",
            Location = new Point(50, 70),
            Width = 380,
            Height = 80,
            BackColor = ThemeColors.ButtonPrimary,
            ForeColor = Color.White,
            Font = new Font("Segoe UI", 11, FontStyle.Bold),
            FlatStyle = FlatStyle.Flat,
            TextAlign = ContentAlignment.MiddleCenter
        };
        installButton.Click += (s, e) => HandleInstallClick();
        Controls.Add(installButton);

        var projectButton = new Button
        {
            Text = "Create Project",
            Location = new Point(50, 160),
            Width = 380,
            Height = 80,
            BackColor = ThemeColors.ButtonSuccess,
            ForeColor = Color.White,
            Font = new Font("Segoe UI", 11, FontStyle.Bold),
            FlatStyle = FlatStyle.Flat,
            TextAlign = ContentAlignment.MiddleCenter
        };
        projectButton.Click += (s, e) =>
        {
            SelectedMode = WeaveMode.ProjectCreation;
            DialogResult = DialogResult.OK;
            Close();
        };
        Controls.Add(projectButton);
    }

    private void HandleInstallClick()
    {
        SelectedMode = WeaveMode.Installation;
        DialogResult = DialogResult.OK;
        Close();
    }

    private bool IsRunningAsAdmin()
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