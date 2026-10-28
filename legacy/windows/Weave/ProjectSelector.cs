using System;
using System.Drawing;
using System.Windows.Forms;
using Weave.Shared.Models;
using Weave.Theme;
using static Weave.Theme.ThemeExtensions;

namespace Weave;

public class ProjectsSelector : Form
{
    public WeaveMode? SelectedMode { get; private set; }

    public ProjectsSelector()
    {
        InitializeUI();
    }

    private void InitializeUI()
    {
        Text = "Weave - Projects";
        Width = 500;
        Height = 370;
        StartPosition = FormStartPosition.CenterScreen;
        FormBorderStyle = FormBorderStyle.FixedDialog;
        MaximizeBox = false;
        MinimizeBox = false;
        Icon = SystemIcons.Application;
        ShowInTaskbar = true;

        this.ApplyDarkTheme();

        var titleLabel = new Label
        {
            Text = "Projects",
            Font = new Font("Segoe UI", 14, FontStyle.Bold),
            Location = new Point(20, 20),
            AutoSize = true,
            BackColor = ThemeColors.BackgroundDark,
            ForeColor = ThemeColors.TextPrimary
        };
        Controls.Add(titleLabel);

        var createButton = new Button
        {
            Text = "Create Project",
            Location = new Point(50, 70),
            Width = 380,
            Height = 70,
            BackColor = ThemeColors.ButtonPrimary,
            ForeColor = Color.White,
            Font = new Font("Segoe UI", 11, FontStyle.Bold),
            FlatStyle = FlatStyle.Flat,
            TextAlign = ContentAlignment.MiddleCenter
        };
        createButton.Click += (s, e) => Select(WeaveMode.ProjectCreation);
        Controls.Add(createButton);

        var updateButton = new Button
        {
            Text = "Update Project (Community Modules)",
            Location = new Point(50, 155),
            Width = 380,
            Height = 70,
            BackColor = ThemeColors.ButtonPrimary,
            ForeColor = Color.White,
            Font = new Font("Segoe UI", 11, FontStyle.Bold),
            FlatStyle = FlatStyle.Flat,
            TextAlign = ContentAlignment.MiddleCenter
        };
        updateButton.Click += (s, e) => Select(WeaveMode.UpdateProject);
        Controls.Add(updateButton);

        var communityButton = new Button
        {
            Text = "Create Community Module",
            Location = new Point(50, 240),
            Width = 380,
            Height = 70,
            BackColor = ThemeColors.ButtonPrimary,
            ForeColor = Color.White,
            Font = new Font("Segoe UI", 11, FontStyle.Bold),
            FlatStyle = FlatStyle.Flat,
            TextAlign = ContentAlignment.MiddleCenter
        };
        communityButton.Click += (s, e) => Select(WeaveMode.CreateCommunity);
        Controls.Add(communityButton);
    }

    private void Select(WeaveMode mode)
    {
        SelectedMode = mode;
        DialogResult = DialogResult.OK;
        Close();
    }
}