using System.Diagnostics;
using Weave.Utils;

namespace Weave.UI.Pages;

internal class DependenciesHost : WindowsInstallationHost
{
    internal virtual TimeSpan ProcessTimeout => TimeSpan.FromSeconds(30);
    internal virtual TimeSpan InstallTimeout => TimeSpan.FromHours(1);
    internal virtual IDependencyProcess? StartProcess(ProcessStartInfo startInfo)
    {
        var process = Process.Start(startInfo);
        return process == null ? null : new WindowsInstallationProcess(process);
    }
}

internal interface IDependencyProcess : IDisposable
{
    int ExitCode { get; }
    bool HasExited { get; }
    void Terminate();
    string ReadStandardOutput();
    Task<string> ReadStandardOutputAsync();
    Task<string> ReadStandardErrorAsync();
    void WaitForExit();
    Task WaitForExitAsync(CancellationToken cancellationToken = default);
}
