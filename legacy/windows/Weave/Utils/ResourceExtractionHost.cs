namespace Weave.Utils;

internal class ResourceExtractionHost : WindowsInstallationHost
{
    internal virtual string[] ResourceNames => typeof(ResourceExtractor).Assembly.GetManifestResourceNames();
    internal virtual Stream? OpenResource(string name) => typeof(ResourceExtractor).Assembly.GetManifestResourceStream(name);
    internal virtual long FileLength(string path) => new FileInfo(path).Length;
    internal virtual Stream CreateFile(string path) => new FileStream(path, FileMode.CreateNew, FileAccess.Write, FileShare.None);
    internal virtual string[] GetFiles(string path, string pattern, SearchOption option) => Directory.GetFiles(path, pattern, option);
    internal virtual void MoveFile(string source, string destination) => File.Move(source, destination, true);
}
