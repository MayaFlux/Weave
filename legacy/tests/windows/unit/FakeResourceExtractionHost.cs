using Weave.Utils;

namespace Weave.Windows.UnitTests;

internal sealed class FakeResourceExtractionHost : ResourceExtractionHost
{
    internal readonly Dictionary<string, byte[]?> Resources = new(StringComparer.Ordinal);
    internal readonly Dictionary<string, byte[]> Files = new(StringComparer.OrdinalIgnoreCase);
    internal readonly HashSet<string> Directories = new(StringComparer.OrdinalIgnoreCase);
    internal readonly HashSet<string> ReparsePaths = new(StringComparer.OrdinalIgnoreCase);
    internal string? FailedCreate;
    internal string? FailedWrite;
    internal string? FailedMove;
    internal bool FailDelete;
    internal bool FailEnumeration;
    internal string? FailedResourceRead;
    internal readonly List<string> Mutations = [];
    internal FakeResourceExtractionHost()
    {
        var real = new ResourceExtractionHost();
        foreach (var name in real.ResourceNames)
        {
            using var stream = real.OpenResource(name);
            using var copy = new MemoryStream();
            stream!.CopyTo(copy);
            Resources[name] = copy.ToArray();
        }
        Directories.Add(@"D:\Maya Flux");
    }
    internal override string[] ResourceNames => Resources.Keys.ToArray();
    internal override Stream? OpenResource(string name) => name == FailedResourceRead ? new UnreadableStream() : Resources[name] is { } bytes ? new MemoryStream(bytes, false) : null;
    internal override bool DirectoryExists(string path) => Directories.Contains(path);
    internal override bool FileExists(string path) => Files.ContainsKey(path);
    internal override long FileLength(string path) => Files[path].Length;
    internal override FileAttributes GetAttributes(string path) => ReparsePaths.Contains(path) ? FileAttributes.ReparsePoint : FileAttributes.Directory;
    internal override void CreateDirectory(string path)
    {
        Mutations.Add(path);
        if (path.Contains(FailedCreate ?? "\0", StringComparison.OrdinalIgnoreCase)) throw new UnauthorizedAccessException("Directory creation denied");
        for (string? current = path; current != null; current = Path.GetDirectoryName(current)) Directories.Add(current);
    }
    internal override Stream CreateFile(string path)
    {
        Mutations.Add(path);
        return new DestinationStream(bytes => Files[path] = bytes, path.Contains(FailedWrite ?? "\0", StringComparison.OrdinalIgnoreCase));
    }
    internal override string[] GetFiles(string path, string pattern, SearchOption option)
    {
        if (FailEnumeration) throw new UnauthorizedAccessException("File enumeration denied");
        return Files.Keys.Where(file => file.StartsWith(path + "\\", StringComparison.OrdinalIgnoreCase) && (pattern == "*" || file.EndsWith(".ps1", StringComparison.OrdinalIgnoreCase))).ToArray();
    }
    internal override void MoveFile(string source, string destination)
    {
        Mutations.Add(destination);
        if (destination.Contains(FailedMove ?? "\0", StringComparison.OrdinalIgnoreCase)) throw new IOException("File replacement denied");
        Files[destination] = Files[source];
        Files.Remove(source);
    }
    internal override void DeleteFile(string path)
    {
        if (FailDelete) throw new IOException("Temporary cleanup denied");
        Files.Remove(path);
    }
    private sealed class DestinationStream(Action<byte[]> save, bool fail) : MemoryStream
    {
        public override void Write(byte[] buffer, int offset, int count)
        {
            if (fail) throw new IOException("Resource write failed");
            base.Write(buffer, offset, count);
        }
        protected override void Dispose(bool disposing)
        {
            if (disposing) save(ToArray());
            base.Dispose(disposing);
        }
    }
    private sealed class UnreadableStream : MemoryStream
    {
        public override void CopyTo(Stream destination, int bufferSize) => throw new IOException("Packaged stream read failed");
    }
}
