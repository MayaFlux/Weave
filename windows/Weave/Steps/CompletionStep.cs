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
    private readonly CompletionHost host;
    internal event Action<string>? ErrorReported;
    public CompletionStep() : this(new CompletionHost()) { }
    internal CompletionStep(CompletionHost host)
    {
        this.host = host;
    }
    internal static string Subtitle(InstallationConfig config) =>
        (config.SkipDownload ? "Your existing MayaFlux installation has been configured." : "MayaFlux has been successfully installed.") +
        (config.SkipDependencies ? " Dependencies were skipped; verify them before building projects." : "");

    internal static string Instructions(InstallationConfig config) =>
        (config.NeedsReboot ? "1. Restart Windows before building projects." : "1. Restart your terminal/PowerShell completely.") +
        (config.SkipDependencies ? "\r\n   Check that the required dependencies are installed before building." : "") +
        "\r\n\r\n2. Reopen Weave, choose Projects, then Create Project." +
        "\r\n\r\n3. Open an x64 Visual Studio developer shell in your project directory and build:" +
        "\r\n   cmake --preset release\r\n   cmake --build --preset release" +
        "\r\n\r\nDocumentation: https://github.com/MayaFlux/MayaFlux";

    public void BuildUI(
        LayoutManager layout,
        InstallationConfig config,
        Action<string> logCallback,
        Action nextCallback)
    {
        layout.AddTitle("Installation Complete!");

        layout.AddSubtitle(Subtitle(config));

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
            Text = $"Location: {config.MayaFluxRoot}\r\nTemplates: {config.TemplatesDirectory}"
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
            Height = 180,
            Width = layout.GetUsableWidth(),
            Text = Instructions(config)
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
            if (!OpenLog(out var error))
                MessageBox.Show(error, "Error", MessageBoxButtons.OK, MessageBoxIcon.Error);
        };

        var exitBtn = layout.AddButton("Exit", ThemeColors.ButtonPrimary);
        exitBtn.Click += (s, e) => Application.Exit();
    }
    internal bool OpenLog(out string? error)
    {
        error = null;
        try
        {
            var logPath = host.LogPath;
            if (!host.FileExists(logPath)) throw new FileNotFoundException($"Installation log was not found: {logPath}");
            var request = new ProcessStartInfo { FileName = "notepad.exe", UseShellExecute = false };
            request.ArgumentList.Add(logPath);
            if (!host.StartProcess(request)) throw new IOException("The log viewer could not be started. Retry opening the log.");
            return true;
        }
        catch (Exception ex)
        {
            error = $"Failed to open log: {ex.Message}";
            ErrorReported?.Invoke(error);
            return false;
        }
    }
}
