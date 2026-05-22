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
    private ReleaseType releaseType;
    private IWeaveMode? modeHandler;
    private Panel? contentPanel;

    public MainWindow(WeaveMode mode, ReleaseType release = ReleaseType.Stable)
    {
        currentMode = mode;
        releaseType = release;
        InitializeComponent();
        SetupUI();
        ShowMode();
    }

    private void InitializeComponent()
    {
        // WinForms designer empty for not using designer
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
        Icon = new Icon(Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "weave.ico"));

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
                modeHandler = new InstallationMode(releaseType);
                break;
            case WeaveMode.ProjectCreation:
                modeHandler = new ProjectCreationMode();
                break;
            case WeaveMode.UpdateProject:
                // no-op this commit — placeholder
                break;
            case WeaveMode.CreateCommunity:
                // no-op this commit — placeholder
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
