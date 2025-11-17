using System;
using System.Windows.Forms;
using Weave.Shared.Models;
using Weave.UI.Layout;

namespace Weave.Modes;

/// <summary>
/// Interface for installation steps. Each step receives a LayoutManager
/// instead of building UI manually. This ensures consistent layout across all steps.
/// </summary>
public interface IInstallationStep
{
    /// <summary>
    /// Builds the UI for this step using LayoutManager primitives.
    /// The LayoutManager handles all positioning and sizing.
    /// </summary>
    /// <param name="layoutManager">Global layout manager for consistent UI</param>
    /// <param name="config">Installation configuration</param>
    /// <param name="logCallback">Callback to log messages</param>
    /// <param name="nextCallback">Callback when user clicks next</param>
    void BuildUI(
        LayoutManager layoutManager,
        InstallationConfig config,
        Action<string> logCallback,
        Action nextCallback
    );
}