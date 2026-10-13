using System.Diagnostics;
using Weave.Utils;
using System.Net.Http;
using System.Runtime.InteropServices;
using System.Security.Principal;

namespace Weave.UI.Pages;

internal class SystemCheckHost : WindowsInstallationHost
{
    internal virtual int PointerSize => IntPtr.Size;
    internal virtual Architecture ProcessArchitecture => RuntimeInformation.ProcessArchitecture;
    internal virtual int WindowsBuild => Environment.OSVersion.Version.Build;
    internal virtual string TempDirectory => Path.GetTempPath();
    internal virtual IEnumerable<string> EnumerateFiles(string path, string pattern) =>
        Directory.Exists(path) ? Directory.EnumerateFiles(path, pattern, SearchOption.AllDirectories) : [];
    internal virtual Stream CreateFile(string path) => new FileStream(path, FileMode.CreateNew, FileAccess.ReadWrite, FileShare.None);
    internal virtual Task DelayAsync(TimeSpan duration) => Task.Delay(duration);

    internal virtual bool IsAdministrator()
    {
        using var identity = WindowsIdentity.GetCurrent();
        return new WindowsPrincipal(identity).IsInRole(WindowsBuiltInRole.Administrator);
    }

    internal virtual async Task<HttpResponseMessage> DownloadAsync(string url, TimeSpan timeout)
    {
        using var client = new HttpClient { Timeout = timeout };
        return await client.GetAsync(url);
    }

    internal virtual ISystemCheckProcess? StartProcess(ProcessStartInfo startInfo)
    {
        var process = Process.Start(startInfo);
        return process == null ? null : new WindowsInstallationProcess(process, captureError: false);
    }

}
internal interface ISystemCheckProcess : IDisposable
{
    int ExitCode { get; }
    string ReadStandardOutput();
    void WaitForExit();
    bool WaitForExit(int milliseconds);
    void Terminate();
}
