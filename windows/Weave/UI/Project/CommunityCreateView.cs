using System;
using System.IO;
using System.Text.RegularExpressions;
using System.Windows.Forms;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using Weave.Theme;
using Weave.UI.Layout;

namespace Weave.UI.Project;

public partial class CommunityCreatorView : UserControl
{
    private TextBox moduleNameInput;
    private TextBox descriptionInput;
    private TextBox minVersionInput;
    private TextBox modulePathInput;
    private CheckBox needsLilaCheckbox;
    private Button browseButton;
    private Button createButton;
    private Button cancelButton;
    private RichTextBox outputLog;
    private Label nameErrorLabel;
    private string templatesDir;
    private LayoutManager layoutManager;

    public CommunityCreatorView(string templatesDirectory)
    {
        templatesDir = templatesDirectory;
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

        layoutManager.AddTitle("Create a Community Module");

        var nameInput = layoutManager.AddLabeledInput("Module Name (snake_case):", "my_module");
        moduleNameInput = nameInput;
        moduleNameInput.TextChanged += (s, e) => ValidateName();

        nameErrorLabel = new Label
        {
            Text = "",
            Font = new Font("Segoe UI", 9),
            ForeColor = ThemeColors.Error,
            BackColor = ThemeColors.BackgroundDark,
            AutoSize = true
        };
        layoutManager.AddToStack(nameErrorLabel, 2);

        var descInput = layoutManager.AddLabeledInput("Description:", "");
        descriptionInput = descInput;
        descriptionInput.PlaceholderText = "A short description of what this module does";

        var verInput = layoutManager.AddLabeledInput("Minimum MayaFlux Version:", "0.4.0");
        minVersionInput = verInput;

        needsLilaCheckbox = layoutManager.AddCheckbox("Requires Lila (live coding)");

        var pathInput = layoutManager.AddLabeledInput("Destination:",
            Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments));
        modulePathInput = pathInput;
        modulePathInput.TextChanged += (s, e) => UpdatePreview();
        moduleNameInput.TextChanged += (s, e) => UpdatePreview();

        browseButton = layoutManager.AddButton("Browse...", ThemeColors.ButtonSecondary);
        browseButton.Click += BrowseButton_Click;

        var previewLabel = new Label
        {
            Text = "",
            Font = new Font("Consolas", 9),
            ForeColor = ThemeColors.TextSecondary,
            BackColor = ThemeColors.BackgroundDark,
            AutoSize = true
        };
        layoutManager.AddToStack(previewLabel, LayoutConstants.SpacingSmall);
        _previewLabel = previewLabel;
        UpdatePreview();

        var outputLabel = new Label
        {
            Text = "Output:",
            Font = new Font("Segoe UI", 9, FontStyle.Bold),
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
            Font = new Font("Consolas", 9),
            BorderStyle = BorderStyle.FixedSingle,
            Height = 160,
            Width = layoutManager.GetUsableWidth()
        };
        layoutManager.AddToStack(outputLog, LayoutConstants.SpacingLarge);

        layoutManager.AddFlexibleSpacer();

        createButton = layoutManager.AddButton("Create Module", ThemeColors.ButtonSuccess);
        createButton.Click += CreateButton_Click;

        cancelButton = layoutManager.AddButton("Cancel", ThemeColors.ButtonSecondary);
        cancelButton.Click += (s, e) => ParentForm?.Close();

        layoutManager.RefreshLayout();
    }

    private Label _previewLabel;

    private void UpdatePreview()
    {
        var name = moduleNameInput.Text.Trim();
        var dest = modulePathInput.Text.Trim();
        var safeName = string.IsNullOrEmpty(name) ? "my_module" : name;
        _previewLabel.Text = Path.Combine(dest, safeName);
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

    private void BrowseButton_Click(object sender, EventArgs e)
    {
        using var dialog = new FolderBrowserDialog
        {
            Description = "Select destination directory",
            SelectedPath = modulePathInput.Text
        };
        if (dialog.ShowDialog() == DialogResult.OK)
            modulePathInput.Text = dialog.SelectedPath;
    }

    private void CreateButton_Click(object sender, EventArgs e)
    {
        var moduleName = moduleNameInput.Text.Trim();
        var destPath = modulePathInput.Text.Trim();
        var description = descriptionInput.Text.Trim();
        var minVersion = minVersionInput.Text.Trim();
        var needsLila = needsLilaCheckbox.Checked;

        if (string.IsNullOrWhiteSpace(moduleName))
        {
            MessageBox.Show("Please enter a module name.", "Validation Error",
                MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }
        if (!Regex.IsMatch(moduleName, @"^[a-z][a-z0-9_]*$"))
        {
            MessageBox.Show("Module name must be snake_case (lowercase letters, digits, underscores, no leading digit).",
                "Validation Error", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }
        if (string.IsNullOrWhiteSpace(destPath) || !Directory.Exists(destPath))
        {
            MessageBox.Show($"Destination directory does not exist:\n{destPath}", "Path Error",
                MessageBoxButtons.OK, MessageBoxIcon.Error);
            return;
        }

        SetFormEnabled(false);
        outputLog.Clear();
        Log("Creating community module...\n");

        try
        {
            CreateModule(moduleName, destPath, description, minVersion, needsLila);
            Log("\nModule created successfully!");
            MessageBox.Show($"Community module '{moduleName}' created at:\n{Path.Combine(destPath, moduleName)}",
                "Success", MessageBoxButtons.OK, MessageBoxIcon.Information);
            ParentForm?.Close();
        }
        catch (Exception ex)
        {
            Log($"\nError: {ex.Message}");
            MessageBox.Show($"Failed to create module:\n{ex.Message}", "Error",
                MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
        finally
        {
            SetFormEnabled(true);
        }
    }

    private void CreateModule(string moduleName, string destPath, string description, string minVersion, bool needsLila)
    {
        var moduleDir = Path.Combine(destPath, moduleName);
        var srcDir = Path.Combine(moduleDir, "src");
        var testDir = Path.Combine(moduleDir, "test");

        Log($"Creating module structure at: {moduleDir}");

        if (Directory.Exists(moduleDir))
            throw new Exception($"Directory already exists: {moduleDir}");

        Directory.CreateDirectory(srcDir);
        Log("  Created src/");

        Directory.CreateDirectory(testDir);
        Log("  Created test/");

        var gitignoreSrc = Path.Combine(templatesDir, ".gitignore");
        if (!File.Exists(gitignoreSrc))
            throw new FileNotFoundException("Required template missing: .gitignore");
        File.Copy(gitignoreSrc, Path.Combine(moduleDir, ".gitignore"));
        Log("  Copied .gitignore");

        var moduleCmakeSrc = Path.Combine(templatesDir, "community", "module.cmake");
        if (!File.Exists(moduleCmakeSrc))
            throw new FileNotFoundException("Required template missing: community/module.cmake");
        var moduleCmake = File.ReadAllText(moduleCmakeSrc)
            .Replace("@MODULE_NAME@", moduleName)
            .Replace("set(MF_NEEDS_LILA OFF)", $"set(MF_NEEDS_LILA {(needsLila ? "ON" : "OFF")})");
        File.WriteAllText(Path.Combine(moduleDir, $"{moduleName}.cmake"), moduleCmake);
        Log($"  Generated {moduleName}.cmake");

        var communityJsonSrc = Path.Combine(templatesDir, "community", "community.json");
        if (!File.Exists(communityJsonSrc))
            throw new FileNotFoundException("Required template missing: community/community.json");
        var rawJson = File.ReadAllText(communityJsonSrc).Replace("@MODULE_NAME@", moduleName);
        var json = JObject.Parse(rawJson);
        if (!string.IsNullOrEmpty(description))
            json["description"] = description;
        if (!string.IsNullOrEmpty(minVersion))
            json["min_version"] = minVersion;
        json["needs_lila"] = needsLila;
        File.WriteAllText(Path.Combine(moduleDir, "community.json"), json.ToString(Formatting.Indented).Replace("\r\n", "\n") + "\n"); ;
        Log("  Generated community.json");

        var testCmakeSrc = Path.Combine(templatesDir, "community", "test", "CMakeLists.txt");
        if (!File.Exists(testCmakeSrc))
            throw new FileNotFoundException("Required template missing: community/test/CMakeLists.txt");
        var testCmake = File.ReadAllText(testCmakeSrc).Replace("@MODULE_NAME@", moduleName);
        File.WriteAllText(Path.Combine(testDir, "CMakeLists.txt"), testCmake);
        Log("  Generated test/CMakeLists.txt");

        var testFilePath = Path.Combine(testDir, $"test_{moduleName}.cpp");
        File.WriteAllText(testFilePath, "");
        Log($"  Created test/test_{moduleName}.cpp");

        RunGit(moduleDir, "init", "-b", "main");
        RunGit(moduleDir, "add", "-A");
        Log("  Initialized git repository");

        Log("");
        Log("  src/              <- put your sources here");
        Log("  test/CMakeLists.txt");
        Log($"  test/test_{moduleName}.cpp");
        Log($"  {moduleName}.cmake");
        Log("  community.json");
        Log("");
        Log("Build test:");
        Log($"  cmake -G Ninja -B test/build -S test/");
        Log($"  cmake --build test/build --parallel");
    }

    private void RunGit(string workingDir, params string[] args)
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
    }

    private void SetFormEnabled(bool enabled)
    {
        createButton.Enabled = enabled;
        cancelButton.Enabled = enabled;
        moduleNameInput.Enabled = enabled;
        descriptionInput.Enabled = enabled;
        minVersionInput.Enabled = enabled;
        needsLilaCheckbox.Enabled = enabled;
        modulePathInput.Enabled = enabled;
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
