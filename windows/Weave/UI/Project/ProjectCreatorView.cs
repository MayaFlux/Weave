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
    private CheckBox withShaderSourcesCheckbox;
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
        withShaderSourcesCheckbox = layoutManager.AddCheckbox("Copy MayaFlux shader sources");

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
            var warnings = outputLog.Lines.Where(line => line.Contains("WARNING:")).ToArray();
            var message = $"Project '{projectName}' created at:\n{Path.Combine(projectPath, projectName)}";
            if (warnings.Length > 0) message += "\n\nThe project is ready. Some optional editor files need attention:\n" + string.Join("\n", warnings);
            MessageBox.Show(message, "Success", MessageBoxButtons.OK, MessageBoxIcon.Information);
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
        var service = new ProjectCreationService(templatesDir, new ProjectCreationHost(), Log);
        service.Create(projectName, projectPath, withLilaCheckbox.Checked, withVscodeCheckbox.Checked, withShaderSourcesCheckbox.Checked);
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
