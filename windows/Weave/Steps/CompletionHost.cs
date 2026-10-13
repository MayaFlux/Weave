using System.Diagnostics;

namespace Weave.UI.Pages;

internal class CompletionHost
{
    internal virtual string LogPath => Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "weave_install.log");
    internal virtual bool FileExists(string path) => File.Exists(path);
    internal virtual bool StartProcess(ProcessStartInfo info)
    {
        using var process = Process.Start(info);
        return process != null;
    }
}
