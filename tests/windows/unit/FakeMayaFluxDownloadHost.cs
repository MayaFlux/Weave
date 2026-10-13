using System.Net;
using System.Text;
using System.Text.Json;
using Weave.UI.Pages;
using Weave.Utils;

namespace Weave.Windows.UnitTests;

internal sealed class FakeMayaFluxDownloadHost : MayaFluxDownloadHost
{
    internal const string AssetUrl = "https://github.com/MayaFlux/MayaFlux/releases/download/v0.5.0/MayaFlux-0.5.0-windows-x64.7z";
    internal const string InstallRoot = @"D:\Creative Tools\MayaFlux";
    internal const string TemporaryRoot = @"D:\Test User\Temp\Weave-Install";
    internal DateTime Now { get; set; } = new(2026, 10, 13, 12, 0, 0, DateTimeKind.Utc);
    internal string Metadata { get; set; } = Release();
    internal HttpStatusCode MetadataStatus { get; set; } = HttpStatusCode.OK;
    internal HttpStatusCode ArchiveStatus { get; set; } = HttpStatusCode.OK;
    internal byte[] ArchiveBytes { get; set; } = ArchiveFixture();
    internal long? DeclaredLength { get; set; }
    internal bool UnknownLength { get; set; }
    internal bool InterruptArchive { get; set; }
    internal bool ExtractionResult { get; set; } = true;
    internal bool VerificationResult { get; set; } = true;
    internal Exception? MetadataError { get; set; }
    internal Exception? ArchiveError { get; set; }
    internal Exception? WriteError { get; set; }
    internal Exception? DirectoryError { get; set; }
    internal Exception? VersionReadError { get; set; }
    internal Func<string, string, Exception?>? MoveError { get; set; }
    internal Func<string, Exception?>? DeleteDirectoryError { get; set; }
    internal List<(string Source, string Destination)> Moves { get; } = [];
    internal HashSet<string> ReparsePoints { get; } = new(StringComparer.OrdinalIgnoreCase);
    internal List<(string Url, string UserAgent)> Requests { get; } = [];
    internal List<string> CreatedDirectories { get; } = [];
    internal List<string> DeletedDirectories { get; } = [];
    internal List<string> WrittenArchives { get; } = [];
    internal List<(string Archive, string Destination)> Extractions { get; } = [];
    internal List<string> Verifications { get; } = [];
    internal HashSet<string> Directories { get; } = new(StringComparer.OrdinalIgnoreCase);
    internal Dictionary<string, byte[]> Files { get; } = new(StringComparer.OrdinalIgnoreCase);

    internal override DateTime UtcNow => Now;
    internal override HttpClient CreateHttpClient() => new(new Handler(this));
    internal override bool DirectoryExists(string path) => Directories.Contains(path);
    internal override bool FileExists(string path) => Files.ContainsKey(path);
    internal override bool IsReparsePoint(string path) => ReparsePoints.Contains(path);
    internal override void CreateDirectory(string path)
    {
        if (DirectoryError != null) throw DirectoryError;
        CreatedDirectories.Add(path);
        Directories.Add(path);
    }
    internal override void DeleteDirectory(string path)
    {
        var error = DeleteDirectoryError?.Invoke(path);
        if (error != null) throw error;
        DeletedDirectories.Add(path);
        Directories.Remove(path);
        foreach (var file in Files.Keys.Where(file => file.StartsWith(path.TrimEnd('\\') + "\\", StringComparison.OrdinalIgnoreCase)).ToArray())
            Files.Remove(file);
    }
    internal override void DeleteFile(string path) => Files.Remove(path);
    internal override void MoveDirectory(string source, string destination)
    {
        Moves.Add((source, destination));
        var error = MoveError?.Invoke(source, destination);
        if (error != null) throw error;
        if (!Directories.Contains(source) || Directories.Contains(destination)) throw new IOException("Cannot move directory");
        Directories.Remove(source);
        Directories.Add(destination);
        foreach (var file in Files.Keys.Where(file => file.StartsWith(source.TrimEnd('\\') + "\\", StringComparison.OrdinalIgnoreCase)).ToArray())
        {
            Files[destination + file[source.Length..]] = Files[file];
            Files.Remove(file);
        }
    }
    internal override Task<string> ReadAllTextAsync(string path) => VersionReadError == null
        ? Task.FromResult(Encoding.UTF8.GetString(Files[path]))
        : Task.FromException<string>(VersionReadError);
    internal override Stream CreateArchive(string path)
    {
        if (WriteError != null) throw WriteError;
        if (Files.ContainsKey(path)) throw new IOException("Archive already exists");
        WrittenArchives.Add(path);
        return new SavedStream(bytes => Files[path] = bytes);
    }
    internal override bool Extract7z(string archive, string destination, Logger? logger)
    {
        Extractions.Add((archive, destination));
        Files[Path.Combine(destination, "new-installation.txt")] = "replacement"u8.ToArray();
        return ExtractionResult;
    }
    internal override bool VerifyInstallation(string root, Logger? logger)
    {
        Verifications.Add(root);
        return VerificationResult;
    }
    internal void ExistingInstallation(string? versionJson = null)
    {
        Directories.Add(InstallRoot);
        Files[Path.Combine(InstallRoot, "keep.txt")] = "existing installation"u8.ToArray();
        if (versionJson != null)
            Files[Path.Combine(InstallRoot, "share", "MayaFlux", ".version")] = Encoding.UTF8.GetBytes(versionJson);
    }
    internal static string Release(string tag = "v0.5.0", string name = "MayaFlux-0.5.0-windows-x64.7z", string? url = null,
        bool draft = false, bool prerelease = false) => JsonSerializer.Serialize(new
        {
            tag_name = tag, draft, prerelease, published_at = "2026-10-13T10:00:00Z",
            assets = new[] { new { name, browser_download_url = url ?? $"https://github.com/MayaFlux/MayaFlux/releases/download/{tag}/{name}", size = 32, content_type = "application/x-7z-compressed" } }
        });
    internal static string Version(string version = "0.5.0", string buildTime = "2026-10-13 10:00:00 UTC") =>
        JsonSerializer.Serialize(new { version, build_time = buildTime });

    private sealed class Handler(FakeMayaFluxDownloadHost host) : HttpMessageHandler
    {
        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
        {
            var url = request.RequestUri!.AbsoluteUri;
            host.Requests.Add((url, request.Headers.UserAgent.ToString()));
            if (url.StartsWith("https://api.github.com/repos/MayaFlux/MayaFlux/releases", StringComparison.Ordinal))
            {
                if (host.MetadataError != null) return Task.FromException<HttpResponseMessage>(host.MetadataError);
                return Task.FromResult(new HttpResponseMessage(host.MetadataStatus) { Content = new StringContent(host.Metadata) });
            }
            if (host.ArchiveError != null) return Task.FromException<HttpResponseMessage>(host.ArchiveError);
            HttpContent content = host.InterruptArchive
                ? new StreamContent(new InterruptedStream(host.ArchiveBytes))
                : host.UnknownLength ? new UnknownLengthContent(host.ArchiveBytes) : new ByteArrayContent(host.ArchiveBytes);
            if (host.DeclaredLength.HasValue) content.Headers.ContentLength = host.DeclaredLength;
            return Task.FromResult(new HttpResponseMessage(host.ArchiveStatus) { Content = content });
        }
    }
    private static byte[] ArchiveFixture()
    {
        var bytes = new byte[32];
        new byte[] { 0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C, 0, 4 }.CopyTo(bytes, 0);
        return bytes;
    }
    private sealed class SavedStream(Action<byte[]> saved) : MemoryStream
    {
        protected override void Dispose(bool disposing)
        {
            if (disposing) saved(ToArray());
            base.Dispose(disposing);
        }
    }
    private sealed class InterruptedStream(byte[] bytes) : MemoryStream(bytes)
    {
        private bool read;
        public override Task<int> ReadAsync(byte[] buffer, int offset, int count, CancellationToken cancellationToken)
        {
            if (read) return Task.FromException<int>(new IOException("Archive stream interrupted"));
            read = true;
            return base.ReadAsync(buffer, offset, Math.Min(count, 4), cancellationToken);
        }
    }
    private sealed class UnknownLengthContent(byte[] bytes) : HttpContent
    {
        protected override Task SerializeToStreamAsync(Stream stream, TransportContext? context) => stream.WriteAsync(bytes).AsTask();
        protected override bool TryComputeLength(out long length) { length = 0; return false; }
    }
}
