using System;
using System.Drawing;
using System.IO;
using System.Windows.Forms;
using Weave.Theme;
using Weave.UI.Layout;

namespace Weave.UI.Project;

public partial class ProjectCreatorView : UserControl
{
    private TextBox projectNameInput;
    private TextBox projectPathInput;
    private Button browseButton;
    private Button createButton;
    private Button cancelButton;
    private RichTextBox outputLog;
    private CheckBox withLilaCheckbox;
    private CheckBox withVscodeCheckbox;
    private string templatesDir;
    private LayoutManager layoutManager;

    public ProjectCreatorView(string templatesDirectory)
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

        layoutManager.AddTitle("Create a New MayaFlux Project");

        var nameInput = layoutManager.AddLabeledInput("Project Name:", "MyProject");
        projectNameInput = nameInput;

        var pathInput = layoutManager.AddLabeledInput("Project Location:",
            Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments));
        projectPathInput = pathInput;

        browseButton = layoutManager.AddButton("Browse...", ThemeColors.ButtonSecondary);
        browseButton.Click += BrowseButton_Click;

        withLilaCheckbox = layoutManager.AddCheckbox("Enable Live Coding (Lila)");
        withVscodeCheckbox = layoutManager.AddCheckbox("Configure for VS Code");
        withVscodeCheckbox.Checked = true;

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
            Height = 200,
            Width = layoutManager.GetUsableWidth()
        };
        layoutManager.AddToStack(outputLog, LayoutConstants.SpacingLarge);

        layoutManager.AddFlexibleSpacer();

        createButton = layoutManager.AddButton("Create Project", ThemeColors.ButtonSuccess);
        createButton.Click += CreateButton_Click;

        cancelButton = layoutManager.AddButton("Cancel", ThemeColors.ButtonSecondary);
        cancelButton.Click += (s, e) => ParentForm?.Close();

        layoutManager.RefreshLayout();
    }

    private void BrowseButton_Click(object sender, EventArgs e)
    {
        using (var dialog = new FolderBrowserDialog())
        {
            dialog.Description = "Select project location";
            dialog.SelectedPath = projectPathInput.Text;

            if (dialog.ShowDialog() == DialogResult.OK)
            {
                projectPathInput.Text = dialog.SelectedPath;
            }
        }
    }

    private void CreateButton_Click(object sender, EventArgs e)
    {
        string projectName = projectNameInput.Text.Trim();
        string projectPath = projectPathInput.Text.Trim();

        if (string.IsNullOrWhiteSpace(projectName))
        {
            MessageBox.Show("Please enter a project name.", "Validation Error", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }

        if (string.IsNullOrWhiteSpace(projectPath))
        {
            MessageBox.Show("Please select a project location.", "Validation Error", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }

        if (!Directory.Exists(projectPath))
        {
            MessageBox.Show($"Directory does not exist: {projectPath}", "Path Error", MessageBoxButtons.OK, MessageBoxIcon.Error);
            return;
        }

        createButton.Enabled = false;
        cancelButton.Enabled = false;
        projectNameInput.Enabled = false;
        projectPathInput.Enabled = false;
        browseButton.Enabled = false;

        outputLog.Clear();
        Log("Creating MayaFlux project...\n");

        try
        {
            CreateProject(projectName, projectPath);
            Log("\nProject created successfully!");
            MessageBox.Show($"Project '{projectName}' created at:\n{projectPath}", "Success", MessageBoxButtons.OK, MessageBoxIcon.Information);
            ParentForm?.Close();
        }
        catch (Exception ex)
        {
            Log($"\nError: {ex.Message}");
            MessageBox.Show($"Failed to create project:\n{ex.Message}", "Error", MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
        finally
        {
            createButton.Enabled = true;
            cancelButton.Enabled = true;
            projectNameInput.Enabled = true;
            projectPathInput.Enabled = true;
            browseButton.Enabled = true;
        }
    }

    private void CreateProject(string projectName, string projectPath)
    {
        var projectDir = Path.Combine(projectPath, projectName);
        var srcDir = Path.Combine(projectDir, "src");
        var vsCodeDir = Path.Combine(projectDir, ".vscode");

        Log($"Creating project structure at: {projectDir}");

        if (Directory.Exists(projectDir))
        {
            throw new Exception($"Project directory already exists: {projectDir}");
        }

        Directory.CreateDirectory(srcDir);
        Log($"  Created src directory");

        if (withVscodeCheckbox.Checked)
        {
            Directory.CreateDirectory(vsCodeDir);
            Log($"  Created .vscode directory");
        }

        var mayaFluxRoot = Environment.GetEnvironmentVariable("MAYAFLUX_ROOT") ?? "C:\\MayaFlux";
        var mayaFluxCmakePath = Path.Combine(mayaFluxRoot, "lib", "cmake", "MayaFlux");
        Log($"  Using MAYAFLUX_ROOT: {mayaFluxRoot}");

        Log("Generating CMakeLists.txt");
        var lilaBlock = withLilaCheckbox.Checked
            ? @"if(TARGET MayaFlux::Lila)
    target_link_libraries(${PROJECT_NAME} PRIVATE MayaFlux::Lila)
    message(STATUS ""Lila live coding enabled"")
else()
    message(WARNING ""Lila not found - live coding disabled"")
endif()"
            : @"# Lila live coding not enabled";

        var cmakelists = GenerateCMakeLists(projectName, mayaFluxCmakePath, lilaBlock);
        File.WriteAllText(Path.Combine(projectDir, "CMakeLists.txt"), cmakelists);
        Log("  Generated CMakeLists.txt");

        Log("Copying shaders.cmake");
        var shadersCmakeSrc = Path.Combine(templatesDir, "shaders.cmake");
        if (File.Exists(shadersCmakeSrc))
        {
            File.Copy(shadersCmakeSrc, Path.Combine(projectDir, "shaders.cmake"));
            Log("  Copied shaders.cmake");
        }
        else
        {
            Log("  WARNING: shaders.cmake template not found, skipping");
        }

        var dataShadersDir = Path.Combine(projectDir, "data", "shaders");
        Directory.CreateDirectory(dataShadersDir);
        Log("  Created data/shaders");

        var templateShadersDir = Path.Combine(templatesDir, "shaders");
        if (Directory.Exists(templateShadersDir))
        {
            foreach (var shader in Directory.GetFiles(templateShadersDir))
            {
                File.Copy(shader, Path.Combine(dataShadersDir, Path.GetFileName(shader)));
                Log($"  Copied {Path.GetFileName(shader)}");
            }
        }

        Log("Generating source files");
        var mainCpp = LoadTemplate("main.cpp");
        File.WriteAllText(Path.Combine(srcDir, "main.cpp"), mainCpp);
        Log("  Generated main.cpp");

        var userProject = LoadTemplate("user_project.hpp");
        File.WriteAllText(Path.Combine(srcDir, "user_project.hpp"), userProject);
        Log("  Generated user_project.hpp");

        if (withVscodeCheckbox.Checked)
        {
            Log("Generating VS Code configuration");

            var settings = LoadTemplate("vscode/settings.json");
            File.WriteAllText(Path.Combine(vsCodeDir, "settings.json"), settings);
            Log("  Generated settings.json");

            var tasks = LoadTemplate("vscode/tasks.json").Replace("@PROJECT_NAME@", projectName);
            File.WriteAllText(Path.Combine(vsCodeDir, "tasks.json"), tasks);
            Log("  Generated tasks.json");

            var launch = LoadTemplate("vscode/launch.json").Replace("@PROJECT_NAME@", projectName);
            File.WriteAllText(Path.Combine(vsCodeDir, "launch.json"), launch);
            Log("  Generated launch.json");
        }

        Log("Generating README.md");
        var readme = $@"# {projectName}

A MayaFlux multimedia DSP project.

## Building

```bash
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release --parallel
```

## Running

```bash
.\build\Release\{projectName}.exe
```

## Editing

Open in VS Code:
```bash
code .
```

Edit your code in `src/user_project.hpp`:
- `settings()`: Configure sample rate, buffer size, graphics
- `compose()`: Create your nodes, buffers, and processing chains

## Documentation

See [MayaFlux Documentation](https://github.com/MayaFlux/MayaFlux)
";

        File.WriteAllText(Path.Combine(projectDir, "README.md"), readme);
        Log("  Generated README.md");
    }

    private string GenerateCMakeLists(string projectName, string mayaFluxCmakePath, string lilaBlock)
    {
        var template = LoadTemplate("CMakeLists.txt");
        template = template.Replace("@PROJECT_NAME@", projectName);
        template = template.Replace("@MAYAFLUX_CMAKE_PATH@", mayaFluxCmakePath);
        template = template.Replace("@LILA_LINK_BLOCK@", lilaBlock);
        template = template.Replace("@LILA_DLL_COPY@", "");
        return template;
    }

    private string LoadTemplate(string templateName)
    {
        var path = Path.Combine(templatesDir, templateName);
        if (!File.Exists(path))
        {
            throw new FileNotFoundException($"Template not found: {templateName} at {path}");
        }

        return File.ReadAllText(path);
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
