using System;
using System.Windows.Forms;
using System.Drawing;
using Weave.Shared.Models;

namespace Weave;

public class ModeSelector : Form
{
    public WeaveMode? SelectedMode { get; private set; }

    public ModeSelector()
    {
        InitializeUI();
    }

    private void InitializeUI()
    {
        Text = "Weave - Select Mode";
        Width = 500;
        Height = 300;
        StartPosition = FormStartPosition.CenterScreen;
        FormBorderStyle = FormBorderStyle.FixedDialog;
        MaximizeBox = false;
        MinimizeBox = false;
        Icon = SystemIcons.Application;
        ShowInTaskbar = true;

        var titleLabel = new Label
        {
            Text = "What would you like to do?",
            Font = new Font("Segoe UI", 14, FontStyle.Bold),
            Location = new Point(20, 20),
            AutoSize = true
        };
        Controls.Add(titleLabel);

        var installButton = new Button
        {
            Text = "Install MayaFlux",
            Location = new Point(50, 70),
            Width = 380,
            Height = 80,
            BackColor = Color.FromArgb(0, 120, 215),
            ForeColor = Color.White,
            Font = new Font("Segoe UI", 11, FontStyle.Bold),
            FlatStyle = FlatStyle.Flat,
            TextAlign = ContentAlignment.MiddleCenter
        };
        installButton.Click += (s, e) =>
        {
            SelectedMode = WeaveMode.Installation;
            DialogResult = DialogResult.OK;
            Close();
        };
        Controls.Add(installButton);

        var projectButton = new Button
        {
            Text = "Create Project",
            Location = new Point(50, 160),
            Width = 380,
            Height = 80,
            BackColor = Color.FromArgb(16, 124, 16),
            ForeColor = Color.White,
            Font = new Font("Segoe UI", 11, FontStyle.Bold),
            FlatStyle = FlatStyle.Flat,
            TextAlign = ContentAlignment.MiddleCenter
        };
        projectButton.Click += (s, e) =>
        {
            SelectedMode = WeaveMode.ProjectCreation;
            DialogResult = DialogResult.OK;
            Close();
        };
        Controls.Add(projectButton);
    }
}