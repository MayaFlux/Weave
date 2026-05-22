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
        var cmakeDir = Path.Combine(projectDir, "cmake");
        var vsCodeDir = Path.Combine(projectDir, ".vscode");

        Log($"Creating project structure at: {projectDir}");

        if (Directory.Exists(projectDir))
        {
            throw new Exception($"Project directory already exists: {projectDir}");
        }

        Directory.CreateDirectory(srcDir);
        Log("  Created src/");

        Directory.CreateDirectory(cmakeDir);
        Log("  Created cmake/");

        if (withVscodeCheckbox.Checked)
        {
            Directory.CreateDirectory(vsCodeDir);
            Log("  Created .vscode/");
        }

        var mayaFluxRoot = Environment.GetEnvironmentVariable("MAYAFLUX_ROOT") ?? "C:\\MayaFlux";
        Log($"  Using MAYAFLUX_ROOT: {mayaFluxRoot}");

        // -------------------------------------------------------------------------
        // CMakeLists.txt
        // -------------------------------------------------------------------------
        Log("Generating CMakeLists.txt");

        string lilaLinkBlock;
        string lilaDebuggerPath;
        string lilaDllCopy;

        if (withLilaCheckbox.Checked)
        {
            lilaLinkBlock = "target_link_libraries(${PROJECT_NAME} PRIVATE MayaFlux::MayaFluxHost)";
            lilaDebuggerPath = "$<TARGET_FILE_DIR:MayaFlux::MayaFluxHost>;";
            lilaDllCopy =
                "if(EXISTS \"$ENV{MAYAFLUX_ROOT}/bin/MayaFluxHost.dll\")\r\n" +
                "        add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD\r\n" +
                "            COMMAND ${CMAKE_COMMAND} -E copy_if_different\r\n" +
                "                \"$ENV{MAYAFLUX_ROOT}/bin/MayaFluxHost.dll\"\r\n" +
                "                $<TARGET_FILE_DIR:${PROJECT_NAME}>\r\n" +
                "        )\r\n" +
                "    endif()";
        }
        else
        {
            lilaLinkBlock = "";
            lilaDebuggerPath = "";
            lilaDllCopy = "";
        }

        var cmakelists = LoadTemplate("CMakeLists.txt")
            .Replace("@PROJECT_NAME@", projectName)
            .Replace("@LILA_LINK_BLOCK@", lilaLinkBlock)
            .Replace("@LILA_DEBUGGER_PATH@", lilaDebuggerPath)
            .Replace("@LILA_DLL_COPY@", lilaDllCopy);

        File.WriteAllText(Path.Combine(projectDir, "CMakeLists.txt"), cmakelists);
        Log("  Generated CMakeLists.txt");

        // -------------------------------------------------------------------------
        // cmake/ modules
        // -------------------------------------------------------------------------
        Log("Copying cmake modules");

        var shadersCmakeSrc = Path.Combine(templatesDir, "cmake", "shaders.cmake");
        if (File.Exists(shadersCmakeSrc))
        {
            File.Copy(shadersCmakeSrc, Path.Combine(cmakeDir, "shaders.cmake"));
            Log("  Copied cmake/shaders.cmake");
        }
        else
        {
            throw new FileNotFoundException("Required template missing: cmake/shaders.cmake");
        }

        var buildCommunitySrc = Path.Combine(templatesDir, "cmake", "build_community.cmake");
        if (File.Exists(buildCommunitySrc))
        {
            File.Copy(buildCommunitySrc, Path.Combine(cmakeDir, "build_community.cmake"));
            Log("  Copied cmake/build_community.cmake");
        }
        else
        {
            throw new FileNotFoundException("Required template missing: cmake/build_community.cmake");
        }

        // -------------------------------------------------------------------------
        // Source files
        // -------------------------------------------------------------------------
        Log("Generating source files");

        File.WriteAllText(Path.Combine(srcDir, "main.cpp"), LoadTemplate("main.cpp"));
        Log("  Generated main.cpp");

        File.WriteAllText(Path.Combine(srcDir, "user_project.hpp"), LoadTemplate("user_project.hpp"));
        Log("  Generated user_project.hpp");

        // -------------------------------------------------------------------------
        // data/shaders
        // -------------------------------------------------------------------------
        var dataShadersDir = Path.Combine(projectDir, "data", "shaders");
        Directory.CreateDirectory(dataShadersDir);
        Log("  Created data/shaders/");

        var templateShadersDir = Path.Combine(templatesDir, "shaders");
        if (Directory.Exists(templateShadersDir))
        {
            foreach (var shader in Directory.GetFiles(templateShadersDir))
            {
                File.Copy(shader, Path.Combine(dataShadersDir, Path.GetFileName(shader)));
                Log($"  Copied {Path.GetFileName(shader)}");
            }
        }

        // -------------------------------------------------------------------------
        // community.cmake (empty sentinel)
        // -------------------------------------------------------------------------
        File.WriteAllText(Path.Combine(projectDir, "community.cmake"), "");
        Log("  Created community.cmake");

        // -------------------------------------------------------------------------
        // .gitignore
        // -------------------------------------------------------------------------
        var gitignoreSrc = Path.Combine(templatesDir, ".gitignore");
        if (File.Exists(gitignoreSrc))
        {
            File.Copy(gitignoreSrc, Path.Combine(projectDir, ".gitignore"));
            Log("  Copied .gitignore");
        }
        else
        {
            throw new FileNotFoundException("Required template missing: .gitignore");
        }

        // -------------------------------------------------------------------------
        // VS Code configuration
        // -------------------------------------------------------------------------
        if (withVscodeCheckbox.Checked)
        {
            Log("Generating VS Code configuration");
            var vscodeTemplatesDir = Path.Combine(templatesDir, "vscode");
            if (Directory.Exists(vscodeTemplatesDir))
            {
                foreach (var vscodeFile in new[] { "settings.json", "tasks.json", "launch.json" })
                {
                    var src = Path.Combine(vscodeTemplatesDir, vscodeFile);
                    if (File.Exists(src))
                    {
                        var content = File.ReadAllText(src).Replace("@PROJECT_NAME@", projectName);
                        File.WriteAllText(Path.Combine(vsCodeDir, vscodeFile), content);
                        Log($"  Generated .vscode/{vscodeFile}");
                    }
                }
            }
            else
            {
                Log("  WARNING: vscode templates not found, skipping");
            }
        }

        // -------------------------------------------------------------------------
        // README.md
        // -------------------------------------------------------------------------
        Log("Generating README.md");
        var readme =
            $"# {projectName}\r\n\r\n" +
            "A MayaFlux multimedia DSP project.\r\n\r\n" +
            "## Building\r\n\r\n" +
            "```bash\r\n" +
            "mkdir build && cd build\r\n" +
            "cmake .. -DCMAKE_BUILD_TYPE=Release\r\n" +
            "cmake --build . --config Release --parallel\r\n" +
            "```\r\n\r\n" +
            "## Running\r\n\r\n" +
            $"```bash\r\n.\\build\\Release\\{projectName}.exe\r\n```\r\n\r\n" +
            "## Editing\r\n\r\n" +
            "Open in VS Code:\r\n```bash\r\ncode .\r\n```\r\n\r\n" +
            "Edit your code in `src/user_project.hpp`:\r\n" +
            "- `settings()`: Configure sample rate, buffer size, graphics\r\n" +
            "- `compose()`: Create your nodes, buffers, and processing chains\r\n\r\n" +
            "## Documentation\r\n\r\n" +
            "See [MayaFlux Documentation](https://github.com/MayaFlux/MayaFlux)\r\n";

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
