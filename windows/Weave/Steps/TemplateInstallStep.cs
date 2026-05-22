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
    private Logger logger = new();
    private bool extractSuccess = false;
    private TextBox? logBox;
    private Button? nextButton;

    public void BuildUI(
        LayoutManager layout,
        InstallationConfig config,
        Action<string> logCallback,
        Action nextCallback)
    {
        layout.AddTitle("Step 5: Install Templates & Tools");

        var statusLabel = layout.AddStatusLabel("Extracting project templates and tools...");

        logBox = layout.AddLogBox(LayoutConstants.LogBoxMaxHeight);

        layout.AddFlexibleSpacer();

        (nextButton, var cancelButton) = layout.AddButtonPair("Next >", "Cancel");
        nextButton.Enabled = false;
        nextButton.Click += (s, e) => nextCallback();
        cancelButton.Click += (s, e) => Application.Exit();

        Task.Run(() => ExtractResourcesAsync(config, statusLabel, logCallback));
    }

    private async Task ExtractResourcesAsync(InstallationConfig config, Label statusLabel, Action<string> logCallback)
    {
        try
        {
            await LogAsync("=== Template & Tool Installation ===");
            await LogAsync("");
            await LogAsync("Extracting embedded resources...");
            await LogAsync($"Target directory: {config.MayaFluxRoot}");
            await LogAsync("");

            await Task.Run(() =>
            {
                ResourceExtractor.ExtractAllResources(config.MayaFluxRoot);
            });

            await LogAsync("[OK] Resource extraction complete");
            await LogAsync("");

            await LogAsync("Verifying templates installation...");
            if (Directory.Exists(config.TemplatesDirectory))
            {
                var required = new[]
                {
                    "CMakeLists.txt",
                    "main.cpp",
                    "user_project.hpp",
                    ".gitignore",
                    Path.Combine("cmake", "mayaflux.cmake"),
                    Path.Combine("cmake", "shaders.cmake"),
                    Path.Combine("cmake", "build_community.cmake"),

                    Path.Combine("community", "module.cmake"),
                    Path.Combine("community", "community.json"),
                    Path.Combine("community", "test", "CMakeLists.txt"),

                    Path.Combine("vscode", "settings.json"),
                    Path.Combine("vscode", "tasks.json"),
                    Path.Combine("vscode", "launch.json"),
                };

                var missing = new System.Collections.Generic.List<string>();
                foreach (var rel in required)
                {
                    var full = Path.Combine(config.TemplatesDirectory, rel);
                    if (File.Exists(full))
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

                var totalFiles = Directory.GetFiles(config.TemplatesDirectory, "*", SearchOption.AllDirectories);
                await LogAsync($"[OK] Templates directory: {config.TemplatesDirectory}");
                await LogAsync($"[OK] {totalFiles.Length} total template files");
            }
            else
            {
                throw new Exception($"Templates directory not found: {config.TemplatesDirectory}");
            }

            await LogAsync("");

            await LogAsync("Verifying scripts installation...");
            if (Directory.Exists(config.ScriptsDirectory))
            {
                var scriptFiles = Directory.GetFiles(config.ScriptsDirectory, "*.ps1", SearchOption.AllDirectories);
                await LogAsync($"[OK] Scripts directory: {config.ScriptsDirectory}");
                await LogAsync($"[OK] Found {scriptFiles.Length} PowerShell scripts");
            }
            else
            {
                await LogAsync($"[WARN] Scripts directory not found: {config.ScriptsDirectory}");
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
            EnableButton();
        }
    }

    private async Task LogAsync(string message)
    {
        if (logBox?.Parent != null)
        {
            await logBox.Invoke(new Func<Task>(async () =>
            {
                logBox.AppendText(message + Environment.NewLine);
                await Task.CompletedTask;
            }));
        }
    }

    private void UpdateStatus(Label statusLabel, string text, Color color)
    {
        if (statusLabel.Parent != null)
        {
            statusLabel.Invoke(new Action(() =>
            {
                statusLabel.Text = text;
                statusLabel.ForeColor = color;
            }));
        }
    }

    private void EnableButton()
    {
        if (nextButton?.Parent != null)
        {
            nextButton.Invoke(new Action(() =>
            {
                nextButton.Enabled = true;
            }));
        }
    }
}