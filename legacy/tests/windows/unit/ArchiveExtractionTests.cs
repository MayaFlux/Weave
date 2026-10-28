using System.Diagnostics;
using System.Text;
using NUnit.Framework;
using Weave.UI.Pages;
using Weave.Utils;

namespace Weave.Windows.UnitTests;

[TestFixture]
public class ArchiveExtractionTests
{
    [TestCase(false)]
    [TestCase(true)]
    public void NativeRedirectedProcessHandlesPipePressureAndTerminatesStalls(bool stalled)
    {
        using var directory = new TestDirectory();
        var host = new NativeHost(directory.PathName, stalled);
        var archive = Path.Combine(directory.PathName, "fixture.7z");
        File.WriteAllText(archive, "inert fixture");
        var destination = Path.Combine(directory.PathName, "stage");
        try
        {
            Assert.That(FileOperations.Extract7z(archive, destination, new Logger("NUL"), host), Is.EqualTo(!stalled));
            Assert.That(Directory.GetDirectories(directory.PathName, "weave_extract_*"), Is.Empty);
            Assert.That(File.Exists(Path.Combine(destination, "payload.txt")), Is.EqualTo(!stalled));
            using var process = FindProcess(host.ProcessId);
            Assert.That(process == null || process.WaitForExit(2000), Is.True, "The owned fixture process must finish or be terminated");
        }
        finally
        {
            using var remaining = FindProcess(host.ProcessId);
            if (remaining != null && !remaining.HasExited) { remaining.Kill(true); remaining.WaitForExit(2000); }
        }
    }

    private static Process? FindProcess(int id)
    {
        if (id == 0) return null;
        try { return Process.GetProcessById(id); }
        catch (ArgumentException) { return null; }
    }

    private sealed class NativeHost(string temporary, bool stalled) : ArchiveExtractionHost
    {
        internal override string TempDirectory => temporary;
        internal override TimeSpan Timeout => TimeSpan.FromSeconds(stalled ? 1 : 10);
        internal int ProcessId;
        internal override bool FileExists(string path) => path.EndsWith(@"\7-Zip\7z.exe") || base.FileExists(path);
        internal override IDependencyProcess? StartProcess(ProcessStartInfo request)
        {
            var output = request.ArgumentList.Single(argument => argument.StartsWith("-o"))[2..];
            var payload = Path.Combine(output, "dist_staging");
            Directory.CreateDirectory(payload);
            File.WriteAllText(Path.Combine(payload, "payload.txt"), "inert package fixture");
            var script = "[Console]::Out.Write(('o' * 524288)); [Console]::Error.Write(('e' * 524288));" + (stalled ? "[Threading.Thread]::Sleep(60000)" : "exit 0");
            var fixture = new ProcessStartInfo("powershell.exe") { UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true, CreateNoWindow = true };
            foreach (var argument in new[] { "-NoProfile", "-NonInteractive", "-EncodedCommand", Convert.ToBase64String(Encoding.Unicode.GetBytes(script)) }) fixture.ArgumentList.Add(argument);
            var process = Process.Start(fixture)!;
            ProcessId = process.Id;
            return new WindowsInstallationProcess(process);
        }
    }

    [TestCase("success")]
    [TestCase("exit")]
    [TestCase("timeout")]
    [TestCase("linked")]
    public void ControlledExtractionUsesBoundedProcessAndCleansOwnedTemporaryTree(string scenario)
    {
        var host = new Host(scenario);
        var logger = new Logger("NUL");
        var success = FileOperations.Extract7z(@"D:\Downloads\Maya Flux.7z", @"D:\Install\stage", logger, host);
        Assert.Multiple(() =>
        {
            Assert.That(success, Is.EqualTo(scenario == "success"));
            Assert.That(host.Request, Is.Not.Null);
            Assert.That(host.Request!.RedirectStandardOutput && host.Request.RedirectStandardError && !host.Request.UseShellExecute, Is.True);
            Assert.That(host.Process.Bounded, Is.True);
            Assert.That(host.Deleted, Has.Count.EqualTo(1));
            Assert.That(host.Deleted.Single(), Does.StartWith(@"D:\Temporary Files\weave_extract_"));
            Assert.That(host.Copied, Has.Count.EqualTo(scenario == "success" ? 1 : 0));
            Assert.That(host.Process.Killed, Is.EqualTo(scenario == "timeout"));
            if (scenario != "success") Assert.That(logger.GetLogs(), Has.Some.Contains("Retry"));
            if (scenario == "exit") Assert.That(logger.GetLogs(), Has.Some.Contains("output").And.Some.Contains("fixture diagnostic"));
        });
    }

    private sealed class Host(string scenario) : ArchiveExtractionHost
    {
        internal override string TempDirectory => @"D:\Temporary Files";
        internal override TimeSpan Timeout => TimeSpan.FromMilliseconds(30);
        internal ProcessStartInfo? Request;
        internal ProcessFixture Process { get; } = new(scenario);
        internal List<string> Deleted { get; } = [];
        internal List<string> Copied { get; } = [];
        internal HashSet<string> Directories { get; } = new(StringComparer.OrdinalIgnoreCase);
        private string? extraction;
        internal override bool FileExists(string path) => path.EndsWith(@"\7-Zip\7z.exe") || path.EndsWith("MayaFluxLib.lib");
        internal override bool DirectoryExists(string path) => Directories.Contains(path);
        internal override string? GetEnvironmentVariable(string name, EnvironmentVariableTarget target) => null;
        internal override FileAttributes GetAttributes(string path) => (DirectoryExists(path) ? FileAttributes.Directory : FileAttributes.Normal) | (scenario == "linked" && path.EndsWith("MayaFluxLib.lib") ? FileAttributes.ReparsePoint : 0);
        internal override void CreateDirectory(string path)
        {
            Directories.Add(path);
            if (path.Contains("weave_extract_")) extraction = path;
        }
        internal override IDependencyProcess? StartProcess(ProcessStartInfo request)
        {
            Request = request;
            Directories.Add(Path.Combine(extraction!, "dist_staging"));
            return Process;
        }
        internal override string[] GetFileSystemEntries(string path) => path.EndsWith("dist_staging") ? [Path.Combine(path, "MayaFluxLib.lib")] : [];
        internal override void CopyFile(string source, string destination) => Copied.Add(destination);
        internal override void DeleteDirectory(string path) => Deleted.Add(path);
    }

    private sealed class ProcessFixture(string scenario) : IDependencyProcess
    {
        internal bool Bounded;
        internal bool Killed;
        public int ExitCode => scenario == "exit" ? 2 : 0;
        public bool HasExited => scenario != "timeout" || Killed;
        public void Terminate() => Killed = true;
        public void WaitForExit() => throw new InvalidOperationException("Unbounded wait requested");
        public Task WaitForExitAsync(CancellationToken cancellationToken = default)
        {
            Bounded = cancellationToken.CanBeCanceled;
            return scenario == "timeout" ? Task.Delay(Timeout.Infinite, cancellationToken) : Task.CompletedTask;
        }
        public string ReadStandardOutput() => "output";
        public Task<string> ReadStandardOutputAsync() => Task.FromResult("output");
        public Task<string> ReadStandardErrorAsync() => Task.FromResult("fixture diagnostic");
        public void Dispose() { }
    }
}
