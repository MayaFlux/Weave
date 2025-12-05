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
        var mainWindow = new MainWindow(mode);
        Application.Run(mainWindow);
    }
}
