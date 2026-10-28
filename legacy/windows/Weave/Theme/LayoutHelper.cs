using System;
using System.Windows.Forms;
using System.Drawing;

namespace Weave.Theme;

public static class LayoutHelper
{
    // ============================================================================
    // Positions controls vertically with consistent spacing.
    // Returns the Y position for the next control.
    // ============================================================================
    public static int StackVertical(Control control, int currentY, int spacing = LayoutConstants.SpacingMedium)
    {
        control.Location = new Point(control.Location.X, currentY);
        return currentY + control.Height + spacing;
    }

    // ============================================================================
    // Creates a label-input pair with consistent spacing
    // ============================================================================
    public static int AddLabeledInput(
        Panel container,
        string labelText,
        TextBox input,
        int currentY,
        int width)
    {
        var label = new Label
        {
            Text = labelText,
            Location = new Point(LayoutConstants.MarginMedium, currentY),
            AutoSize = true,
            ForeColor = ThemeColors.TextSecondary,
            BackColor = ThemeColors.BackgroundDark,
            Font = new Font("Segoe UI", 9)
        };
        container.Controls.Add(label);

        input.Location = new Point(LayoutConstants.MarginMedium, currentY + LayoutConstants.LabelHeight + LayoutConstants.SpacingSmall);
        input.Width = width - (LayoutConstants.MarginMedium * 2);
        input.Height = LayoutConstants.InputHeight;
        input.BackColor = ThemeColors.BackgroundMedium;
        input.ForeColor = ThemeColors.TextPrimary;
        container.Controls.Add(input);

        return input.Location.Y + input.Height + LayoutConstants.SpacingMedium;
    }

    // ============================================================================
    // Creates title label
    // ============================================================================
    public static int AddTitle(
        Panel container,
        string titleText,
        int startY = LayoutConstants.MarginMedium)
    {
        var title = new Label
        {
            Text = titleText,
            Location = new Point(LayoutConstants.MarginMedium, startY),
            AutoSize = true,
            Font = new Font("Segoe UI", 14, FontStyle.Bold),
            ForeColor = ThemeColors.TextPrimary,
            BackColor = ThemeColors.BackgroundDark
        };
        container.Controls.Add(title);

        return startY + title.Height + LayoutConstants.SpacingLarge;
    }

    // ============================================================================
    // Creates section divider with space
    // ============================================================================
    public static int AddSectionBreak(Panel container, int currentY)
    {
        return currentY + LayoutConstants.SpacingLarge;
    }

    // ============================================================================
    // Positions button(s) at bottom of container with consistent spacing
    // ============================================================================
    public static void PositionButtonsAtBottom(
        Panel container,
        int buttonsHeight,
        params (Button button, int order)[] buttons)
    {
        int bottomY = container.Height - buttonsHeight - LayoutConstants.MarginMedium;
        int rightX = container.Width - LayoutConstants.MarginMedium;

        // Sort by order descending (right-to-left)
        var sorted = buttons;
        Array.Sort(sorted, (a, b) => b.order.CompareTo(a.order));

        foreach (var (button, _) in sorted)
        {
            button.Location = new Point(rightX - button.Width, bottomY);
            rightX -= button.Width + LayoutConstants.SpacingSmall;
        }
    }
}