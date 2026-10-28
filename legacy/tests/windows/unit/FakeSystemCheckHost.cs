using System.Diagnostics;
using System.Net;
using System.Runtime.InteropServices;
using Weave.UI.Pages;

namespace Weave.Windows.UnitTests;

internal sealed class FakeSystemCheckHost : SystemCheckHost
{
    internal Architecture ArchitectureKind { get; set; } = Architecture.X64;
    internal bool Administrator { get; set; } = true;
    internal bool SevenZipPresent { get; set; } = true;
    internal bool VswherePresent { get; set; } = true;
    internal bool MsvcPresent { get; set; } = true;
    internal bool InstallSevenZipFiles { get; set; } = true;
    internal bool InstallMsvcFiles { get; set; }
    internal bool SevenZipWaitCompleted { get; set; } = true;
    internal bool MsvcWaitCompleted { get; set; } = true;
    internal Task? MsvcLifetime { get; set; }
    internal Action? BeforeMsvcStart { get; set; }
    internal bool NullSevenZipProcess { get; set; }
    internal bool NullMsvcProcess { get; set; }
    internal bool InterruptDownload { get; set; }
    internal int WindowsBuildNumber { get; set; } = 26100;
    internal int SevenZipExitCode { get; set; }
    internal int VswhereExitCode { get; set; }
    internal int MsvcExitCode { get; set; }
    internal string? VswhereOutput { get; set; }
    internal string TemporaryDirectory { get; set; } = @"D:\Test User\Temp";
    internal HttpStatusCode DownloadStatus { get; set; } = HttpStatusCode.OK;
    internal byte[] DownloadBytes { get; set; } = ExecutableFixture();
    internal Exception? AdministratorError { get; set; }
    internal Exception? SevenZipProbeError { get; set; }
    internal Exception? DownloadError { get; set; }
    internal Exception? WriteError { get; set; }
    internal Exception? DeleteError { get; set; }
    internal Exception? SevenZipStartError { get; set; }
    internal Exception? VswhereStartError { get; set; }
    internal Exception? VswhereReadError { get; set; }
    internal Exception? MsvcStartError { get; set; }
    internal Exception? MsvcWaitError { get; set; }
    internal List<ProcessStartInfo> Processes { get; } = [];
    internal List<(string FileName, int? Milliseconds)> Waits { get; } = [];
    internal List<(string Url, TimeSpan Timeout)> Downloads { get; } = [];
    internal List<string> Writes { get; } = [];
    internal List<string> DeletedFiles { get; } = [];
    internal HashSet<string> Directories { get; } = new(StringComparer.OrdinalIgnoreCase);
    internal Dictionary<string, byte[]> Files { get; } = new(StringComparer.OrdinalIgnoreCase);

    internal override int PointerSize => ArchitectureKind is Architecture.X64 or Architecture.Arm64 ? 8 : 4;
    internal override Architecture ProcessArchitecture => ArchitectureKind;
    internal override int WindowsBuild => WindowsBuildNumber;
    internal override string TempDirectory => TemporaryDirectory;

    internal override bool IsAdministrator()
    {
        if (AdministratorError != null) throw AdministratorError;
        return Administrator;
    }

    internal override bool FileExists(string path)
    {
        path = Path.GetFullPath(path);
        if (Files.ContainsKey(path)) return true;
        if (path.EndsWith(@"\7-Zip\7z.exe", StringComparison.OrdinalIgnoreCase))
        {
            if (SevenZipProbeError != null) throw SevenZipProbeError;
            return SevenZipPresent;
        }
        if (path.EndsWith(@"\Installer\vswhere.exe", StringComparison.OrdinalIgnoreCase))
            return VswherePresent;
        if (path.EndsWith(@"\bin\Hostx64\x64\cl.exe", StringComparison.OrdinalIgnoreCase))
            return MsvcPresent;
        return Files.ContainsKey(path);
    }

    internal override bool DirectoryExists(string path) => Directories.Contains(Path.GetFullPath(path));
    internal override IEnumerable<string> EnumerateFiles(string path, string pattern) =>
        MsvcPresent ? [Path.Combine(path, @"14.50\bin\Hostx64\x64\cl.exe")] : [];

    internal override Stream CreateFile(string path)
    {
        if (WriteError != null) throw WriteError;
        if (Files.ContainsKey(path)) throw new IOException("File already exists");
        Writes.Add(path);
        return new RecordedStream(bytes => Files[path] = bytes);
    }

    internal override void DeleteFile(string path)
    {
        DeletedFiles.Add(path);
        if (DeleteError != null) throw DeleteError;
        Files.Remove(path);
    }

    internal override Task<HttpResponseMessage> DownloadAsync(string url, TimeSpan timeout)
    {
        Downloads.Add((url, timeout));
        if (DownloadError != null) return Task.FromException<HttpResponseMessage>(DownloadError);
        return Task.FromResult(new HttpResponseMessage(DownloadStatus)
        {
            Content = InterruptDownload ? new InterruptedContent() : new ByteArrayContent(DownloadBytes)
        });
    }

    internal override Task DelayAsync(TimeSpan duration) => Task.CompletedTask;

    internal override ISystemCheckProcess? StartProcess(ProcessStartInfo startInfo)
    {
        Processes.Add(startInfo);
        if (startInfo.FileName.EndsWith("7z-install.exe", StringComparison.OrdinalIgnoreCase))
        {
            if (SevenZipStartError != null) throw SevenZipStartError;
            if (NullSevenZipProcess) return null;
            return new FakeProcess(SevenZipExitCode, "", null, null,
                () => SevenZipPresent = InstallSevenZipFiles, SevenZipWaitCompleted,
                milliseconds => Waits.Add((startInfo.FileName, milliseconds)));
        }
        if (startInfo.FileName.EndsWith("vswhere.exe", StringComparison.OrdinalIgnoreCase))
        {
            if (VswhereStartError != null) throw VswhereStartError;
            return new FakeProcess(VswhereExitCode, VswhereOutput ?? (MsvcPresent ? @"C:\Program Files\Microsoft Visual Studio\2026\BuildTools\VC\Tools\MSVC\14.50\bin\Hostx64\x64\cl.exe" : ""),
                VswhereReadError, null, () => { }, true,
                milliseconds => Waits.Add((startInfo.FileName, milliseconds)));
        }
        if (startInfo.FileName == "winget")
        {
            BeforeMsvcStart?.Invoke();
            if (MsvcStartError != null) throw MsvcStartError;
            if (NullMsvcProcess) return null;
            return new FakeProcess(MsvcExitCode, "", null, MsvcWaitError,
                () => MsvcPresent = InstallMsvcFiles, MsvcWaitCompleted,
                milliseconds => Waits.Add((startInfo.FileName, milliseconds)), MsvcLifetime);
        }
        throw new InvalidOperationException("Unexpected process request: " + startInfo.FileName);
    }

    private sealed class FakeProcess(
        int exitCode, string output, Exception? readError, Exception? waitError,
        Action waited, bool waitCompleted, Action<int?> waitRequested, Task? lifetime = null) : ISystemCheckProcess
    {
        public bool HasExited => lifetime?.IsCompleted ?? waitCompleted;
        public int ExitCode => exitCode;
        public string ReadStandardOutput()
        {
            if (readError != null) throw readError;
            return output;
        }
        private void Wait(int? milliseconds)
        {
            waitRequested(milliseconds);
            if (waitError != null) throw waitError;
            waited();
        }
        public void WaitForExit() { lifetime?.GetAwaiter().GetResult(); Wait(null); }
        public bool WaitForExit(int milliseconds)
        {
            Wait(milliseconds);
            return waitCompleted;
        }
        public void Dispose() { }
        public void Terminate() { }
    }

    private static byte[] ExecutableFixture()
    {
        var bytes = new byte[512];
        using var stream = new MemoryStream(bytes);
        using var writer = new BinaryWriter(stream);
        writer.Write((ushort)0x5A4D);
        stream.Position = 0x3C;
        writer.Write(128);
        stream.Position = 128;
        writer.Write(0x00004550);
        writer.Write((ushort)0x8664);
        stream.Position = 148;
        writer.Write((ushort)240);
        writer.Write((ushort)2);
        writer.Write((ushort)0x20B);
        stream.Position = 212;
        writer.Write(512);
        stream.Position = 260;
        writer.Write(16);
        return bytes;
    }

    private sealed class RecordedStream(Action<byte[]> saved) : MemoryStream
    {
        protected override void Dispose(bool disposing)
        {
            if (disposing) saved(ToArray());
            base.Dispose(disposing);
        }
    }

    private sealed class InterruptedContent : HttpContent
    {
        protected override async Task SerializeToStreamAsync(Stream stream, TransportContext? context)
        {
            await stream.WriteAsync(new byte[] { 0x4D, 0x5A });
            throw new IOException("Download interrupted");
        }
        protected override bool TryComputeLength(out long length)
        {
            length = 0;
            return false;
        }
    }
}
