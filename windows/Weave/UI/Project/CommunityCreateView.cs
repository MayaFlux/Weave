using System;
using System.IO;
using System.Text.RegularExpressions;
using System.Windows.Forms;
using Weave.Theme;
using Weave.UI.Layout;
using Weave.Shared;
using Weave.UI.Pages;

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
    private readonly Action<string, string, MessageBoxIcon> notify;
    private readonly ProjectCreationHost creationHost = new();
    private readonly DependenciesHost gitHost = new();

    public CommunityCreatorView(string templatesDirectory, Action<string, string, MessageBoxIcon>? notify = null)
    {
        this.notify = notify ?? ((message, title, icon) => MessageBox.Show(message, title, MessageBoxButtons.OK, icon));
        templatesDir = templatesDirectory;
        InitializeComponent();
        InitializeUI();
    }

    internal CommunityCreatorView(string templatesDirectory, ProjectCreationHost creationHost, DependenciesHost gitHost, Action<string, string, MessageBoxIcon>? notify = null)
        : this(templatesDirectory, notify)
    {
        this.creationHost = creationHost;
        this.gitHost = gitHost;
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

        var verInput = layoutManager.AddLabeledInput("Minimum MayaFlux Version:", "0.5.0");
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
        nameErrorLabel.Text = Regex.IsMatch(name, @"\A[a-z][a-z0-9_]*\z") && !Regex.IsMatch(name, @"\A(?:con|prn|aux|nul|com[1-9]|lpt[1-9])\z")
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

    private async void CreateButton_Click(object sender, EventArgs e)
    {
        SetFormEnabled(false);
        outputLog.Clear();
        var warnings = new List<string>();
        try
        {
            var service = new CommunityCreationService(templatesDir, creationHost, gitHost, message =>
            {
                Log(message);
                if (message.StartsWith("WARNING:", StringComparison.Ordinal)) warnings.Add(message);
            });
            var module = await service.CreateAsync(moduleNameInput.Text.Trim(), modulePathInput.Text.Trim(), descriptionInput.Text.Trim(), minVersionInput.Text.Trim(), needsLilaCheckbox.Checked);
            if (IsDisposed || Disposing) return;
            notify($"Community module created at:\n{module}" + (warnings.Count == 0 ? "" : "\n\n" + string.Join("\n", warnings)), "Success", MessageBoxIcon.Information);
            ParentForm?.Close();
        }
        catch (Exception error)
        {
            Log($"Error: {error.Message}");
            if (!IsDisposed && !Disposing) notify($"Failed to create module:\n{error.Message}", "Error", MessageBoxIcon.Error);
        }
        finally
        {
            if (!IsDisposed && !Disposing) SetFormEnabled(true);
        }
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
        if (IsDisposed || Disposing) return;
        if (outputLog.InvokeRequired)
        {
            outputLog.Invoke(new Action(() => Log(message)));
            return;
        }
        outputLog.AppendText(message + "\n");
        outputLog.ScrollToCaret();
    }
}
