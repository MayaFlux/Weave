using Weave.Utils;

namespace Weave.UI.Pages;

internal class MayaFluxDownloadHost : WindowsInstallationHost
{
    internal virtual DateTime UtcNow => DateTime.UtcNow;
    internal virtual HttpClient CreateHttpClient() => new();
    internal virtual void DeleteDirectory(string path) => Directory.Delete(path, recursive: true);
    internal virtual Task<string> ReadAllTextAsync(string path) => File.ReadAllTextAsync(path);
    internal virtual Stream CreateArchive(string path) => new FileStream(path, FileMode.CreateNew, FileAccess.Write, FileShare.None, 8192, true);
    internal virtual bool Extract7z(string archive, string destination, Logger? logger) => FileOperations.Extract7z(archive, destination, logger!);
    internal virtual bool VerifyInstallation(string root, Logger? logger) => FileOperations.VerifyInstallation(root, logger!);
}
