using Weave.Utils;

namespace Weave.UI.Pages;

internal class MayaFluxDownloadHost : WindowsInstallationHost
{
    internal Action<string>? Report { get; set; }
    internal virtual DateTime UtcNow => DateTime.UtcNow;
    internal virtual HttpClient CreateHttpClient() => new();
    internal virtual void DeleteDirectory(string path) => Directory.Delete(path, recursive: true);
    internal virtual Task<string> ReadAllTextAsync(string path) => File.ReadAllTextAsync(path);
    internal virtual Stream CreateArchive(string path) => new FileStream(path, FileMode.CreateNew, FileAccess.Write, FileShare.None, 8192, true);
    internal virtual void CopyFile(string source, string destination) => File.Copy(source, destination, overwrite: true);

    internal void InstallFiles(string source, string destination)
    {
        MayaFluxReleaseValidation.ValidateDirectory(source, this);
        MayaFluxReleaseValidation.ValidateDirectory(destination, this);
        if (!DirectoryExists(source) || MayaFluxReleaseValidation.ContainsPath(source, destination) || MayaFluxReleaseValidation.ContainsPath(destination, source))
            throw new IOException("The extracted package must exist outside the installation directory.");
        if (DirectoryExists(destination)) DeleteDirectory(destination);
        var files = new List<(string Source, string Destination)>();
        var pending = new Stack<(string Source, string Destination)>();
        pending.Push((source, destination));
        while (pending.Count > 0)
        {
            var directory = pending.Pop();
            WindowsInstallationPaths.EnsureNoReparseAncestors(directory.Source, this);
            WindowsInstallationPaths.EnsureNoReparseAncestors(directory.Destination, this);
            CreateDirectory(directory.Destination);
            foreach (var entry in GetFileSystemEntries(directory.Source))
            {
                WindowsInstallationPaths.EnsureNoReparseAncestors(entry, this);
                var target = Path.Combine(directory.Destination, Path.GetFileName(entry));
                if (DirectoryExists(entry)) pending.Push((entry, target));
                else files.Add((entry, target));
            }
        }
        var versionFile = Path.Combine(destination, "share", "MayaFlux", ".version");
        foreach (var file in files.OrderBy(file => file.Destination.Equals(versionFile, StringComparison.OrdinalIgnoreCase)))
        {
            WindowsInstallationPaths.EnsureNoReparseAncestors(file.Destination, this);
            try { CopyFile(file.Source, file.Destination); }
            catch (Exception error) { throw new IOException($"Could not replace MayaFlux file {file.Destination}: {error.Message}", error); }
        }
    }
    internal virtual bool Extract7z(string archive, string destination, Logger? logger) => FileOperations.Extract7z(archive, destination, logger!, new ArchiveExtractionHost(), Report);
    internal virtual bool VerifyInstallation(string root, Logger? logger) => FileOperations.VerifyInstallation(root, logger!, Report);
}
