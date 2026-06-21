using System;
using System.IO;
using System.Net.Http;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using System.Windows.Forms;
using System.Diagnostics;
using Weave.Theme;
using Weave.UI.Layout;

namespace Weave.UI.Project;

public class AddCommunityView : UserControl
{
    private TextBox projectPathInput;
    private TextBox moduleNameInput;
    private Label nameErrorLabel;
    private Button browseButton;
    private Button addButton;
    private Button cancelButton;
    private RichTextBox outputLog;
    private LayoutManager layoutManager;

    private const string RegistryUrl =
        "https://raw.githubusercontent.com/MayaFlux/community-sources-registry/main/registry.json";

    public AddCommunityView()
    {
        InitializeComponent();
        InitializeUI();
    }

    private void InitializeComponent()
    {
        BackColor = ThemeColors.BackgroundDark;
        Dock = DockStyle.Fill;
    }

    private void InitializeUI()
    {
        var scrollPanel = new Panel
        {
            Dock = DockStyle.Fill,
            BackColor = ThemeColors.BackgroundDark,
            AutoScroll = true
        };
        Controls.Add(scrollPanel);

        layoutManager = new LayoutManager(scrollPanel);

        layoutManager.AddTitle("Add Community Module");

        projectPathInput = layoutManager.AddLabeledInput("Project Directory:", "");
        projectPathInput.Text = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile) + @"\Projects";

        browseButton = layoutManager.AddButton("Browse…", ThemeColors.ButtonSecondary);
        browseButton.Click += BrowseButton_Click;

        moduleNameInput = layoutManager.AddLabeledInput("Module Name:", "my_module");
        moduleNameInput.TextChanged += (s, e) => ValidateName();

        nameErrorLabel = new Label
        {
            Text = "",
            ForeColor = ThemeColors.Error,
            BackColor = ThemeColors.BackgroundDark,
            AutoSize = true,
            Font = new System.Drawing.Font("Segoe UI", 8.5f)
        };
        layoutManager.AddToStack(nameErrorLabel, LayoutConstants.SpacingSmall);

        var outputLabel = new Label
        {
            Text = "Output:",
            Font = new System.Drawing.Font("Segoe UI", 9, System.Drawing.FontStyle.Bold),
            ForeColor = ThemeColors.TextSecondary,
            BackColor = ThemeColors.BackgroundDark,
            AutoSize = true
        };
        layoutManager.AddToStack(outputLabel, LayoutConstants.SpacingSmall);

        outputLog = new RichTextBox
        {
            ReadOnly = true,
            BackColor = ThemeColors.BackgroundMedium,
            ForeColor = ThemeColors.TextPrimary,
            Font = new System.Drawing.Font("Consolas", 9),
            BorderStyle = BorderStyle.FixedSingle,
            Height = 200,
            Width = layoutManager.GetUsableWidth()
        };
        layoutManager.AddToStack(outputLog, LayoutConstants.SpacingLarge);

        layoutManager.AddFlexibleSpacer();

        addButton = layoutManager.AddButton("Add Module", ThemeColors.ButtonSuccess);
        addButton.Click += AddButton_Click;

        cancelButton = layoutManager.AddButton("Cancel", ThemeColors.ButtonSecondary);
        cancelButton.Click += (s, e) => ParentForm?.Close();

        layoutManager.RefreshLayout();
    }

    private void BrowseButton_Click(object sender, EventArgs e)
    {
        using var dialog = new FolderBrowserDialog
        {
            Description = "Select project directory",
            SelectedPath = projectPathInput.Text
        };
        if (dialog.ShowDialog() == DialogResult.OK)
            projectPathInput.Text = dialog.SelectedPath;
    }

    private void ValidateName()
    {
        var name = moduleNameInput.Text.Trim();
        if (string.IsNullOrEmpty(name))
        {
            nameErrorLabel.Text = "";
            return;
        }
        nameErrorLabel.Text = Regex.IsMatch(name, @"^[a-z][a-z0-9_]*$")
            ? ""
            : "Must be snake_case: lowercase letters, digits, underscores, no leading digit";
    }

    private void AddButton_Click(object sender, EventArgs e)
    {
        var projectPath = projectPathInput.Text.Trim();
        var moduleName = moduleNameInput.Text.Trim();

        if (string.IsNullOrWhiteSpace(projectPath) || !Directory.Exists(projectPath))
        {
            MessageBox.Show("Project directory does not exist.", "Validation Error",
                MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }
        if (!File.Exists(Path.Combine(projectPath, "CMakeLists.txt")))
        {
            MessageBox.Show("Not a MayaFlux project (no CMakeLists.txt found).", "Validation Error",
                MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }
        if (string.IsNullOrWhiteSpace(moduleName) || !Regex.IsMatch(moduleName, @"^[a-z][a-z0-9_]*$"))
        {
            MessageBox.Show("Please enter a valid snake_case module name.", "Validation Error",
                MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }

        SetFormEnabled(false);
        outputLog.Clear();

        Task.Run(() => AddModuleAsync(projectPath, moduleName));
    }

    private async Task AddModuleAsync(string projectPath, string moduleName)
    {
        try
        {
            Log("Fetching registry...");
            string registryJson;
            using (var http = new HttpClient())
            {
                http.Timeout = TimeSpan.FromSeconds(30);
                registryJson = await http.GetStringAsync(RegistryUrl);
            }

            Log($"Looking up module: {moduleName}");
            using var doc = JsonDocument.Parse(registryJson);
            string? repo = null;
            string? minVersion = null;

            foreach (var entry in doc.RootElement.EnumerateArray())
            {
                if (entry.GetProperty("name").GetString() == moduleName)
                {
                    repo = entry.GetProperty("repo").GetString();
                    minVersion = entry.TryGetProperty("min_version", out var mv)
                        ? mv.GetString() : null;
                    break;
                }
            }

            if (repo == null)
            {
                Log($"[ERROR] Module '{moduleName}' not found in registry.");
                Invoke(() => SetFormEnabled(true));
                return;
            }

            if (minVersion != null)
            {
                var mfRoot = Environment.GetEnvironmentVariable("MAYAFLUX_ROOT") ?? @"C:\MayaFlux";
                var versionFile = Path.Combine(mfRoot, "lib", "cmake", "MayaFlux", "MayaFluxConfigVersion.cmake");
                if (File.Exists(versionFile))
                {
                    var content = await File.ReadAllTextAsync(versionFile);
                    var m = Regex.Match(content, @"set\(PACKAGE_VERSION\s+""([^""]+)""");
                    if (m.Success && !VersionGte(m.Groups[1].Value, minVersion))
                    {
                        Log($"[ERROR] Module '{moduleName}' requires MayaFlux >= {minVersion}, found {m.Groups[1].Value}.");
                        Invoke(() => SetFormEnabled(true));
                        return;
                    }
                }
            }

            var communityDir = Path.Combine(projectPath, "community");
            Directory.CreateDirectory(communityDir);

            var moduleDir = Path.Combine(communityDir, moduleName);
            if (Directory.Exists(moduleDir))
            {
                Log($"  {moduleName} already present, skipping clone");
            }
            else
            {
                Log($"  Cloning {repo}...");
                var cloneResult = RunGitCapture(communityDir, "clone", "--depth=1", repo, moduleName);
                if (cloneResult != 0)
                {
                    Log($"[ERROR] git clone failed (exit code {cloneResult}).");
                    Invoke(() => SetFormEnabled(true));
                    return;
                }
                Log($"  ✓ Cloned into community/{moduleName}");
            }

            if (!File.Exists(Path.Combine(moduleDir, $"{moduleName}.cmake")))
            {
                Log($"[ERROR] Module '{moduleName}' is missing {moduleName}.cmake");
                Invoke(() => SetFormEnabled(true));
                return;
            }
            if (!Directory.Exists(Path.Combine(moduleDir, "src")))
            {
                Log($"[ERROR] Module '{moduleName}' is missing src/");
                Invoke(() => SetFormEnabled(true));
                return;
            }

            var communityCmake = Path.Combine(projectPath, "community.cmake");
            var existing = File.Exists(communityCmake)
                ? await File.ReadAllTextAsync(communityCmake)
                : "";
            var lines = existing.Replace("\r\n", "\n").Split('\n',
                StringSplitOptions.RemoveEmptyEntries);
            bool already = Array.Exists(lines, l => l.Trim() == moduleName);
            if (!already)
            {
                await File.AppendAllTextAsync(communityCmake, moduleName + "\n");
                Log($"  ✓ Added {moduleName} to community.cmake");
            }
            else
            {
                Log($"  ✓ {moduleName} already in community.cmake");
            }

            Log("");
            Log("Done. Rebuild your project to include the new module.");

            Invoke(() =>
            {
                MessageBox.Show(
                    $"Module '{moduleName}' added.\n\nRebuild your project to include it.",
                    "Module Added", MessageBoxButtons.OK, MessageBoxIcon.Information);
                ParentForm?.Close();
            });
        }
        catch (Exception ex)
        {
            Log($"[ERROR] {ex.Message}");
            Invoke(() => SetFormEnabled(true));
        }
    }

    private static bool VersionGte(string a, string b)
    {
        var pa = a.Split('.');
        var pb = b.Split('.');
        for (int i = 0; i < 3; i++)
        {
            int av = i < pa.Length && int.TryParse(pa[i], out int x) ? x : 0;
            int bv = i < pb.Length && int.TryParse(pb[i], out int y) ? y : 0;
            if (av > bv) return true;
            if (av < bv) return false;
        }
        return true;
    }

    private int RunGitCapture(string workingDir, params string[] args)
    {
        var psi = new ProcessStartInfo("git")
        {
            WorkingDirectory = workingDir,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true,
        };
        foreach (var a in args) psi.ArgumentList.Add(a);
        using var p = Process.Start(psi)!;
        p.WaitForExit();
        return p.ExitCode;
    }

    private void SetFormEnabled(bool enabled)
    {
        addButton.Enabled = enabled;
        cancelButton.Enabled = enabled;
        projectPathInput.Enabled = enabled;
        moduleNameInput.Enabled = enabled;
        browseButton.Enabled = enabled;
    }

    private void Log(string message)
    {
        if (outputLog.InvokeRequired)
        {
            outputLog.Invoke(new Action(() => Log(message)));
            return;
        }
        outputLog.AppendText(message + "\n");
        outputLog.ScrollToCaret();
    }
}