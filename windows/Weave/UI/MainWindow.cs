using Weave.Shared.Models;
using Weave.Modes;

namespace Weave.UI;

public partial class MainWindow : Form
{
    private WeaveMode currentMode;
    private IWeaveMode modeHandler;
    private Panel contentPanel;

    public MainWindow(WeaveMode mode)
    {
        currentMode = mode;
        InitializeComponent();
        SetupUI();
        ShowMode();
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

        // Content panel (will be replaced by mode-specific UI)
        contentPanel = new Panel
        {
            Dock = DockStyle.Fill,
            BackColor = Color.White
        };
        Controls.Add(contentPanel);
    }

    private void ShowMode()
    {
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

        modeHandler.Initialize(contentPanel);
    }
}