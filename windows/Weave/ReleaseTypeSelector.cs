using System;
using System.Drawing;
using System.Windows.Forms;
using Weave.Shared.Models;
using Weave.Theme;
using static Weave.Theme.ThemeExtensions;

namespace Weave;

public enum ReleaseType
{
    Stable,
    Development
}

public class ReleaseTypeSelector : Form
{
    public ReleaseType? SelectedType { get; private set; }
    private RadioButton stableRadio;
    private RadioButton devRadio;

    public ReleaseTypeSelector()
    {
        InitializeUI();
    }

    private void InitializeUI()
    {
        Text = "Weave - Select Release Channel";
        Width = 550;
        Height = 400;
        StartPosition = FormStartPosition.CenterScreen;
        FormBorderStyle = FormBorderStyle.FixedDialog;
        MaximizeBox = false;
        MinimizeBox = false;
        Icon = SystemIcons.Application;
        ShowInTaskbar = true;

        this.ApplyDarkTheme();

        var mainPanel = new Panel
        {
            Dock = DockStyle.Fill,
            Padding = new Padding(30),
            BackColor = ThemeColors.BackgroundDark
        };

        var titleLabel = new Label
        {
            Text = "Select Release Channel",
            Font = new Font("Segoe UI", 14, FontStyle.Bold),
            Location = new Point(0, 0),
            AutoSize = true,
            BackColor = ThemeColors.BackgroundDark,
            ForeColor = ThemeColors.TextPrimary
        };
        mainPanel.Controls.Add(titleLabel);

        var subtitleLabel = new Label
        {
            Text = "Choose which version of MayaFlux to install:",
            Font = new Font("Segoe UI", 9),
            Location = new Point(0, 35),
            AutoSize = true,
            BackColor = ThemeColors.BackgroundDark,
            ForeColor = ThemeColors.TextSecondary
        };
        mainPanel.Controls.Add(subtitleLabel);

        var stablePanel = new Panel
        {
            Location = new Point(0, 70),
            Width = 490,
            Height = 90,
            BackColor = ThemeColors.BackgroundMedium,
            BorderStyle = BorderStyle.FixedSingle
        };

        stableRadio = new RadioButton
        {
            Text = "Stable Release (Recommended)",
            Location = new Point(15, 15),
            AutoSize = true,
            Checked = true,
            Font = new Font("Segoe UI", 10, FontStyle.Bold),
            BackColor = ThemeColors.BackgroundMedium,
            ForeColor = ThemeColors.TextPrimary
        };
        stablePanel.Controls.Add(stableRadio);

        var stableDesc = new Label
        {
            Text = "Production-ready release. Tested and stable.\nRecommended for general use and production environments.",
            Location = new Point(35, 40),
            Width = 440,
            Height = 40,
            Font = new Font("Segoe UI", 8.5f),
            BackColor = ThemeColors.BackgroundMedium,
            ForeColor = ThemeColors.TextSecondary
        };
        stablePanel.Controls.Add(stableDesc);

        mainPanel.Controls.Add(stablePanel);

        var devPanel = new Panel
        {
            Location = new Point(0, 170),
            Width = 490,
            Height = 90,
            BackColor = ThemeColors.BackgroundMedium,
            BorderStyle = BorderStyle.FixedSingle
        };

        devRadio = new RadioButton
        {
            Text = "Development Release",
            Location = new Point(15, 15),
            AutoSize = true,
            Font = new Font("Segoe UI", 10, FontStyle.Bold),
            BackColor = ThemeColors.BackgroundMedium,
            ForeColor = ThemeColors.TextPrimary
        };
        devRadio.CheckedChanged += (s, e) => stableRadio.Checked = !devRadio.Checked;
        stableRadio.CheckedChanged += (s, e) => devRadio.Checked = !stableRadio.Checked;
        devPanel.Controls.Add(devRadio);

        var devDesc = new Label
        {
            Text = "Latest features and improvements. May contain bugs.\nFor testing and early access to new functionality.",
            Location = new Point(35, 40),
            Width = 440,
            Height = 40,
            Font = new Font("Segoe UI", 8.5f),
            BackColor = ThemeColors.BackgroundMedium,
            ForeColor = ThemeColors.TextSecondary
        };
        devPanel.Controls.Add(devDesc);

        mainPanel.Controls.Add(devPanel);

        var buttonPanel = new Panel
        {
            Height = 50,
            Dock = DockStyle.Bottom,
            BackColor = ThemeColors.BackgroundDark,
            Padding = new Padding(0, 10, 0, 0)
        };

        var continueButton = new Button
        {
            Text = "Continue",
            Width = 100,
            Height = 35,
            Location = new Point(390, 5),
            BackColor = ThemeColors.ButtonPrimary,
            ForeColor = Color.White,
            Font = new Font("Segoe UI", 9, FontStyle.Bold),
            FlatStyle = FlatStyle.Flat,
            Cursor = Cursors.Hand
        };
        continueButton.Click += (s, e) =>
        {
            SelectedType = stableRadio.Checked ? ReleaseType.Stable : ReleaseType.Development;
            DialogResult = DialogResult.OK;
            Close();
        }; 
        buttonPanel.Controls.Add(continueButton);

        var cancelButton = new Button
        {
            Text = "Cancel",
            Width = 100,
            Height = 35,
            Location = new Point(280, 5),
            BackColor = ThemeColors.ButtonSecondary,
            ForeColor = Color.White,
            Font = new Font("Segoe UI", 9),
            FlatStyle = FlatStyle.Flat,
            Cursor = Cursors.Hand
        };
        cancelButton.Click += (s, e) =>
        {
            DialogResult = DialogResult.Cancel;
            Close();
        };
        buttonPanel.Controls.Add(cancelButton);

        mainPanel.Controls.Add(buttonPanel);
        Controls.Add(mainPanel);
    }
}