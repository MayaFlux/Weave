using System.Runtime.CompilerServices;
using System.Text.Json;
using NUnit.Framework;
using Weave.Shared;

namespace Weave.Windows.UnitTests;

[TestFixture]
public class CommunityManifestVectorTests
{
    private static string VectorDirectory([CallerFilePath] string source = "") =>
        Path.GetFullPath(Path.Combine(Path.GetDirectoryName(source)!, "..", "..", "vectors", "community"));

    private static IEnumerable<string> Files() =>
        Directory.Exists(VectorDirectory())
            ? Directory.GetFiles(VectorDirectory(), "*.json").OrderBy(path => path, StringComparer.Ordinal)
            : [];

    public static IEnumerable<TestCaseData> Vectors() =>
        Files().Select(path => new TestCaseData(path).SetName($"Vector_{Path.GetFileNameWithoutExtension(path)}"));

    [Test]
    public void CorpusIsPresent()
    {
        var names = Files().Select(Path.GetFileName).ToArray();

        Assert.That(names.Any(name => name!.StartsWith("valid-")), Is.True, VectorDirectory());
        Assert.That(names.Any(name => name!.StartsWith("invalid-")), Is.True, VectorDirectory());
        Assert.That(names.All(name => name!.StartsWith("valid-") || name.StartsWith("invalid-")), Is.True);
    }

    [TestCaseSource(nameof(Vectors))]
    public void VectorIsAcceptedOrRejectedAsNamed(string path)
    {
        var source = File.ReadAllText(path);
        using var document = JsonDocument.Parse(source);
        var name = document.RootElement.GetProperty("name").GetString();

        if (Path.GetFileName(path).StartsWith("valid-"))
            Assert.DoesNotThrow(() => CommunityManifest.Parse(source, name));
        else
            Assert.Throws<ManifestException>(() => CommunityManifest.Parse(source, name));
    }
}
