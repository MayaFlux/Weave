using System;
using System.Windows.Forms;
using Weave.Shared.Models;
using Weave.UI;
using Weave.Utils;
using System.Diagnostics;

namespace Weave;

static class Program
{
    [STAThread]
    static void Main(string[] args)
    {
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);

        WeaveMode mode;
        var installationRequested = args.Length == 1 && args[0] == InstallationElevation.InstallationArgument;
        while (true)
        {
            if (installationRequested)
            {
                mode = WeaveMode.Installation;
                installationRequested = false;
            }
            else
            {
                using var modeSelector = new ModeSelector();
                if (modeSelector.ShowDialog() != DialogResult.OK || modeSelector.SelectedMode == null) return;
                mode = modeSelector.SelectedMode.Value;
            }
            if (mode != WeaveMode.Installation || InstallationElevation.IsAdministrator()) break;
            if (InstallationElevation.TryLaunch(Application.ExecutablePath, request =>
            {
                using var child = Process.Start(request);
                return child != null;
            }, message => MessageBox.Show(message, "Installation", MessageBoxButtons.OK, MessageBoxIcon.Information))) return;
        }

        ReleaseType releaseType = ReleaseType.Stable;

        if (mode == WeaveMode.Installation)
        {
            using var releaseSelector = new ReleaseTypeSelector();
            if (releaseSelector.ShowDialog() != DialogResult.OK || releaseSelector.SelectedType == null)
            {
                Application.Exit();
                return;
            }
            releaseType = releaseSelector.SelectedType.Value;
        }
        else if (mode == WeaveMode.Projects)
        {
            using var projectsSelector = new ProjectsSelector();
            if (projectsSelector.ShowDialog() != DialogResult.OK || projectsSelector.SelectedMode == null)
            {
                Application.Exit();
                return;
            }
            mode = projectsSelector.SelectedMode.Value;
        }

        var mainWindow = new MainWindow(mode, releaseType);
        Application.Run(mainWindow);
    }
}
