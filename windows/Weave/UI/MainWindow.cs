using System;
using System.Windows.Forms;
using Weave.Shared.Models;
using Weave.Modes;
using Weave.UI.Pages;
using static Weave.Theme.ThemeExtensions;

namespace Weave.UI;

public partial class MainWindow : Form
{
    private WeaveMode currentMode;
    private IWeaveMode? modeHandler;
    private Panel? contentPanel;

    public MainWindow(WeaveMode mode)
    {
        currentMode = mode;
        InitializeComponent();
        SetupUI();
        ShowMode();
    }

    private void InitializeComponent()
    {
        // WinForms designer - can be empty if not using designer
    }

    private void SetupUI()
    {
        Text = "Weave - MayaFlux Workspace Manager";
        Width = 900;
        Height = 700;
        StartPosition = FormStartPosition.CenterScreen;
        FormBorderStyle = FormBorderStyle.FixedDialog;
        MaximizeBox = false;
        MinimizeBox = true;
        Icon = SystemIcons.Application;

        this.ApplyDarkTheme();

        contentPanel = new Panel
        {
            Dock = DockStyle.Fill,
            BackColor = Weave.Theme.ThemeColors.BackgroundDark
        };
        Controls.Add(contentPanel);
    }

    private void ShowMode()
    {
        if (contentPanel == null)
            return;

        switch (currentMode)
        {
            case WeaveMode.Installation:
                modeHandler = new InstallationMode();
                break;
            case WeaveMode.ProjectCreation:
                modeHandler = new ProjectCreationMode();
                break;
            default:
                throw new InvalidOperationException($"Unknown mode: {currentMode}");
        }

        if (modeHandler != null)
        {
            modeHandler.Initialize(contentPanel);
        }
    }
}
