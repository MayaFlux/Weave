using System;
using System.IO;
using System.Net.Http;
using System.Threading.Tasks;
using System.Windows.Forms;
using Weave.Shared;
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

        moduleNameInput = layoutManager.AddLabeledInput("Module Names:", "module_one, module_two");
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
        if (moduleNameInput.Text.Length == 0)
        {
            nameErrorLabel.Text = "";
            return;
        }

        try
        {
            CommunityUpdate.ParseNames([moduleNameInput.Text]);
            nameErrorLabel.Text = "";
        }
        catch (InvalidOperationException error)
        {
            nameErrorLabel.Text = error.Message;
        }
    }

    private void AddButton_Click(object sender, EventArgs e)
    {
        var projectPath = projectPathInput.Text.Trim();
        var names = moduleNameInput.Text;

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
        try
        {
            CommunityUpdate.ParseNames([names]);
        }
        catch (InvalidOperationException error)
        {
            MessageBox.Show(error.Message, "Validation Error", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }

        SetFormEnabled(false);
        outputLog.Clear();

        Task.Run(() => AddModuleAsync(projectPath, names));
    }

    private async Task AddModuleAsync(string projectPath, string names)
    {
        try
        {
            using var http = new HttpClient { Timeout = TimeSpan.FromSeconds(30) };
            var update = new CommunityUpdate(
                http,
                Log,
                Environment.GetEnvironmentVariable("MAYAFLUX_ROOT"));

            await update.RunAsync(projectPath, [names]);

            Invoke(() =>
            {
                MessageBox.Show(
                    "Modules added. Rebuild your project to include them.",
                    "Modules Added",
                    MessageBoxButtons.OK,
                    MessageBoxIcon.Information);
                ParentForm?.Close();
            });
        }
        catch (Exception error)
        {
            Log($"[ERROR] {error.Message}");
            Invoke(() => SetFormEnabled(true));
        }
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
