using System.Text;
using System.Text.Json;
using NUnit.Framework;
using Weave.UI.Project;
using Weave.Utils;

namespace Weave.Windows.UnitTests;

[TestFixture]
public class ProjectCreationTests
{
    private const string Root = @"D:\Maya Flux";
    private const string Templates = Root + @"\share\weave\templates";
    private const string Parent = @"E:\Creative Projects";
    private const string Project = Parent + @"\MyProject";
    private FakeProjectCreationHost host = null!;
    private List<string> messages = null!;
    private ProjectCreationService service = null!;

    [SetUp]
    public void Setup()
    {
        host = new FakeProjectCreationHost();
        var resources = new FakeResourceExtractionHost();
        ResourceExtractor.ExtractTemplates(Root, resources);
        foreach (var file in resources.Files) host.Files[file.Key] = Encoding.UTF8.GetString(file.Value);
        foreach (var directory in resources.Directories) host.Directories.Add(directory);
        host.Directories.Add(Parent);
        host.Root = Root;
        messages = [];
        service = new ProjectCreationService(Templates, host, messages.Add);
    }

    [TestCase(false, true)]
    [TestCase(true, false)]
    public void CreatesPortableProjectFromPackagedTemplates(bool lila, bool vscode)
    {
        Assert.That(service.Create("MyProject", Parent, lila, vscode), Is.EqualTo(Project));
        Assert.Multiple(() =>
        {
            Assert.That(host.Files[Project + @"\src\main.cpp"], Is.EqualTo(host.Files[Templates + @"\main.cpp"]));
            Assert.That(host.Files[Project + @"\src\user_project.hpp"], Is.EqualTo(host.Files[Templates + @"\user_project.hpp"]));
            Assert.That(host.Files[Project + @"\validate.ps1"], Is.EqualTo(host.Files[Templates + @"\validate.ps1"]));
            Assert.That(host.Files[Project + @"\run-report.ps1"], Does.Contain("$ProjectName = 'MyProject'").And.Not.Contain("@PROJECT_NAME@"));
            Assert.That(host.Files.ContainsKey(Project + @"\validate.bat"), Is.True);
            Assert.That(host.Files.ContainsKey(Project + @"\run-report.bat"), Is.True);
            Assert.That(host.Files[Project + @"\CMakeLists.txt"], Does.Contain("project(MyProject"));
            Assert.That(host.Files[Project + @"\CMakeLists.txt"], Does.Contain("GLOB_RECURSE").And.Contain("/include/*.tpp"));
            Assert.That(host.Files[Project + @"\CMakePresets.json"], Is.EqualTo(host.Files[Templates + @"\CMakePresets.json"]));
            Assert.That(host.Files[Project + @"\community.cmake"], Is.Empty);
            Assert.That(host.Files[Project + @"\cmake\mayaflux.cmake"].Contains("target_link_libraries(${PROJECT_NAME} PRIVATE MayaFlux::MayaFluxHost)"), Is.EqualTo(lila));
            Assert.That(host.Files.ContainsKey(Project + @"\.vscode\tasks.json"), Is.EqualTo(vscode));
            Assert.That(host.Files.Where(file => file.Key.StartsWith(Project + "\\")).Any(file => file.Value.Contains("@PROJECT_NAME@") || file.Value.Contains("@LILA_")), Is.False);
            Assert.That(host.Files[Project + @"\README.md"], Does.Contain("run this manually").And.Contain("cmake --preset release").And.Contain(".\\validate.bat").And.Contain(".\\run-report.bat"));
        });
        using var presets = JsonDocument.Parse(host.Files[Project + @"\CMakePresets.json"]);
        var configure = presets.RootElement.GetProperty("configurePresets").EnumerateArray().ToDictionary(preset => preset.GetProperty("name").GetString()!);
        foreach (var build in presets.RootElement.GetProperty("buildPresets").EnumerateArray())
            Assert.That(configure.ContainsKey(build.GetProperty("configurePreset").GetString()!), Is.True);
        Assert.That(configure["release"].GetProperty("cacheVariables").GetProperty("CMAKE_BUILD_TYPE").GetString(), Is.EqualTo("Release"));
        Assert.That(configure["debug"].GetProperty("cacheVariables").GetProperty("CMAKE_BUILD_TYPE").GetString(), Is.EqualTo("Debug"));
        if (vscode)
        {
            using var tasks = JsonDocument.Parse(host.Files[Project + @"\.vscode\tasks.json"]);
            using var launch = JsonDocument.Parse(host.Files[Project + @"\.vscode\launch.json"]);
            using var settings = JsonDocument.Parse(host.Files[Project + @"\.vscode\settings.json"]);
            var build = tasks.RootElement.GetProperty("tasks").EnumerateArray().Single(task => task.GetProperty("label").GetString() == "Build");
            var windows = launch.RootElement.GetProperty("configurations").EnumerateArray().Single(config => config.GetProperty("name").GetString() == "Run MyProject (Windows)");
            Assert.Multiple(() =>
            {
                Assert.That(build.GetProperty("preset").GetString(), Is.EqualTo("release"));
                Assert.That(windows.GetProperty("type").GetString(), Is.EqualTo("cppvsdbg"));
                Assert.That(windows.GetProperty("program").GetString(), Is.EqualTo("${command:cmake.launchTargetPath}"));
                Assert.That(settings.RootElement.GetProperty("cmake.buildBeforeRun").GetBoolean(), Is.True);
            });
        }
    }

    [TestCase("")]
    [TestCase(@"..\escape")]
    [TestCase("bad\nname")]
    [TestCase("nul")]
    public void UnsafeNameIsRejectedBeforeAnyWrites(string name)
    {
        Assert.That(() => service.Create(name, Parent), Throws.Exception);
        Assert.That(host.Mutations, Is.Empty);
    }

    [TestCase("D:relative")]
    [TestCase(@"E:\missing")]
    [TestCase(@"E:\bad;parent")]
    public void InvalidDestinationIsRejectedBeforeAnyWrites(string parent)
    {
        Assert.That(() => service.Create("MyProject", parent), Throws.Exception);
        Assert.That(host.Mutations, Is.Empty);
    }

    [TestCase(false)]
    [TestCase(true)]
    public void ExistingDestinationIsPreserved(bool file)
    {
        if (file) host.Files[Project] = "original";
        else { host.Directories.Add(Project); host.Files[Project + @"\keep.txt"] = "original"; }
        Assert.That(() => service.Create("MyProject", Parent), Throws.Exception);
        Assert.That(host.Mutations, Is.Empty);
        Assert.That(host.Files.Values, Does.Contain("original"));
    }

    [TestCase("main.cpp")]
    [TestCase("run-report.ps1")]
    [TestCase("CMakePresets.json")]
    [TestCase(@"cmake\shaders.cmake")]
    public void MissingRequiredTemplateFailsBeforeProjectWrites(string relative)
    {
        host.Files.Remove(Templates + "\\" + relative);
        Assert.That(() => service.Create("MyProject", Parent), Throws.Exception.Message.Contains(relative).And.Message.Contains("reinstall"));
        Assert.That(host.Mutations, Is.Empty);
    }

    [Test]
    public void DisabledEditorDoesNotRequireEditorTemplates()
    {
        foreach (var relative in new[] { "settings.json", "tasks.json", "launch.json" }) host.Files.Remove(Templates + @"\vscode\" + relative);
        Assert.That(() => service.Create("MyProject", Parent, withVscode: false), Throws.Nothing);
        Assert.That(messages.Any(message => message.Contains("WARNING")), Is.False);
    }

    [Test]
    public void MissingOptionalEditorFileWarnsAndStillCreatesUsableProject()
    {
        host.Files.Remove(Templates + @"\vscode\launch.json");
        Assert.That(() => service.Create("MyProject", Parent), Throws.Nothing);
        Assert.Multiple(() =>
        {
            Assert.That(host.Files.ContainsKey(Project + @"\CMakePresets.json"), Is.True);
            Assert.That(host.Files.ContainsKey(Project + @"\.vscode\tasks.json"), Is.True);
            Assert.That(messages, Has.Some.Contains("WARNING").And.Some.Contains("launch.json"));
        });
    }

    [TestCase("")]
    [TestCase("@PROJECT_NAME@")]
    public void UnusableRequiredTemplateIsRejected(string contents)
    {
        host.Files[Templates + @"\main.cpp"] = contents;
        Assert.That(() => service.Create("MyProject", Parent), Throws.Exception);
        Assert.That(host.Mutations, Is.Empty);
    }

    [Test]
    public void OptionalShaderCopyUsesCustomInstallationPreservesNestingAndExcludesSpirv()
    {
        var shaderRoot = Root + @"\share\MayaFlux\src\shaders";
        host.AddFile(shaderRoot + @"\nested\edit.comp", "editable GLSL");
        host.AddFile(shaderRoot + @"\include\common.glsl", "shared GLSL");
        host.AddFile(shaderRoot + @"\nested\edit.comp.spv", "binary");
        host.AddFile(shaderRoot + @"\attractor.frag", "library shader");
        service.Create("MyProject", Parent, copyShaderSources: true);
        Assert.Multiple(() =>
        {
            Assert.That(host.Files[Project + @"\data\shaders\nested\edit.comp"], Is.EqualTo("editable GLSL"));
            Assert.That(host.Files[Project + @"\data\shaders\include\common.glsl"], Is.EqualTo("shared GLSL"));
            Assert.That(host.Files[Project + @"\data\shaders\attractor.frag"], Is.EqualTo("library shader"));
            Assert.That(host.Files.Keys.Where(path => path.StartsWith(Project + "\\")), Has.None.EndsWith(".spv"));
        });
    }

    [Test]
    public void MissingInstalledShadersOnlyBlocksWhenRequested()
    {
        Assert.That(() => service.Create("MyProject", Parent, copyShaderSources: true), Throws.Exception.Message.Contains("shader sources"));
        Assert.That(host.Mutations, Is.Empty);
        Assert.That(() => service.Create("MyProject", Parent), Throws.Nothing);
    }

    [TestCase(false)]
    [TestCase(true)]
    public void EmptyOrLinkedShaderSourcesFailBeforeWrites(bool linked)
    {
        var shaderRoot = Root + @"\share\MayaFlux\src\shaders";
        host.Directories.Add(shaderRoot);
        if (linked)
        {
            host.AddFile(shaderRoot + @"\nested\edit.comp", "source");
            host.LinkedPaths.Add(shaderRoot + @"\nested");
        }
        Assert.That(() => service.Create("MyProject", Parent, copyShaderSources: true), Throws.Exception.Message.Contains("shader sources"));
        Assert.That(host.Mutations, Is.Empty);
    }

    [Test]
    public void DefaultInstallationRootIsUsedWhenNoEnvironmentRootIsSet()
    {
        host.Root = null;
        host.AddFile(@"C:\MayaFlux\share\MayaFlux\src\shaders\edit.comp", "default installation");
        service.Create("MyProject", Parent, copyShaderSources: true);
        Assert.That(host.Files[Project + @"\data\shaders\edit.comp"], Is.EqualTo("default installation"));
    }

    [TestCase(Parent)]
    [TestCase(Templates)]
    public void LinkedProjectOrTemplatePathIsRejectedBeforeWrites(string path)
    {
        host.LinkedPaths.Add(path);
        Assert.That(() => service.Create("MyProject", Parent), Throws.Exception);
        Assert.That(host.Mutations, Is.Empty);
    }

    [TestCase("write")]
    [TestCase("copy")]
    public void FailedCreationCleansOnlyItsNewProjectAndCanBeRetried(string operation)
    {
        host.AddFile(Parent + @"\Other\keep.txt", "existing work");
        host.Failure = operation;
        Assert.That(() => service.Create("MyProject", Parent), Throws.Exception);
        Assert.That(host.DirectoryExists(Project), Is.False);
        Assert.That(host.Files[Parent + @"\Other\keep.txt"], Is.EqualTo("existing work"));
        host.Failure = null;
        Assert.That(() => service.Create("MyProject", Parent), Throws.Nothing);
    }

    [Test]
    public void CleanupFailureReportsIncompleteLocationAndPreservesOtherProjects()
    {
        host.AddFile(Parent + @"\Other\keep.txt", "existing work");
        host.Failure = "copy";
        host.CleanupFailure = true;
        Assert.That(() => service.Create("MyProject", Parent), Throws.Exception.Message.Contains(Project).And.Message.Contains("Controlled copy failure").And.Message.Contains("Controlled cleanup failure"));
        Assert.That(host.Files[Parent + @"\Other\keep.txt"], Is.EqualTo("existing work"));
    }
}

internal sealed class FakeProjectCreationHost : ProjectCreationHost
{
    internal Dictionary<string, string> Files { get; } = new(StringComparer.OrdinalIgnoreCase);
    internal HashSet<string> Directories { get; } = new(StringComparer.OrdinalIgnoreCase);
    internal HashSet<string> LinkedPaths { get; } = new(StringComparer.OrdinalIgnoreCase);
    internal List<string> Mutations { get; } = [];
    internal string? Root { get; set; }
    internal string? Failure { get; set; }
    internal bool CleanupFailure { get; set; }
    internal override string? GetEnvironmentVariable(string name, EnvironmentVariableTarget target) => Root;
    internal override bool FileExists(string path) => Files.ContainsKey(path);
    internal override bool DirectoryExists(string path) => Directories.Contains(path);
    internal override FileAttributes GetAttributes(string path) => (DirectoryExists(path) ? FileAttributes.Directory : FileAttributes.Normal) | (LinkedPaths.Contains(path) ? FileAttributes.ReparsePoint : 0);
    internal override string ReadAllText(string path) => Files.TryGetValue(path, out var contents) ? contents : throw new FileNotFoundException(path);
    internal override string[] GetFiles(string path) => Files.Keys.Where(file => Path.GetDirectoryName(file)!.Equals(path, StringComparison.OrdinalIgnoreCase)).ToArray();
    internal override string[] GetFileSystemEntries(string path) => Files.Keys.Concat(Directories).Where(entry => Path.GetDirectoryName(entry)?.Equals(path, StringComparison.OrdinalIgnoreCase) == true).ToArray();
    internal override void CreateDirectory(string path) { Mutations.Add("mkdir " + path); AddParents(path); }
    internal override void WriteAllText(string path, string text)
    {
        Mutations.Add("write " + path);
        if (Failure == "write") throw new IOException("Controlled write failure");
        Files[path] = text;
    }
    internal override void CopyFile(string source, string destination)
    {
        Mutations.Add("copy " + destination);
        if (Failure == "copy") throw new IOException("Controlled copy failure");
        Files.Add(destination, ReadAllText(source));
    }
    internal override void DeleteDirectory(string path)
    {
        Mutations.Add("delete " + path);
        if (CleanupFailure) throw new IOException("Controlled cleanup failure");
        foreach (var file in Files.Keys.Where(file => file.StartsWith(path + "\\", StringComparison.OrdinalIgnoreCase)).ToArray()) Files.Remove(file);
        Directories.RemoveWhere(directory => directory.Equals(path, StringComparison.OrdinalIgnoreCase) || directory.StartsWith(path + "\\", StringComparison.OrdinalIgnoreCase));
    }
    internal void AddFile(string path, string contents) { Files[path] = contents; AddParents(Path.GetDirectoryName(path)!); }
    private void AddParents(string path) { for (string? current = path; current != null; current = Path.GetDirectoryName(current)) Directories.Add(current); }
}
