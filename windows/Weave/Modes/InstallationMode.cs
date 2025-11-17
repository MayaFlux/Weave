using System;
using System.Windows.Forms;
using Weave.Shared;
using Weave.Shared.Models;
using Weave.UI.Pages;
using Weave.UI.Layout;
using Weave.Theme;

namespace Weave.Modes;

public class InstallationMode : IWeaveMode
{
    private InstallationConfig config = new();
    private Panel? containerPanel;
    private LayoutManager? layoutManager;
    private int currentStep = 0;
    private IInstallationStep[] steps = Array.Empty<IInstallationStep>();

    public void Initialize(Panel container)
    {
        containerPanel = container;
        containerPanel.Controls.Clear();
        containerPanel.BackColor = ThemeColors.BackgroundDark;

        layoutManager = new LayoutManager(containerPanel);

        steps = new IInstallationStep[]
        {
            new SystemCheckStep(),
            new MayaFluxDownloadStep(),
            new DependenciesStep(),
            new EnvironmentSetupStep(),
            new TemplatesInstallStep(),
            new CompletionStep()
        };

        ShowStep(0);
    }

    private void ShowStep(int stepIndex)
    {
        if (stepIndex >= steps.Length || layoutManager == null)
            return;

        currentStep = stepIndex;

        layoutManager.Clear();

        var step = steps[stepIndex];
        step.BuildUI(
            layoutManager,
            config,
            LogCallback,
            NextStepCallback
        );

        layoutManager.RefreshLayout();
    }

    private void LogCallback(string message)
    {
        // Log messages are handled by individual steps updating their textboxes
        // This callback could be used for central logging if needed
    }

    private void NextStepCallback()
    {
        if (currentStep < steps.Length - 1)
        {
            ShowStep(currentStep + 1);
        }
    }
}