using System;
using System.Windows.Forms;
using System.Drawing;

namespace Weave.Theme;

public static class ThemeExtensions
{
    public static void ApplyDarkTheme(this Form form)
    {
        form.BackColor = ThemeColors.BackgroundDark;
        form.ForeColor = ThemeColors.TextPrimary;
        ApplyThemeRecursive(form);
    }

    public static void ApplyDarkTheme(this Panel panel)
    {
        panel.BackColor = ThemeColors.BackgroundDark;
        panel.ForeColor = ThemeColors.TextPrimary;
        ApplyThemeRecursive(panel);
    }

    private static void ApplyThemeRecursive(Control control)
    {
        foreach (Control child in control.Controls)
        {
            switch (child)
            {
                case Label label:
                    label.BackColor = ThemeColors.BackgroundDark;
                    label.ForeColor = ThemeColors.TextPrimary;
                    break;

                case TextBox textBox:
                    textBox.BackColor = ThemeColors.BackgroundMedium;
                    textBox.ForeColor = ThemeColors.TextPrimary;
                    break;

                case RichTextBox richTextBox:
                    richTextBox.BackColor = ThemeColors.BackgroundMedium;
                    richTextBox.ForeColor = ThemeColors.TextPrimary;
                    break;

                case Button button:
                    // Keep buttons their assigned colors (primary, success, etc)
                    button.ForeColor = Color.White;
                    break;

                case CheckBox checkBox:
                    checkBox.BackColor = ThemeColors.BackgroundDark;
                    checkBox.ForeColor = ThemeColors.TextPrimary;
                    break;

                case ProgressBar progressBar:
                    progressBar.BackColor = ThemeColors.BackgroundMedium;
                    progressBar.ForeColor = ThemeColors.ButtonPrimary;
                    break;

                case Panel panel:
                    panel.BackColor = ThemeColors.BackgroundDark;
                    panel.ForeColor = ThemeColors.TextPrimary;
                    break;

                case GroupBox groupBox:
                    groupBox.BackColor = ThemeColors.BackgroundDark;
                    groupBox.ForeColor = ThemeColors.TextPrimary;
                    break;

                default:
                    child.BackColor = ThemeColors.BackgroundDark;
                    child.ForeColor = ThemeColors.TextPrimary;
                    break;
            }

            ApplyThemeRecursive(child);
        }
    }
}