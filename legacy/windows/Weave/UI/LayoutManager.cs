using System;
using System.Windows.Forms;
using System.Drawing;
using System.Collections.Generic;
using Weave.Theme;

namespace Weave.UI.Layout;

/// <summary>
/// Global UI layout management system that ensures consistent positioning,
/// sizing, and spacing across all installer steps. This is the single source
/// of truth for all layout calculations.
/// </summary>
public class LayoutManager
{
    private Panel container;
    private int usableWidth;
    private List<LayoutControlWrapper> managedControls = new();

    public LayoutManager(Panel containerPanel)
    {
        container = containerPanel;
        container.Resize += Container_Resize;
        CalculateUsableWidth();
    }

    // ============================================================================
    // LAYOUT LIFECYCLE
    // ============================================================================

    private void Container_Resize(object? sender, EventArgs e)
    {
        CalculateUsableWidth();
        RefreshLayout();
    }

    private void CalculateUsableWidth()
    {
        usableWidth = container.ClientSize.Width - (LayoutConstants.MarginMedium * 2);
        if (usableWidth < 200)
            usableWidth = 200;
    }

    /// <summary>
    /// Called after all controls are added to recalculate positions based on actual container size.
    /// </summary>
    public void RefreshLayout()
    {
        if (container.ClientSize.Width == 0 || container.ClientSize.Height == 0)
            return;

        CalculateUsableWidth();

        int currentY = LayoutConstants.MarginMedium;
        foreach (var wrapper in managedControls)
        {
            wrapper.UpdateLayout(LayoutConstants.MarginMedium, ref currentY, usableWidth);
        }
    }

    // ============================================================================
    // VERTICAL STACK LAYOUT - Core layout primitive
    // ============================================================================

    /// <summary>
    /// Adds a control to a vertical stack and tracks it for layout management.
    /// </summary>
    public void AddToStack(Control control, int spacing = LayoutConstants.SpacingMedium)
    {
        var wrapper = new LayoutControlWrapper(control, spacing);
        managedControls.Add(wrapper);
        container.Controls.Add(control);
    }

    /// <summary>
    /// Adds multiple controls as a vertical stack.
    /// </summary>
    public void AddToStack(params Control[] controls)
    {
        foreach (var control in controls)
        {
            AddToStack(control, LayoutConstants.SpacingMedium);
        }
    }

    // ============================================================================
    // SECTION BUILDERS - High-level components
    // ============================================================================

    /// <summary>
    /// Creates and adds a title label with consistent styling and spacing.
    /// </summary>
    public Label AddTitle(string text)
    {
        var title = new Label
        {
            Text = text,
            Font = new Font("Segoe UI", 14, FontStyle.Bold),
            AutoSize = true,
            ForeColor = ThemeColors.TextPrimary,
            BackColor = ThemeColors.BackgroundDark
        };
        AddToStack(title, LayoutConstants.SpacingLarge);
        return title;
    }

    /// <summary>
    /// Creates and adds a subtitle label.
    /// </summary>
    public Label AddSubtitle(string text)
    {
        var subtitle = new Label
        {
            Text = text,
            Font = new Font("Segoe UI", 10),
            ForeColor = ThemeColors.TextSecondary,
            BackColor = ThemeColors.BackgroundDark,
            AutoSize = true
        };
        AddToStack(subtitle, LayoutConstants.SpacingMedium);
        return subtitle;
    }

    /// <summary>
    /// Creates a label-input pair with consistent spacing.
    /// </summary>
    public TextBox AddLabeledInput(string labelText, string placeholder = "")
    {
        var label = new Label
        {
            Text = labelText,
            Font = new Font("Segoe UI", 9),
            ForeColor = ThemeColors.TextSecondary,
            BackColor = ThemeColors.BackgroundDark,
            AutoSize = true
        };
        AddToStack(label, LayoutConstants.SpacingSmall);

        var input = new TextBox
        {
            Text = placeholder,
            Font = new Font("Segoe UI", 10),
            BackColor = ThemeColors.BackgroundMedium,
            ForeColor = ThemeColors.TextPrimary,
            Height = LayoutConstants.InputHeight,
            Width = usableWidth
        };
        input.BorderStyle = BorderStyle.FixedSingle;
        AddToStack(input, LayoutConstants.SpacingMedium);

        return input;
    }

    /// <summary>
    /// Creates a multiline log box with consistent sizing.
    /// </summary>
    public TextBox AddLogBox(int height = LayoutConstants.LogBoxMaxHeight)
    {
        var logBox = new TextBox
        {
            Multiline = true,
            ReadOnly = true,
            Font = new Font("Consolas", 9),
            BackColor = ThemeColors.BackgroundMedium,
            ForeColor = ThemeColors.TextPrimary,
            ScrollBars = ScrollBars.Vertical,
            Width = usableWidth,
            Height = height,
            BorderStyle = BorderStyle.FixedSingle
        };
        AddToStack(logBox, LayoutConstants.SpacingMedium);
        return logBox;
    }

    /// <summary>
    /// Creates a progress bar with consistent sizing.
    /// </summary>
    public ProgressBar AddProgressBar()
    {
        var progressBar = new ProgressBar
        {
            Width = usableWidth,
            Height = 20,
            Style = ProgressBarStyle.Continuous
        };
        AddToStack(progressBar, LayoutConstants.SpacingLarge);
        return progressBar;
    }

    /// <summary>
    /// Creates a status label.
    /// </summary>
    public Label AddStatusLabel(string text = "")
    {
        var status = new Label
        {
            Text = text,
            Font = new Font("Segoe UI", 10),
            ForeColor = ThemeColors.TextSecondary,
            BackColor = ThemeColors.BackgroundDark,
            AutoSize = true
        };
        AddToStack(status, LayoutConstants.SpacingMedium);
        return status;
    }

    /// <summary>
    /// Creates a checkbox with label.
    /// </summary>
    public CheckBox AddCheckbox(string text)
    {
        var checkbox = new CheckBox
        {
            Text = text,
            Font = new Font("Segoe UI", 10),
            ForeColor = ThemeColors.TextPrimary,
            BackColor = ThemeColors.BackgroundDark,
            AutoSize = true,
            Checked = false
        };
        AddToStack(checkbox, LayoutConstants.SpacingMedium);
        return checkbox;
    }

    // ============================================================================
    // BUTTON LAYOUT - Bottom action buttons with consistent spacing
    // ============================================================================

    /// <summary>
    /// Adds a spacer to push buttons to the bottom of the container.
    /// </summary>
    public void AddFlexibleSpacer()
    {
        var spacer = new Label { Height = 1, AutoSize = false };
        managedControls.Add(new LayoutControlWrapper(spacer, 0, true)); // isFlexible = true
    }

    /// <summary>
    /// Creates a button panel with consistent styling.
    /// </summary>
    public Button AddButton(string text, Color? backgroundColor = null)
    {
        var button = new Button
        {
            Text = text,
            Width = 100,
            Height = LayoutConstants.ButtonHeight,
            BackColor = backgroundColor ?? ThemeColors.ButtonPrimary,
            ForeColor = Color.White,
            FlatStyle = FlatStyle.Flat,
            Font = new Font("Segoe UI", 9),
            Cursor = Cursors.Hand
        };
        AddToStack(button, LayoutConstants.SpacingSmall);
        return button;
    }

    /// <summary>
    /// Creates a button group (Next, Cancel, etc.) with proper spacing.
    /// </summary>
    public (Button, Button) AddButtonPair(string primaryText, string secondaryText)
    {
        AddFlexibleSpacer();

        var primaryBtn = AddButton(primaryText, ThemeColors.ButtonPrimary);
        var secondaryBtn = AddButton(secondaryText, ThemeColors.ButtonSecondary);

        return (primaryBtn, secondaryBtn);
    }

    // ============================================================================
    // UTILITY METHODS
    /// </summary>

    /// <summary>
    /// Gets the current usable width (container width minus margins).
    /// </summary>
    public int GetUsableWidth() => usableWidth;

    /// <summary>
    /// Gets the container's ClientSize height.
    /// </summary>
    public int GetUsableHeight() => container.ClientSize.Height;

    /// <summary>
    /// Clears all managed controls and the container.
    /// </summary>
    public void Clear()
    {
        container.Controls.Clear();
        managedControls.Clear();
    }
}

// ============================================================================
// INTERNAL: Control Wrapper for Layout Tracking
// ============================================================================

internal class LayoutControlWrapper
{
    private Control control;
    private int spacingAfter;
    private bool isFlexible;

    public LayoutControlWrapper(Control ctrl, int spacing, bool flexible = false)
    {
        control = ctrl;
        spacingAfter = spacing;
        isFlexible = flexible;
    }

    public void UpdateLayout(int leftMargin, ref int currentY, int availableWidth)
    {
        if (isFlexible)
        {
            control.Height = Math.Max(1, 300 - currentY);
            control.Location = new Point(leftMargin, currentY);
        }
        else
        {
            if (control is TextBox || control is ProgressBar)
            {
                control.Width = availableWidth;
            }

            control.Location = new Point(leftMargin, currentY);
            currentY += control.Height + spacingAfter;
        }
    }
}