using System.Diagnostics;
using Weave.UI.Pages;

namespace Weave.Utils;

internal class ArchiveExtractionHost : WindowsInstallationHost
{
    internal virtual string TempDirectory => Path.GetTempPath();
    internal virtual TimeSpan Timeout => TimeSpan.FromMinutes(15);
    internal virtual void CopyFile(string source, string destination) => File.Copy(source, destination, true);
    internal virtual void DeleteDirectory(string path) => Directory.Delete(path, true);
    internal virtual IDependencyProcess? StartProcess(ProcessStartInfo request)
    {
        var process = Process.Start(request);
        return process == null ? null : new WindowsInstallationProcess(process);
    }
}
