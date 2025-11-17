using System;
using System.IO;
using System.Windows.Forms;
using System.Drawing;

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

    public ProjectCreatorView(string templatesDirectory)
    {
        templatesDir = templatesDirectory;
        InitializeUI();
    }

    private void InitializeUI()
    {
        BackColor = Color.White;
        Padding = new Padding(20);

        // ============================================================================
        // TITLE LABEL
        // ============================================================================
        var titleLabel = new Label
        {
            Text = "Create a New MayaFlux Project",
            Font = new Font("Segoe UI", 14, FontStyle.Bold),
            Location = new Point(0, 0),
            AutoSize = true,
            ForeColor = Color.FromArgb(29, 29, 31)
        };
        Controls.Add(titleLabel);

        // ============================================================================
        // PROJECT NAME
        // ============================================================================
        var nameLabel = new Label
        {
            Text = "Project Name:",
            Location = new Point(0, 40),
            AutoSize = true,
            ForeColor = Color.FromArgb(110, 110, 115)
        };
        Controls.Add(nameLabel);

        projectNameInput = new TextBox
        {
            Location = new Point(0, 60),
            Width = Width - 40,
            Height = 30,
            Text = "MyProject",
            Font = new Font("Segoe UI", 10)
        };
        Controls.Add(projectNameInput);

        // ============================================================================
        // PROJECT PATH
        // ============================================================================
        var pathLabel = new Label
        {
            Text = "Project Location:",
            Location = new Point(0, 100),
            AutoSize = true,
            ForeColor = Color.FromArgb(110, 110, 115)
        };
        Controls.Add(pathLabel);

        projectPathInput = new TextBox
        {
            Location = new Point(0, 120),
            Width = Width - 110,
            Height = 30,
            Text = Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments),
            Font = new Font("Segoe UI", 10)
        };
        Controls.Add(projectPathInput);

        browseButton = new Button
        {
            Text = "Browse...",
            Location = new Point(Width - 100, 120),
            Width = 85,
            Height = 30,
            BackColor = Color.FromArgb(0, 120, 215),
            ForeColor = Color.White,
            FlatStyle = FlatStyle.Flat,
            Font = new Font("Segoe UI", 9)
        };
        browseButton.Click += BrowseButton_Click;
        Controls.Add(browseButton);

        // ============================================================================
        // OPTIONS
        // ============================================================================
        withLilaCheckbox = new CheckBox
        {
            Text = "Enable Live Coding (Lila)",
            Location = new Point(0, 170),
            AutoSize = true,
            Checked = false
        };
        Controls.Add(withLilaCheckbox);

        withVscodeCheckbox = new CheckBox
        {
            Text = "Configure for VS Code",
            Location = new Point(0, 195),
            AutoSize = true,
            Checked = true
        };
        Controls.Add(withVscodeCheckbox);

        // ============================================================================
        // OUTPUT LOG
        // ============================================================================
        var outputLabel = new Label
        {
            Text = "Output:",
            Location = new Point(0, 230),
            AutoSize = true,
            ForeColor = Color.FromArgb(110, 110, 115),
            Font = new Font("Segoe UI", 9, FontStyle.Bold)
        };
        Controls.Add(outputLabel);

        outputLog = new RichTextBox
        {
            Location = new Point(0, 250),
            Width = Width - 40,
            Height = Height - 350,
            ReadOnly = true,
            BackColor = Color.FromArgb(242, 242, 242),
            Font = new Font("Consolas", 9),
            BorderStyle = BorderStyle.Fixed3D
        };
        Controls.Add(outputLog);

        // ============================================================================
        // BUTTONS
        // ============================================================================
        createButton = new Button
        {
            Text = "Create Project",
            Location = new Point(Width - 200, Height - 50),
            Width = 180,
            Height = 40,
            BackColor = Color.FromArgb(16, 124, 16),
            ForeColor = Color.White,
            FlatStyle = FlatStyle.Flat,
            Font = new Font("Segoe UI", 10, FontStyle.Bold)
        };
        createButton.Click += CreateButton_Click;
        Controls.Add(createButton);

        cancelButton = new Button
        {
            Text = "Cancel",
            Location = new Point(0, Height - 50),
            Width = 100,
            Height = 40,
            BackColor = Color.FromArgb(200, 200, 200),
            ForeColor = Color.Black,
            FlatStyle = FlatStyle.Flat,
            Font = new Font("Segoe UI", 10)
        };
        cancelButton.Click += (s, e) => ParentForm?.Close();
        Controls.Add(cancelButton);
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
            Log("\n✓ Project created successfully!");
            MessageBox.Show($"Project '{projectName}' created at:\n{projectPath}", "Success", MessageBoxButtons.OK, MessageBoxIcon.Information);
            ParentForm?.Close();
        }
        catch (Exception ex)
        {
            Log($"\n✗ Error: {ex.Message}");
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
        Log($"  ✓ Created src directory");

        if (withVscodeCheckbox.Checked)
        {
            Directory.CreateDirectory(vsCodeDir);
            Log($"  ✓ Created .vscode directory");
        }

        var mayaFluxRoot = Environment.GetEnvironmentVariable("MAYAFLUX_ROOT") ?? "C:\\MayaFlux";
        var mayaFluxCmakePath = Path.Combine(mayaFluxRoot, "lib", "cmake", "MayaFlux");
        Log($"  ↳ Using MAYAFLUX_ROOT: {mayaFluxRoot}");

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
        Log("  ✓ Generated CMakeLists.txt");

        Log("Generating source files");
        var mainCpp = LoadTemplate("main.cpp.in");
        File.WriteAllText(Path.Combine(srcDir, "main.cpp"), mainCpp);
        Log("  ✓ Generated main.cpp");

        var userProject = LoadTemplate("user_project.hpp.in");
        File.WriteAllText(Path.Combine(srcDir, "user_project.hpp"), userProject);
        Log("  ✓ Generated user_project.hpp");

        if (withVscodeCheckbox.Checked)
        {
            Log("Generating VS Code configuration");

            var settings = LoadTemplate("vscode/settings.json.in");
            File.WriteAllText(Path.Combine(vsCodeDir, "settings.json"), settings);
            Log("  ✓ Generated settings.json");

            var tasks = LoadTemplate("vscode/tasks.json.in").Replace("@PROJECT_NAME@", projectName);
            File.WriteAllText(Path.Combine(vsCodeDir, "tasks.json"), tasks);
            Log("  ✓ Generated tasks.json");

            var launch = LoadTemplate("vscode/launch.json.in").Replace("@PROJECT_NAME@", projectName);
            File.WriteAllText(Path.Combine(vsCodeDir, "launch.json"), launch);
            Log("  ✓ Generated launch.json");
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
        Log("  ✓ Generated README.md");
    }

    private string GenerateCMakeLists(string projectName, string mayaFluxCmakePath, string lilaBlock)
    {
        var template = LoadTemplate("CMakeLists.txt");
        template = template.Replace("@PROJECT_NAME@", projectName);
        template = template.Replace("@MAYAFLUX_CMAKE_PATH@", mayaFluxCmakePath);
        template = template.Replace("@LILA_LINK_BLOCK@", lilaBlock);
        template = template.Replace("@LILA_DLL_COPY@", "# Lila DLL copy not needed in GUI");
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
