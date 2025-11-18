using System;
using System.Diagnostics;
using System.Security.Principal;
using System.Windows.Forms;
using Weave.Shared.Models;
using Weave.UI;

namespace Weave;

static class Program
{
    [STAThread]
    static void Main()
    {
        if (!IsRunningAsAdmin())
        {
            RestartAsAdmin();
            return;
        }

        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);

        var modeSelector = new ModeSelector();
        if (modeSelector.ShowDialog() != DialogResult.OK || modeSelector.SelectedMode == null)
        {
            Application.Exit();
            return;
        }

        var mode = modeSelector.SelectedMode.Value;
        var mainWindow = new MainWindow(mode);
        Application.Run(mainWindow);
    }

    private static bool IsRunningAsAdmin()
    {
        try
        {
            var identity = WindowsIdentity.GetCurrent();
            var principal = new WindowsPrincipal(identity);
            return principal.IsInRole(WindowsBuiltInRole.Administrator);
        }
        catch
        {
            return false;
        }
    }

    private static void RestartAsAdmin()
    {
        try
        {
            var exePath = System.Reflection.Assembly.GetExecutingAssembly().Location;
            var psi = new ProcessStartInfo
            {
                FileName = exePath,
                UseShellExecute = true,
                Verb = "runas"
            };
            Process.Start(psi);
        }
        catch
        {
            MessageBox.Show(
                "This application requires administrator privileges to run.\n\n" +
                "Please run Weave as administrator.",
                "Administrator Required",
                MessageBoxButtons.OK,
                MessageBoxIcon.Error
            );
        }
    }
}