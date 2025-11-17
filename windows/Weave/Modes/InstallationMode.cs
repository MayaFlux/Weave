using Weave.Shared;
using Weave.Shared.Models;
using Weave.UI.Pages;

namespace Weave.Modes;

public class InstallationMode : IWeaveMode
{
    private InstallationConfig config = new();
    private Panel? containerPanel;
    private int currentStep = 0;
    private IInstallationStep[] steps = Array.Empty<IInstallationStep>();

    public void Initialize(Panel container)
    {
        containerPanel = container;
        containerPanel.Controls.Clear();

        // Initialize steps
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
        if (stepIndex >= steps.Length)
            return;

        currentStep = stepIndex;
        containerPanel?.Controls.Clear();

        var step = steps[stepIndex];
        var stepPanel = step.CreateUI(config, LogCallback, NextStepCallback, this);

        if (stepPanel != null && containerPanel != null)
        {
            stepPanel.Dock = DockStyle.Fill;
            containerPanel.Controls.Add(stepPanel);
        }
    }

    private void LogCallback(string message)
    {
        // Will be implemented in step UI
    }

    private void NextStepCallback()
    {
        if (currentStep < steps.Length - 1)
        {
            ShowStep(currentStep + 1);
        }
    }
}