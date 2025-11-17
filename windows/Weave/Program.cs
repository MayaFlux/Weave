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

        var modeDetector = new ModeDetector();
        var mode = modeDetector.DetectMode();

        var mainWindow = new MainWindow(mode);
        Application.Run(mainWindow);
    }
}