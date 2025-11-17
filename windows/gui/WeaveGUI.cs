using System;
using System.Diagnostics;
using System.IO;
using System.Windows.Forms;
using System.Drawing;  // Add this for SystemIcons

namespace Weave
{
    /// <summary>
    /// Weave Project Creator GUI
    /// Windows equivalent of WeaveGUI.swift
    /// </summary>
    public partial class WeaveForm : Form
    {
        private TextBox projectNameInput;
        private TextBox projectPathInput;
        private Button browseButton;
        private Button createButton;
        private Button cancelButton;
        private RichTextBox outputLog;
        private CheckBox withLilaCheckbox;
        private CheckBox withVscodeCheckbox;

        public WeaveForm()
        {
            InitializeComponent();
        }

        private void InitializeComponent()
        {
            // ============================================================================
            // FORM SETUP
            // ============================================================================
            this.Text = "Weave - MayaFlux Project Creator";
            this.Width = 600;
            this.Height = 500;
            this.StartPosition = FormStartPosition.CenterScreen;
            this.Font = new System.Drawing.Font("Segoe UI", 9);
            this.FormBorderStyle = FormBorderStyle.FixedDialog;
            this.MaximizeBox = false;
            this.MinimizeBox = true;
            this.Icon = SystemIcons.Application;

            // ============================================================================
            // TITLE LABEL
            // ============================================================================
            Label titleLabel = new Label
            {
                Text = "Create a New MayaFlux Project",
                Font = new System.Drawing.Font("Segoe UI", 14, System.Drawing.FontStyle.Bold),
                Location = new System.Drawing.Point(20, 20),
                AutoSize = true,
                ForeColor = System.Drawing.Color.FromArgb(29, 29, 31)
            };
            this.Controls.Add(titleLabel);

            // ============================================================================
            // PROJECT NAME
            // ============================================================================
            Label nameLabel = new Label
            {
                Text = "Project Name:",
                Location = new System.Drawing.Point(20, 60),
                AutoSize = true,
                ForeColor = System.Drawing.Color.FromArgb(110, 110, 115)
            };
            this.Controls.Add(nameLabel);

            projectNameInput = new TextBox
            {
                Location = new System.Drawing.Point(20, 85),
                Width = 540,
                Height = 30,
                Text = "MyProject",
                Font = new System.Drawing.Font("Segoe UI", 10)
            };
            this.Controls.Add(projectNameInput);

            // ============================================================================
            // PROJECT PATH
            // ============================================================================
            Label pathLabel = new Label
            {
                Text = "Project Location:",
                Location = new System.Drawing.Point(20, 130),
                AutoSize = true,
                ForeColor = System.Drawing.Color.FromArgb(110, 110, 115)
            };
            this.Controls.Add(pathLabel);

            projectPathInput = new TextBox
            {
                Location = new System.Drawing.Point(20, 155),
                Width = 450,
                Height = 30,
                Text = Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments),
                Font = new System.Drawing.Font("Segoe UI", 10)
            };
            this.Controls.Add(projectPathInput);

            browseButton = new Button
            {
                Text = "Browse...",
                Location = new System.Drawing.Point(475, 155),
                Width = 85,
                Height = 30,
                BackColor = System.Drawing.Color.FromArgb(0, 120, 215),
                ForeColor = System.Drawing.Color.White,
                FlatStyle = FlatStyle.Flat,
                Font = new System.Drawing.Font("Segoe UI", 9)
            };
            browseButton.Click += BrowseButton_Click;
            this.Controls.Add(browseButton);

            // ============================================================================
            // OPTIONS
            // ============================================================================
            withLilaCheckbox = new CheckBox
            {
                Text = "Enable Live Coding (Lila)",
                Location = new System.Drawing.Point(20, 200),
                AutoSize = true,
                Checked = false
            };
            this.Controls.Add(withLilaCheckbox);

            withVscodeCheckbox = new CheckBox
            {
                Text = "Configure for VS Code",
                Location = new System.Drawing.Point(20, 225),
                AutoSize = true,
                Checked = true
            };
            this.Controls.Add(withVscodeCheckbox);

            // ============================================================================
            // OUTPUT LOG
            // ============================================================================
            Label outputLabel = new Label
            {
                Text = "Output:",
                Location = new System.Drawing.Point(20, 260),
                AutoSize = true,
                ForeColor = System.Drawing.Color.FromArgb(110, 110, 115),
                Font = new System.Drawing.Font("Segoe UI", 9, System.Drawing.FontStyle.Bold)
            };
            this.Controls.Add(outputLabel);

            outputLog = new RichTextBox
            {
                Location = new System.Drawing.Point(20, 280),
                Width = 540,
                Height = 120,
                ReadOnly = true,
                BackColor = System.Drawing.Color.FromArgb(242, 242, 242),
                Font = new System.Drawing.Font("Consolas", 9),
                BorderStyle = BorderStyle.Fixed3D
            };
            this.Controls.Add(outputLog);

            // ============================================================================
            // BUTTONS
            // ============================================================================
            createButton = new Button
            {
                Text = "Create Project",
                Location = new System.Drawing.Point(380, 420),
                Width = 180,
                Height = 40,
                BackColor = System.Drawing.Color.FromArgb(16, 124, 16),
                ForeColor = System.Drawing.Color.White,
                FlatStyle = FlatStyle.Flat,
                Font = new System.Drawing.Font("Segoe UI", 10, System.Drawing.FontStyle.Bold)
            };
            createButton.Click += CreateButton_Click;
            this.Controls.Add(createButton);

            cancelButton = new Button
            {
                Text = "Cancel",
                Location = new System.Drawing.Point(20, 420),
                Width = 100,
                Height = 40,
                BackColor = System.Drawing.Color.FromArgb(200, 200, 200),
                ForeColor = System.Drawing.Color.Black,
                FlatStyle = FlatStyle.Flat,
                Font = new System.Drawing.Font("Segoe UI", 10)
            };
            cancelButton.Click += (s, e) => this.Close();
            this.Controls.Add(cancelButton);
        }

        private void BrowseButton_Click(object sender, EventArgs e)
        {
            using (FolderBrowserDialog dialog = new FolderBrowserDialog())
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
            // ============================================================================
            // VALIDATION
            // ============================================================================
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

            // ============================================================================
            // DISABLE CONTROLS
            // ============================================================================
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
                this.Close();
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
            string projectDir = Path.Combine(projectPath, projectName);
            string srcDir = Path.Combine(projectDir, "src");
            string vsCodeDir = Path.Combine(projectDir, ".vscode");

            // ============================================================================
            // CREATE DIRECTORIES
            // ============================================================================
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

            // ============================================================================
            // DETERMINE MAYAFLUX ROOT
            // ============================================================================
            string mayaFluxRoot = Environment.GetEnvironmentVariable("MAYAFLUX_ROOT");
            if (string.IsNullOrEmpty(mayaFluxRoot))
            {
                mayaFluxRoot = "C:\\MayaFlux";
            }

            string mayaFluxCmakePath = Path.Combine(mayaFluxRoot, "lib", "cmake", "MayaFlux");
            Log($"  ↳ Using MAYAFLUX_ROOT: {mayaFluxRoot}");

            // ============================================================================
            // GENERATE CMakeLists.txt
            // ============================================================================
            Log("Generating CMakeLists.txt");

            string lilaBlock = withLilaCheckbox.Checked
                ? @"if(TARGET MayaFlux::Lila)
    target_link_libraries(${PROJECT_NAME} PRIVATE MayaFlux::Lila)
    message(STATUS ""Lila live coding enabled"")
else()
    message(WARNING ""Lila not found - live coding disabled"")
endif()"
                : @"# Lila live coding not enabled";

            string cmakelists = GenerateCMakeLists(projectName, mayaFluxCmakePath, lilaBlock);
            File.WriteAllText(Path.Combine(projectDir, "CMakeLists.txt"), cmakelists);
            Log("  ✓ Generated CMakeLists.txt");

            // ============================================================================
            // GENERATE SOURCE FILES
            // ============================================================================
            Log("Generating source files");

            string mainCpp = GenerateMainCpp();
            File.WriteAllText(Path.Combine(srcDir, "main.cpp"), mainCpp);
            Log("  ✓ Generated main.cpp");

            string userProject = GenerateUserProjectHpp();
            File.WriteAllText(Path.Combine(srcDir, "user_project.hpp"), userProject);
            Log("  ✓ Generated user_project.hpp");

            // ============================================================================
            // GENERATE VS CODE CONFIG (if enabled)
            // ============================================================================
            if (withVscodeCheckbox.Checked)
            {
                Log("Generating VS Code configuration");

                string settings = GenerateVsCodeSettings();
                File.WriteAllText(Path.Combine(vsCodeDir, "settings.json"), settings);
                Log("  ✓ Generated settings.json");

                string tasks = GenerateVsCodeTasks(projectName);
                File.WriteAllText(Path.Combine(vsCodeDir, "tasks.json"), tasks);
                Log("  ✓ Generated tasks.json");

                string launch = GenerateVsCodeLaunch(projectName);
                File.WriteAllText(Path.Combine(vsCodeDir, "launch.json"), launch);
                Log("  ✓ Generated launch.json");
            }

            // ============================================================================
            // GENERATE README
            // ============================================================================
            Log("Generating README.md");

            string readme = $@"# {projectName}

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

        private string LoadTemplate(string templateName)
        {
            // Templates are installed by the installer
            // Locations: C:\MayaFlux\share\weave\templates\ (installed)
            //           or ./templates/ (during development)
            
            string[] possiblePaths = new string[]
            {
                Path.Combine(Environment.GetEnvironmentVariable("MAYAFLUX_ROOT") ?? "C:\\MayaFlux", 
                    "share", "weave", "templates", templateName),
                Path.Combine("templates", templateName)
            };

            foreach (string path in possiblePaths)
            {
                if (File.Exists(path))
                {
                    return File.ReadAllText(path);
                }
            }

            throw new FileNotFoundException($"Template not found: {templateName}. Searched in: {string.Join(", ", possiblePaths)}");
        }

        private string GenerateCMakeLists(string projectName, string mayaFluxCmakePath, string lilaBlock)
        {
            string template = LoadTemplate("CMakeLists.txt");
            template = template.Replace("@PROJECT_NAME@", projectName);
            template = template.Replace("@MAYAFLUX_CMAKE_PATH@", mayaFluxCmakePath);
            template = template.Replace("@LILA_LINK_BLOCK@", lilaBlock);
            template = template.Replace("@LILA_DLL_COPY@", "# Lila DLL copy not needed in GUI");
            return template;
        }

        private string GenerateMainCpp()
        {
            return LoadTemplate("main.cpp.in");
        }

        private string GenerateUserProjectHpp()
        {
            return LoadTemplate("user_project.hpp.in");
        }

        private string GenerateVsCodeSettings()
        {
            return LoadTemplate("vscode\\settings.json.in");
        }

        private string GenerateVsCodeTasks(string projectName)
        {
            string template = LoadTemplate("vscode\\tasks.json.in");
            template = template.Replace("@PROJECT_NAME@", projectName);
            return template;
        }

        private string GenerateVsCodeLaunch(string projectName)
        {
            string template = LoadTemplate("vscode\\launch.json.in");
            template = template.Replace("@PROJECT_NAME@", projectName);
            return template;
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

        [STAThread]
        static void Main()
        {
            Application.EnableVisualStyles();
            Application.Run(new WeaveForm());
        }
    }
}