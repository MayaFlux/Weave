using System.IO.Compression;
using System.Reflection;
using System.Reflection.PortableExecutable;
using System.Runtime.Loader;
using NUnit.Framework;

namespace Weave.Windows.IntegrationTests;

internal sealed class PublishedBundle : IDisposable
{
    private sealed record Entry(long Offset, long Size, long CompressedSize);
    private readonly byte[] bytes;
    private readonly Dictionary<string, Entry> entries = new(StringComparer.Ordinal);
    private readonly AssemblyLoadContext context = new("Published Weave", isCollectible: true);
    internal string Root { get; } = Path.Combine(TestContext.CurrentContext.TestDirectory, "packaged fixtures", Guid.NewGuid().ToString("N"));
    internal string Executable { get; }
    internal Assembly ApplicationAssembly { get; }
    internal Assembly SharedAssembly { get; }
    internal IEnumerable<string> Names => entries.Keys;

    internal PublishedBundle()
    {
        var zipPath = Environment.GetEnvironmentVariable("WEAVE_WINDOWS_ZIP");
        Assert.That(zipPath, Is.Not.Null.And.Not.Empty, "Set WEAVE_WINDOWS_ZIP to the published distribution ZIP.");
        using var zip = ZipFile.OpenRead(zipPath!);
        Assert.That(zip.Entries.Select(entry => entry.FullName), Is.EqualTo(new[] { "Weave.exe" }), "The Windows distribution contains one executable at the ZIP root.");
        Directory.CreateDirectory(Root);
        Executable = Path.Combine(Root, "Weave.exe");
        zip.Entries.Single().ExtractToFile(Executable);
        bytes = File.ReadAllBytes(Executable);
        ReadManifest();
        context.Resolving += (_, name) => entries.ContainsKey(name.Name + ".dll") ? Load(name.Name + ".dll") : null;
        SharedAssembly = Load("Shared.dll");
        ApplicationAssembly = Load("Weave.dll");
    }

    private Assembly Load(string name) => context.LoadFromStream(new MemoryStream(Read(name), writable: false));

    private void ReadManifest()
    {
        byte[] signature = [0x8b, 0x12, 0x02, 0xb9, 0x6a, 0x61, 0x20, 0x38, 0x72, 0x7b, 0x93, 0x02, 0x14, 0xd7, 0xa0, 0x32, 0x13, 0xf5, 0xb9, 0xe6, 0xef, 0xae, 0x33, 0x18, 0xee, 0x3b, 0x2d, 0xce, 0x24, 0xb3, 0x6a, 0xae];
        var marker = bytes.AsSpan().IndexOf(signature);
        Assert.That(marker, Is.GreaterThanOrEqualTo(8), "Missing .NET single-file bundle signature.");
        var offset = BitConverter.ToInt64(bytes, marker - 8);
        Assert.That(offset, Is.InRange(1L, bytes.LongLength - 12));
        using var reader = new BinaryReader(new MemoryStream(bytes, writable: false));
        reader.BaseStream.Position = offset;
        var major = reader.ReadUInt32();
        Assert.That(major, Is.EqualTo(6), "Expected the .NET 8 bundle format.");
        Assert.That(reader.ReadUInt32(), Is.Zero);
        var count = reader.ReadInt32();
        Assert.That(count, Is.InRange(1, 10000));
        reader.ReadString();
        reader.BaseStream.Position += 40;
        for (var index = 0; index < count; index++)
        {
            var entry = new Entry(reader.ReadInt64(), reader.ReadInt64(), reader.ReadInt64());
            reader.ReadByte();
            var name = reader.ReadString();
            Assert.That(entry.Offset, Is.InRange(1L, bytes.LongLength - 1), name);
            Assert.That(entry.Size, Is.InRange(1L, int.MaxValue), name);
            Assert.That(entry.CompressedSize, Is.GreaterThanOrEqualTo(0), name);
            Assert.That(entry.Offset + (entry.CompressedSize == 0 ? entry.Size : entry.CompressedSize), Is.LessThanOrEqualTo(offset), name);
            Assert.That(entries.TryAdd(name, entry), Is.True, "Duplicate bundle entry: " + name);
        }
    }

    internal byte[] Read(string name)
    {
        Assert.That(entries.ContainsKey(name), Is.True, "Missing packaged dependency: " + name);
        var entry = entries[name];
        if (entry.CompressedSize == 0) return bytes.AsSpan((int)entry.Offset, (int)entry.Size).ToArray();
        using var input = new MemoryStream(bytes, (int)entry.Offset, (int)entry.CompressedSize, writable: false);
        using var compressed = new DeflateStream(input, CompressionMode.Decompress);
        using var output = new MemoryStream();
        compressed.CopyTo(output);
        Assert.That(output.Length, Is.EqualTo(entry.Size), name);
        return output.ToArray();
    }

    internal string PrepareInstallation()
    {
        var root = Path.Combine(Root, "Maya Flux fixture");
        ApplicationAssembly.GetType("Weave.Utils.ResourceExtractor", throwOnError: true)!
            .GetMethod("ExtractTemplates", [typeof(string)])!.Invoke(null, [root]);
        return root;
    }

    internal PEReader ReadExecutable() => new(new MemoryStream(bytes, writable: false));

    public void Dispose()
    {
        context.Unload();
        var parent = Path.GetFullPath(TestContext.CurrentContext.TestDirectory).TrimEnd(Path.DirectorySeparatorChar) + Path.DirectorySeparatorChar;
        if (!Path.GetFullPath(Root).StartsWith(parent, StringComparison.OrdinalIgnoreCase))
            throw new IOException("Packaged fixture cleanup must remain inside the test directory.");
        if (Directory.Exists(Root)) Directory.Delete(Root, recursive: true);
    }
}
