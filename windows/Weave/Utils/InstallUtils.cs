using System.Diagnostics;
using System.Drawing;
using System.Windows.Forms;
using Weave.UI.Pages;

namespace Weave.Utils;

internal class WindowsInstallationHost
{
    internal virtual bool FileExists(string path) => File.Exists(path);
    internal virtual bool DirectoryExists(string path) => Directory.Exists(path);
    internal virtual void CreateDirectory(string path) => Directory.CreateDirectory(path);
    internal virtual void DeleteFile(string path) => File.Delete(path);
    internal virtual void MoveDirectory(string source, string destination) => Directory.Move(source, destination);
    internal virtual string[] GetDirectories(string path, string pattern = "*") => Directory.GetDirectories(path, pattern);
    internal virtual string[] GetFileSystemEntries(string path) => Directory.GetFileSystemEntries(path);
    internal virtual FileAttributes GetAttributes(string path) => File.GetAttributes(path);
    internal virtual bool IsReparsePoint(string path) =>
        (DirectoryExists(path) || FileExists(path)) && (GetAttributes(path) & FileAttributes.ReparsePoint) != 0;
    internal virtual string? GetEnvironmentVariable(string name, EnvironmentVariableTarget target) => Environment.GetEnvironmentVariable(name, target);
    internal virtual void SetEnvironmentVariable(string name, string value, EnvironmentVariableTarget target) => Environment.SetEnvironmentVariable(name, value, target);
}

internal sealed class WindowsInstallationProcess : ISystemCheckProcess, IDependencyProcess
{
    private readonly Process process;
    private readonly Task<string> output;
    private readonly Task<string> error;

    internal WindowsInstallationProcess(Process process, bool captureError = true)
    {
        this.process = process;
        output = process.StartInfo.RedirectStandardOutput ? process.StandardOutput.ReadToEndAsync() : Task.FromResult("");
        error = !process.StartInfo.RedirectStandardError ? Task.FromResult("") :
            captureError ? process.StandardError.ReadToEndAsync() : DrainErrorAsync(process);
    }
    private static async Task<string> DrainErrorAsync(Process process)
    {
        await process.StandardError.BaseStream.CopyToAsync(Stream.Null);
        return "";
    }
    public int ExitCode => process.ExitCode;
    public bool HasExited => process.HasExited;
    public string ReadStandardOutput()
    {
        if (!output.Wait(TimeSpan.FromSeconds(5))) throw new TimeoutException("Process output timed out");
        return output.GetAwaiter().GetResult();
    }
    public Task<string> ReadStandardOutputAsync() => output;
    public Task<string> ReadStandardErrorAsync() => error;
    public void WaitForExit() => process.WaitForExit();
    public bool WaitForExit(int milliseconds) => process.WaitForExit(milliseconds);
    public Task WaitForExitAsync(CancellationToken cancellationToken = default) => process.WaitForExitAsync(cancellationToken);
    public void Terminate() => process.Kill(entireProcessTree: true);
    public void Dispose() => process.Dispose();
}

internal static class WindowsInstallationPaths
{
    internal static bool IsSafeEnvironmentPath(string path) => !string.IsNullOrWhiteSpace(path) &&
        Path.IsPathFullyQualified(path) && !path.Any(c => char.IsControl(c) || c is ';' or '"' or '<' or '>' or '|' or '*' or '?');

    internal static IEnumerable<string> OrderVersionDirectories(IEnumerable<string> paths) => paths
        .Select(path => new { Path = path, Version = Version.TryParse(Path.GetFileName(path), out var version) ? version : null })
        .Where(entry => entry.Version != null).OrderByDescending(entry => entry.Version).Select(entry => entry.Path);

    internal static void EnsureNoReparseAncestors(string path, WindowsInstallationHost host)
    {
        for (string? current = path; current != null; current = Path.GetDirectoryName(current))
            if (host.IsReparsePoint(current)) throw new IOException($"Installation path crosses a reparse point: {current}");
    }
}

internal static class InstallationStepUi
{
    internal static Task AppendLogAsync(TextBox? logBox, string message, bool refresh = false)
    {
        if (logBox?.Parent != null)
            logBox.Invoke(new Action(() =>
            {
                logBox.AppendText(message + Environment.NewLine);
                if (refresh) logBox.Refresh();
            }));
        return Task.CompletedTask;
    }

    internal static void UpdateStatus(Label? label, string text, Color color)
    {
        if (label?.Parent != null)
            label.Invoke(new Action(() =>
            {
                label.Text = text;
                label.ForeColor = color;
            }));
    }
}
