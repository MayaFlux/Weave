using System.Drawing;

namespace Weave.Theme;

public static class ThemeColors
{
    // Dark backgrounds
    public static readonly Color BackgroundDark = Color.FromArgb(30, 30, 30);
    public static readonly Color BackgroundMedium = Color.FromArgb(45, 45, 45);
    public static readonly Color BackgroundLight = Color.FromArgb(60, 60, 60);
    
    // Text colors
    public static readonly Color TextPrimary = Color.FromArgb(240, 240, 240);
    public static readonly Color TextSecondary = Color.FromArgb(160, 160, 160);
    public static readonly Color TextMuted = Color.FromArgb(100, 100, 100);
    
    // UI elements
    public static readonly Color ButtonPrimary = Color.FromArgb(0, 120, 215);
    public static readonly Color ButtonSuccess = Color.FromArgb(16, 124, 16);
    public static readonly Color ButtonDanger = Color.FromArgb(200, 60, 60);
    public static readonly Color ButtonSecondary = Color.FromArgb(80, 80, 80);
    
    // Status colors
    public static readonly Color Success = Color.FromArgb(76, 175, 80);
    public static readonly Color Warning = Color.FromArgb(255, 193, 7);
    public static readonly Color Error = Color.FromArgb(244, 67, 54);
    
    // Borders
    public static readonly Color Border = Color.FromArgb(70, 70, 70);
}