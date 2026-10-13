using NUnit.Framework;
using Weave.Shared.Models;
using Weave.UI.Pages;
using Weave.Utils;

namespace Weave.Windows.UnitTests;

[TestFixture]
public class TemplateInstallStepTests
{
    private const string Root = @"D:\Maya Flux";
    private const string Templates = @"D:\Maya Flux\share\weave\templates";
    private static readonly string[] Required = ["CMakeLists.txt", "CMakePresets.json", "main.cpp", "user_project.hpp", ".gitignore", "cmake\\mayaflux.cmake", "cmake\\shaders.cmake", "cmake\\build_community.cmake", "community\\module.cmake", "community\\community.json", "community\\CMakePresets.json", "community\\test\\CMakeLists.txt", "vscode\\settings.json", "vscode\\tasks.json", "vscode\\launch.json"];
    private static string Resource(string relative) => "Weave.Resources.templates." + relative.Replace('\\', '.');

    [Test]
    public void NativeExtractionCopiesEmbeddedBytesAndPreservesCustomFiles()
    {
        using var directory = new TestDirectory();
        ResourceExtractor.ExtractAllResources(directory.PathName);
        var templates = Path.Combine(directory.PathName, "share", "weave", "templates");
        var custom = Path.Combine(templates, "custom.txt");
        File.WriteAllText(custom, "User content");
        ResourceExtractor.ExtractTemplates(directory.PathName);
        Assert.That(File.ReadAllText(custom), Is.EqualTo("User content"));
        foreach (var relative in Required)
        {
            using var resource = new ResourceExtractionHost().OpenResource(Resource(relative));
            using var bytes = new MemoryStream();
            resource!.CopyTo(bytes);
            Assert.That(File.ReadAllBytes(Path.Combine(templates, relative)), Is.EqualTo(bytes.ToArray()), relative);
        }
        Assert.That(Directory.GetFiles(directory.PathName, "*.tmp", SearchOption.AllDirectories), Is.Empty);
    }

    [Test]
    public void DuplicateWindowsResourceDestinationsAreRejectedBeforeMutation()
    {
        var host = new FakeResourceExtractionHost();
        host.Resources["Weave.Resources.templates.MAIN.cpp"] = [1];
        Assert.That(() => ResourceExtractor.ExtractAllResources(Root, host), Throws.Exception);
        Assert.That(host.Mutations, Is.Empty);
    }

    [Test]
    public void NestedScriptResourceGetsItsParentDirectory()
    {
        var host = new FakeResourceExtractionHost();
        host.Resources["Weave.Resources.scripts.tools.setup.ps1"] = [1, 2, 3];
        ResourceExtractor.ExtractScripts(Root, host);
        Assert.That(host.Directories, Does.Contain(@"D:\Maya Flux\share\weave\scripts\tools"));
        Assert.That(host.Files[@"D:\Maya Flux\share\weave\scripts\tools\setup.ps1"], Is.EqualTo(new byte[] { 1, 2, 3 }));
        Assert.That(host.Files.Keys, Has.None.Contains("templates"));
    }

    [Test]
    public async Task CleanupFailureReportsTemporaryPathAndOriginalFailure()
    {
        var host = new FakeResourceExtractionHost { FailedWrite = "main.cpp", FailDelete = true };
        var messages = new List<string>();
        var step = new TemplatesInstallStep(host);
        step.MessageLogged += messages.Add;
        await step.ExtractResourcesAsync(new InstallationConfig { MayaFluxRoot = Root });
        Assert.That(messages.Any(message => message.Contains(".tmp") && message.Contains("Resource write failed")), Is.True);
    }

    [Test]
    public async Task PackageWithoutScriptsDoesNotReportAnInstallationWarning()
    {
        var host = new FakeResourceExtractionHost();
        var messages = new List<string>();
        var step = new TemplatesInstallStep(host);
        step.MessageLogged += messages.Add;
        await step.ExtractResourcesAsync(new InstallationConfig { MayaFluxRoot = Root });
        Assert.That(messages.Where(m => m.Contains("[WARN]") || m.Contains("0 PowerShell scripts")), Is.Empty);
        Assert.That(host.Directories, Does.Not.Contain(@"D:\Maya Flux\share\weave\scripts"));
    }

    [TestCase("missing")]
    [TestCase("null")]
    [TestCase("empty")]
    [TestCase("read")]
    public async Task InvalidPackagedResourceCannotBeMaskedByAnExistingInstallation(string failure)
    {
        var host = new FakeResourceExtractionHost();
        ResourceExtractor.ExtractAllResources(Root, host);
        host.Mutations.Clear();
        var previous = host.Files[Templates + @"\main.cpp"].ToArray();
        var name = Resource("main.cpp");
        switch (failure)
        {
            case "missing": host.Resources.Remove(name); break;
            case "null": host.Resources[name] = null; break;
            case "empty": host.Resources[name] = []; break;
            case "read": host.FailedResourceRead = name; break;
        }
        var result = await RunAsync(new TemplatesInstallStep(host));
        result.AssertBlocked();
        Assert.That(host.Mutations, Is.Empty);
        Assert.That(host.Files[Templates + @"\main.cpp"], Is.EqualTo(previous));
    }

    [Test]
    public async Task RetryTransitionsFromFailureToSuccessAndBackWithoutStaleReadiness()
    {
        var host = new FakeResourceExtractionHost { FailedWrite = "main.cpp" };
        var step = new TemplatesInstallStep(host);
        (await RunAsync(step)).AssertBlocked();
        host.FailedWrite = null;
        Assert.That((await RunAsync(step)).Passed, Is.True);
        Assert.That(host.Files[Templates + @"\main.cpp"], Is.Not.Empty);
        host.FailedWrite = "main.cpp";
        (await RunAsync(step)).AssertBlocked();
    }

    private static Task<StepObservation> RunAsync(TemplatesInstallStep step) =>
        StepObservation.CaptureReadinessAsync(async (log, ready) =>
        {
            step.MessageLogged += log;
            step.ProceedChanged += ready;
            try { await step.ExtractResourcesAsync(new InstallationConfig { MayaFluxRoot = Root }); }
            finally
            {
                step.MessageLogged -= log;
                step.ProceedChanged -= ready;
            }
        });
    [Test]
    public async Task RealEmbeddedResourcesProduceUsableTemplateLayout()
    {
        var host = new FakeResourceExtractionHost();
        var step = new TemplatesInstallStep(host);
        bool? ready = null;
        step.ProceedChanged += value => ready = value;
        await step.ExtractResourcesAsync(new InstallationConfig { MayaFluxRoot = Root });
        Assert.That(ready, Is.True);
        foreach (var relative in Required) Assert.That(host.Files[Templates + "\\" + relative], Is.Not.Empty, relative);
        Assert.That(host.Files[Templates + @"\shaders\attractor.frag"], Is.EqualTo(host.Resources["Weave.Resources.templates.shaders.attractor.frag"]));
    }

    [TestCase("")]
    [TestCase("relative")]
    [TestCase("D:relative")]
    [TestCase("D:\\")]
    [TestCase("D:\\Bad;Root")]
    [TestCase("D:\\Bad\nRoot")]
    public void UnsafeDestinationIsRejectedBeforeMutation(string root)
    {
        var host = new FakeResourceExtractionHost();
        Assert.That(() => ResourceExtractor.ExtractAllResources(root, host), Throws.Exception);
        Assert.That(host.Mutations, Is.Empty);
    }

    [TestCase(@"D:\Maya Flux")]
    [TestCase(@"D:\Maya Flux\share")]
    [TestCase(@"D:\Maya Flux\share\weave\templates\cmake")]
    public void ReparseAncestorIsRejectedBeforeMutation(string path)
    {
        var host = new FakeResourceExtractionHost();
        host.Directories.Add(path);
        host.ReparsePaths.Add(path);
        Assert.That(() => ResourceExtractor.ExtractAllResources(Root, host), Throws.Exception);
        Assert.That(host.Mutations, Is.Empty);
    }

    [TestCase("create")]
    [TestCase("write")]
    [TestCase("replace")]
    [TestCase("enumerate")]
    public async Task HostFailureBlocksNextPreservesExistingFilesAndCleansTemporaryWrites(string operation)
    {
        var host = new FakeResourceExtractionHost();
        host.Files[Templates + @"\main.cpp"] = [1, 2, 3];
        if (operation == "create") host.FailedCreate = "templates";
        if (operation == "write") host.FailedWrite = "main.cpp";
        if (operation == "replace") host.FailedMove = "main.cpp";
        if (operation == "enumerate") host.FailEnumeration = true;
        var result = await RunAsync(new TemplatesInstallStep(host));
        result.AssertBlocked();
        Assert.That(result.Messages, Has.None.Contains("Installation Complete"));
        Assert.That(host.Files.Keys.Any(path => path.EndsWith(".tmp")), Is.False);
        if (operation != "enumerate")
            Assert.That(host.Files[Templates + @"\main.cpp"], Is.EqualTo(new byte[] { 1, 2, 3 }));
    }

    [Test]
    public void LookalikeResourcePrefixIsIgnored()
    {
        var host = new FakeResourceExtractionHost();
        host.Resources["Weave.Resources.templatesExtra.evil.txt"] = [1];
        ResourceExtractor.ExtractAllResources(Root, host);
        Assert.That(host.Files.Keys.Any(p => p.Contains("evil")), Is.False);
    }

    [TestCase("..escape.txt")]
    [TestCase("nested/../../escape.txt")]
    [TestCase("D:\\escape.txt")]
    [TestCase("file:stream.txt")]
    [TestCase("CON.txt")]
    [TestCase("bad?.txt")]
    public void UnsafeResourceNameIsRejectedBeforeMutation(string suffix)
    {
        var host = new FakeResourceExtractionHost();
        host.Resources["Weave.Resources.templates." + suffix] = [1];
        Assert.That(() => ResourceExtractor.ExtractAllResources(Root, host), Throws.Exception);
        Assert.That(host.Mutations, Is.Empty);
    }
}
