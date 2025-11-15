using System;
using System.Diagnostics;
using System.IO;
using System.Windows.Forms;

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
            Log($"  → Using MAYAFLUX_ROOT: {mayaFluxRoot}");

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

        private string GenerateCMakeLists(string projectName, string mayaFluxCmakePath, string lilaBlock)
        {
            return $@"cmake_minimum_required(VERSION 3.25)
project({projectName} VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Find MayaFlux
set(MAYAFLUX_SEARCH_PATHS ""{mayaFluxCmakePath}"")
if(DEFINED ENV{{MAYAFLUX_ROOT}})
    list(APPEND MAYAFLUX_SEARCH_PATHS ""$ENV{{MAYAFLUX_ROOT}}/lib/cmake/MayaFlux"")
endif()

find_package(MayaFlux REQUIRED PATHS ${{MAYAFLUX_SEARCH_PATHS}} NO_DEFAULT_PATH)

if(NOT MayaFlux_FOUND)
    message(FATAL_ERROR ""MayaFlux not found! Please set MAYAFLUX_ROOT environment variable."")
endif()

message(STATUS ""Found MayaFlux ${{MayaFlux_VERSION}}: ${{MayaFlux_DIR}}"")

# Create executable
add_executable(${{PROJECT_NAME}}
    src/main.cpp
    src/user_project.hpp
)

# Link MayaFlux
target_link_libraries(${{PROJECT_NAME}} PRIVATE MayaFlux::MayaFluxLib)

# Optional: Lila support
{lilaBlock}

# Platform-specific settings
if(WIN32)
    set_target_properties(${{PROJECT_NAME}} PROPERTIES
        MSVC_RUNTIME_LIBRARY ""MultiThreaded$<$<CONFIG:Debug>:Debug>DLL""
    )
endif()
";
        }

        private string GenerateMainCpp()
        {
            return @"#ifdef __has_include
#if __has_include(""user_project.hpp"")
#include ""user_project.hpp""
#define HAS_USER_PROJECT
#else
#define MAYASIMPLE
#include ""MayaFlux/MayaFlux.hpp""
#endif
#endif

void initialize()
{
#ifdef HAS_USER_PROJECT
    try {
        settings();
    } catch (const std::exception& e) {
        MF_ERROR(MayaFlux::Journal::Component::USER, MayaFlux::Journal::Context::Init, 
                 ""Error during initialization: {}"", e.what());
    }
#endif
}

void run()
{
#ifdef HAS_USER_PROJECT
    try {
        compose();
    } catch (const std::exception& e) {
        MF_ERROR(MayaFlux::Journal::Component::USER, MayaFlux::Journal::Context::Runtime, 
                 ""Error during execution: {}"", e.what());
    }
#endif
}

int main()
{
    try {
        initialize();
        MayaFlux::Init();
        MayaFlux::Start();
        run();
        
        std::cout << ""Press any key to stop...\\n"";
        std::cin.get();
        
        MayaFlux::End();
    } catch (const std::exception& e) {
        std::cerr << ""Error: "" << e.what() << std::flush;
        return 1;
    }
    return 0;
}
";
        }

        private string GenerateUserProjectHpp()
        {
            return @"#pragma once
#define MAYASIMPLE
#include ""MayaFlux/MayaFlux.hpp""

/**
 * @brief Configure engine preferences
 * 
 * This function runs BEFORE the engine starts. Set up:
 * - Sample rate: 48000 (pro), 44100 (CD), 96000 (studio)
 * - Buffer size: 128 (low latency), 512 (default), 1024 (high quality)
 * - Graphics: target_frame_rate, graphics API
 */
void settings()
{
    // Your MayaFlux configuration here
}

/**
 * @brief Create and run your audio/graphics pipeline
 * 
 * This is where you create nodes, buffers, and processing chains.
 */
void compose()
{
    // Your MayaFlux code here
}
";
        }

        private string GenerateVsCodeSettings()
        {
            return @"{
    ""C_Cpp.default.cppStandard"": ""c++23"",
    ""C_Cpp.default.compilerPath"": ""cl.exe"",
    ""C_Cpp.default.includePath"": [
        ""${workspaceFolder}/src"",
        ""${env:MAYAFLUX_ROOT}/include""
    ],
    ""C_Cpp.default.compileCommands"": ""${workspaceFolder}/build/compile_commands.json"",
    ""C_Cpp.intelliSenseEngine"": ""default"",
    ""cmake.configureOnOpen"": true,
    ""cmake.buildDirectory"": ""${workspaceFolder}/build"",
    ""editor.formatOnSave"": false
}
";
        }

        private string GenerateVsCodeTasks(string projectName)
        {
            return $@"{{
    ""version"": ""2.0.0"",
    ""tasks"": [
        {{
            ""label"": ""Configure CMake"",
            ""type"": ""shell"",
            ""command"": ""cmake"",
            ""args"": [
                ""-B"", ""build"",
                ""-S"", ""."",
                ""-DCMAKE_BUILD_TYPE=Release"",
                ""-DCMAKE_EXPORT_COMPILE_COMMANDS=ON""
            ],
            ""group"": ""build"",
            ""problemMatcher"": []
        }},
        {{
            ""label"": ""Build Project"",
            ""type"": ""shell"",
            ""command"": ""cmake"",
            ""args"": [
                ""--build"", ""build"",
                ""--config"", ""Release"",
                ""--parallel""
            ],
            ""group"": {{
                ""kind"": ""build"",
                ""isDefault"": true
            }},
            ""dependsOn"": [""Configure CMake""],
            ""problemMatcher"": [""$msCompile""]
        }},
        {{
            ""label"": ""Run {projectName}"",
            ""type"": ""shell"",
            ""command"": ""build/Release/{projectName}.exe"",
            ""group"": ""test"",
            ""dependsOn"": [""Build Project""],
            ""problemMatcher"": []
        }}
    ]
}}
";
        }

        private string GenerateVsCodeLaunch(string projectName)
        {
            return $@"{{
    ""version"": ""0.2.0"",
    ""configurations"": [
        {{
            ""name"": ""Debug {projectName}"",
            ""type"": ""cppvsdbg"",
            ""request"": ""launch"",
            ""program"": ""${{workspaceFolder}}/build/Debug/{projectName}.exe"",
            ""args"": [],
            ""stopAtEntry"": false,
            ""cwd"": ""${{workspaceFolder}}"",
            ""environment"": [
                {{
                    ""name"": ""MAYAFLUX_ROOT"",
                    ""value"": ""${{env:MAYAFLUX_ROOT}}""
                }}
            ],
            ""externalConsole"": false,
            ""preLaunchTask"": ""Build Project""
        }}
    ]
}}
";
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
