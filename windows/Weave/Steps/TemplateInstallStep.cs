using System;
using System.Drawing;
using System.IO;
using System.Threading.Tasks;
using System.Windows.Forms;
using Weave.Modes;
using Weave.Shared.Models;
using Weave.Theme;
using Weave.UI.Layout;
using Weave.Utils;

namespace Weave.UI.Pages;

public class TemplatesInstallStep : IInstallationStep
{
    private readonly ResourceExtractionHost host;
    internal event Action<string>? MessageLogged;
    internal event Action<bool>? ProceedChanged;
    public TemplatesInstallStep() : this(new ResourceExtractionHost()) { }
    internal TemplatesInstallStep(ResourceExtractionHost host) => this.host = host;
    private bool extractSuccess = false;
    private TextBox? logBox;
    private Button? nextButton;
    private Button? retryButton;
    private int running;
    private Action<string>? logCallback;

    public void BuildUI(
        LayoutManager layout,
        InstallationConfig config,
        Action<string> logCallback,
        Action nextCallback)
    {
        this.logCallback = logCallback;
        layout.AddTitle("Step 5: Install Templates & Tools");

        var statusLabel = layout.AddStatusLabel("Extracting project templates and tools...");

        logBox = layout.AddLogBox(LayoutConstants.LogBoxMaxHeight);

        layout.AddFlexibleSpacer();

        (nextButton, var cancelButton) = layout.AddButtonPair("Next >", "Cancel");
        nextButton.Enabled = false;
        nextButton.Click += (s, e) => nextCallback();
        cancelButton.Click += (s, e) => Application.Exit();
        retryButton = layout.AddButton("Retry", ThemeColors.ButtonSecondary);
        retryButton.Enabled = false;
        retryButton.Click += (s, e) => Task.Run(() => ExtractResourcesAsync(config, statusLabel, logCallback));

        InstallationStepUi.RunWhenReady(logBox, () => ExtractResourcesAsync(config, statusLabel, logCallback));
    }

    internal async Task ExtractResourcesAsync(InstallationConfig config, Label? statusLabel = null, Action<string>? logCallback = null)
    {
        if (System.Threading.Interlocked.CompareExchange(ref running, 1, 0) != 0) return;
        extractSuccess = false;
        EnableButton();
        try
        {
            await LogAsync("=== Template & Tool Installation ===");
            await LogAsync("");
            await LogAsync("Extracting embedded resources...");
            await LogAsync($"Target directory: {config.MayaFluxRoot}");
            await LogAsync("");

            await Task.Run(() =>
            {
                ResourceExtractor.ExtractAllResources(config.MayaFluxRoot, host);
            });

            await LogAsync("[OK] Resource extraction complete");
            await LogAsync("");

            await LogAsync("Verifying templates installation...");
            if (host.DirectoryExists(config.TemplatesDirectory))
            {
                var required = ResourceExtractor.RequiredTemplates;

                var missing = new System.Collections.Generic.List<string>();
                foreach (var rel in required)
                {
                    var full = Path.Combine(config.TemplatesDirectory, rel);
                    if (host.FileExists(full) && host.FileLength(full) > 0)
                        await LogAsync($"[OK] {rel}");
                    else
                        missing.Add(rel);
                }

                if (missing.Count > 0)
                {
                    foreach (var m in missing)
                        await LogAsync($"[ERROR] Missing: {m}");
                    throw new Exception($"{missing.Count} required template file(s) missing — installation is broken.");
                }

                var totalFiles = host.GetFiles(config.TemplatesDirectory, "*", SearchOption.AllDirectories);
                await LogAsync($"[OK] Templates directory: {config.TemplatesDirectory}");
                await LogAsync($"[OK] {totalFiles.Length} total template files");
            }
            else
            {
                throw new Exception($"Templates directory not found: {config.TemplatesDirectory}");
            }

            await LogAsync("");

            if (host.ResourceNames.Any(name => name.StartsWith("Weave.Resources.scripts.", StringComparison.OrdinalIgnoreCase)))
            {
                if (!host.DirectoryExists(config.ScriptsDirectory)) throw new IOException($"Scripts directory not found: {config.ScriptsDirectory}");
                var scriptFiles = host.GetFiles(config.ScriptsDirectory, "*.ps1", SearchOption.AllDirectories);
                await LogAsync($"[OK] Scripts directory: {config.ScriptsDirectory}");
                await LogAsync($"[OK] Found {scriptFiles.Length} PowerShell scripts");
            }

            await LogAsync("");
            await LogAsync("=== Template & Tool Installation Complete ===");
            await LogAsync("");

            extractSuccess = true;
            UpdateStatus(statusLabel, "Templates & tools installed", ThemeColors.Success);
        }
        catch (Exception ex)
        {
            await LogAsync($"[ERROR] {ex.Message}");
            await LogAsync("[ERROR] Failed to extract templates and tools");
            UpdateStatus(statusLabel, "Extraction failed", ThemeColors.Error);
        }
        finally
        {
            System.Threading.Interlocked.Exchange(ref running, 0);
            EnableButton();
        }
    }

    private Task LogAsync(string message)
    {
        MessageLogged?.Invoke(message);
        logCallback?.Invoke(message);
        return InstallationStepUi.AppendLogAsync(logBox, message);
    }

    private void UpdateStatus(Label? statusLabel, string text, Color color) => InstallationStepUi.UpdateStatus(statusLabel, text, color);

    private void EnableButton()
    {
        ProceedChanged?.Invoke(extractSuccess && running == 0);
        if (nextButton?.Parent != null)
        {
            nextButton.Invoke(new Action(() =>
            {
                nextButton.Enabled = extractSuccess && running == 0;
            }));
        }
        if (retryButton?.Parent != null)
            retryButton.Invoke(new Action(() => retryButton.Enabled = !extractSuccess && running == 0));
    }
}
