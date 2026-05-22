using System;
using System.Windows.Forms;
using Weave.Shared.Models;
using Weave.UI;

namespace Weave;

static class Program
{
    [STAThread]
    static void Main()
    {
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);

        var modeSelector = new ModeSelector();
        if (modeSelector.ShowDialog() != DialogResult.OK || modeSelector.SelectedMode == null)
        {
            Application.Exit();
            return;
        }

        var mode = modeSelector.SelectedMode.Value;
        ReleaseType releaseType = ReleaseType.Stable;

        if (mode == WeaveMode.Installation)
        {
            var releaseSelector = new ReleaseTypeSelector();
            if (releaseSelector.ShowDialog() != DialogResult.OK || releaseSelector.SelectedType == null)
            {
                Application.Exit();
                return;
            }
            releaseType = releaseSelector.SelectedType.Value;
        }
        else if (mode == WeaveMode.Projects)
        {
            var projectsSelector = new ProjectsSelector();
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
