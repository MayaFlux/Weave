using System.Diagnostics;
using System.ComponentModel;
using System.Text;
using System.Text.Json;
using System.Windows.Forms;
using NUnit.Framework;
using Weave.UI.Project;
using Weave.UI.Pages;
using Weave.Shared;
using Weave.Utils;

namespace Weave.Windows.UnitTests;

[TestFixture]
public class CommunityCreationTests
{
    private const string Root = @"D:\Maya Flux";
    private const string Templates = Root + @"\share\weave\templates";
    private const string Parent = @"E:\Community Work";
    private const string Module = Parent + @"\terrain_field";
    private FakeProjectCreationHost host = null!;
    private GitHost git = null!;
    private List<string> messages = null!;
    private CommunityCreationService service = null!;

    [SetUp]
    public void Setup()
    {
        host = new FakeProjectCreationHost();
        var resources = new FakeResourceExtractionHost();
        ResourceExtractor.ExtractTemplates(Root, resources);
        foreach (var file in resources.Files) host.AddFile(file.Key, Encoding.UTF8.GetString(file.Value));
        host.Directories.Add(Parent);
        git = new GitHost();
        messages = [];
        service = new CommunityCreationService(Templates, host, git, messages.Add);
    }

    [TestCase(false)]
    [TestCase(true)]
    public async Task CreatesUsableModuleWithConsistentManifestCMakeAndPresetEntryPoint(bool lila)
    {
        Assert.That(await service.CreateAsync("terrain_field", Parent, "Quotes \" and \\ paths", "0.8.1", lila), Is.EqualTo(Module));
        var manifest = CommunityManifest.Parse(host.Files[Module + @"\community.json"], "terrain_field");
        Assert.Multiple(() =>
        {
            Assert.That(manifest.GetProperty("description").GetString(), Is.EqualTo("Quotes \" and \\ paths"));
            Assert.That(manifest.GetProperty("min_version").GetString(), Is.EqualTo("0.8.1"));
            Assert.That(manifest.GetProperty("needs_lila").GetBoolean(), Is.EqualTo(lila));
            Assert.That(host.Files[Module + @"\terrain_field.cmake"], Does.Contain("set(MF_MIN_VERSION \"0.8.1\")").And.Contains(lila ? "set(MF_NEEDS_LILA ON)" : "set(MF_NEEDS_LILA OFF)"));
            Assert.That(host.Files.ContainsKey(Module + @"\CMakeLists.txt"), Is.True);
            Assert.That(host.Files[Module + @"\src\terrain_field.cpp"], Is.Not.Empty);
            Assert.That(host.Files[Module + @"\test\test_terrain_field.cpp"], Does.Contain("main("));
            Assert.That(host.Files[Module + @"\README.md"], Does.Contain("cmake --preset release").And.Contains("cmake --build --preset release"));
            Assert.That(host.Files.Where(file => file.Key.StartsWith(Module + "\\")).Any(file => file.Value.Contains("@MODULE_NAME@")), Is.False);
            Assert.That(git.Requests.Select(request => request.ArgumentList.ToArray()), Is.EqualTo(new[] { new[] { "init", "-b", "main" }, new[] { "add", "-A" } }));
            Assert.That(git.Requests.All(request => request.WorkingDirectory == Module && !request.UseShellExecute && request.RedirectStandardOutput && request.RedirectStandardError), Is.True);
        });
    }

    [TestCase("../bad")]
    [TestCase("con")]
    [TestCase("bad\n")]
    public void UnsafeModuleNamesCannotChangeFiles(string name)
    {
        Assert.CatchAsync(async () => await service.CreateAsync(name, Parent));
        Assert.That(host.Mutations, Is.Empty);
    }

    [TestCase("0.5")]
    [TestCase("01.5.0")]
    public void InvalidManifestInputFailsBeforeWrites(string minimum)
    {
        Assert.CatchAsync(async () => await service.CreateAsync("terrain_field", Parent, minimum: minimum));
        Assert.That(host.Mutations, Is.Empty);
    }

    [TestCase("relative")]
    [TestCase("E:relative")]
    [TestCase("E:\\Bad;Path")]
    public void UnsafeDestinationsFailBeforeWrites(string destination)
    {
        host.Directories.Add(destination);
        Assert.CatchAsync(async () => await service.CreateAsync("terrain_field", destination));
        Assert.That(host.Mutations, Is.Empty);
    }

    [TestCase(false)]
    [TestCase(true)]
    public void ExistingDirectoryOrFileIsPreserved(bool file)
    {
        if (file) host.AddFile(Module, "user work");
        else host.AddFile(Module + @"\keep.txt", "user work");
        Assert.CatchAsync(async () => await service.CreateAsync("terrain_field", Parent));
        Assert.That(host.Mutations, Is.Empty);
        Assert.That(host.Files[file ? Module : Module + @"\keep.txt"], Is.EqualTo("user work"));
    }

    [TestCase("missing")]
    [TestCase("empty")]
    [TestCase("unfinished")]
    [TestCase("bad-json")]
    public void InvalidRequiredTemplateFailsBeforeWritesWithReinstallGuidance(string kind)
    {
        var path = Templates + (kind == "bad-json" ? @"\community\CMakePresets.json" : @"\community\test\CMakeLists.txt");
        if (kind == "missing") host.Files.Remove(path);
        else host.Files[path] = kind == "empty" ? " " : kind == "bad-json" ? "{bad" : "@UNKNOWN_VALUE@";
        var error = Assert.CatchAsync(async () => await service.CreateAsync("terrain_field", Parent));
        Assert.That(host.Mutations, Is.Empty);
        Assert.That(error!.Message, Does.Contain("reinstall"));
    }

    [TestCase(Parent)]
    [TestCase(Templates + @"\community\community.json")]
    public void LinkedDestinationOrTemplateCannotRedirectWrites(string path)
    {
        host.LinkedPaths.Add(path);
        Assert.CatchAsync(async () => await service.CreateAsync("terrain_field", Parent));
        Assert.That(host.Mutations, Is.Empty);
    }

    [Test]
    public async Task FailedFileGenerationCleansOnlyItsModuleAndSupportsRetry()
    {
        host.AddFile(Parent + @"\Other\keep.txt", "user work");
        host.Failure = "write";
        Assert.CatchAsync(async () => await service.CreateAsync("terrain_field", Parent));
        Assert.That(host.DirectoryExists(Module), Is.False);
        Assert.That(host.Files[Parent + @"\Other\keep.txt"], Is.EqualTo("user work"));
        Assert.That(git.Requests, Is.Empty);
        host.Failure = null;
        await service.CreateAsync("terrain_field", Parent);
        Assert.That(host.Files.ContainsKey(Module + @"\community.json"), Is.True);
    }

    [Test]
    public void FailedCleanupReportsTheIncompletePathAndOriginalFailure()
    {
        host.Failure = "write";
        host.CleanupFailure = true;
        var error = Assert.CatchAsync(async () => await service.CreateAsync("terrain_field", Parent));
        Assert.That(error!.Message, Does.Contain(Module).And.Contains("Controlled write failure").And.Contains("Controlled cleanup failure"));
    }

    [TestCase("missing")]
    [TestCase("init-failed")]
    [TestCase("add-failed")]
    [TestCase("timeout")]
    public async Task GitProblemsPreserveAUsableModuleAndExplainManualRecovery(string outcome)
    {
        git.Outcome = outcome;
        Assert.That(await service.CreateAsync("terrain_field", Parent), Is.EqualTo(Module));
        Assert.Multiple(() =>
        {
            Assert.That(host.Files.ContainsKey(Module + @"\community.json"), Is.True);
            Assert.That(messages, Has.Some.Contains("WARNING").And.Some.Contains("git"));
            Assert.That(messages, Has.None.Contains("Initialized git repository"));
            Assert.That(git.Requests.Count, Is.EqualTo(outcome == "add-failed" ? 2 : 1));
            if (outcome == "timeout") Assert.That(git.Processes.Single().Terminated, Is.True);
            if (outcome == "missing") Assert.That(messages, Has.Some.Contains("Install Git"));
            if (outcome is "init-failed" or "add-failed") Assert.That(messages, Has.Some.Contains("Git diagnostic"));
        });
    }
    [Test]
    [Apartment(ApartmentState.STA)]
    public void MissingCriticalTemplateKeepsTheFormAndDoesNotLeaveAnUnusableModule()
    {
        using var directory = new TestDirectory();
        ResourceExtractor.ExtractTemplates(directory.PathName);
        var templates = Path.Combine(directory.PathName, "share", "weave", "templates");
        File.Delete(Path.Combine(templates, ".gitignore"));
        var notifications = new List<string>();
        using var form = new Form();
        var nativeGit = new GitHost { Outcome = "timeout" };
        using var view = new CommunityCreatorView(templates, new ProjectCreationHost(), nativeGit, (message, _, _) => notifications.Add(message));
        form.Controls.Add(view);
        form.Show();
        var inputs = Children(view).OfType<TextBox>().ToArray();
        inputs[0].Text = "terrain_field";
        inputs[3].Text = directory.PathName;
        var create = Children(view).OfType<Button>().Single(button => button.Text == "Create Module");
        create.PerformClick();
        Assert.Multiple(() =>
        {
            Assert.That(Directory.Exists(Path.Combine(directory.PathName, "terrain_field")), Is.False);
            Assert.That(create.Enabled && inputs.All(input => input.Enabled), Is.True);
            Assert.That(form.IsDisposed, Is.False);
            Assert.That(notifications.Single(), Does.Contain("reinstall"));
        });
        ResourceExtractor.ExtractTemplates(directory.PathName);
        create.PerformClick();
        Assert.That(create.Enabled || inputs.Any(input => input.Enabled), Is.False);
        PumpUntil(() => notifications.Count == 2);
        Assert.That(notifications.Last(), Does.Contain(Path.Combine(directory.PathName, "terrain_field")).And.Contains("WARNING").And.Contains("git init"));
        Assert.That(File.Exists(Path.Combine(directory.PathName, "terrain_field", "CMakeLists.txt")), Is.True);
    }

    [Test]
    public async Task NativeModuleFromPackagedTemplatesHasPresetsAcceptedByCMake()
    {
        using var directory = new TestDirectory();
        ResourceExtractor.ExtractTemplates(directory.PathName);
        var nativeService = new CommunityCreationService(Path.Combine(directory.PathName, "share", "weave", "templates"), new ProjectCreationHost(), new GitHost(), _ => { });
        var module = await nativeService.CreateAsync("terrain_field", directory.PathName);
        Assert.That(File.Exists(Path.Combine(module, "CMakeLists.txt")), Is.True);
        var request = new ProcessStartInfo("cmake") { WorkingDirectory = module, UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true, CreateNoWindow = true };
        request.ArgumentList.Add("--list-presets=all");
        using var process = Process.Start(request)!;
        var output = process.StandardOutput.ReadToEndAsync();
        var error = process.StandardError.ReadToEndAsync();
        try { await process.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(10)); }
        catch { if (!process.HasExited) { process.Kill(true); process.WaitForExit(2000); } throw; }
        Assert.That(process.ExitCode, Is.Zero, await error);
        Assert.That(await output, Does.Contain("release").And.Contains("debug"));
        Assert.That(Directory.Exists(Path.Combine(module, "test", "build")), Is.False);
    }

    private static IEnumerable<Control> Children(Control control) => control.Controls.Cast<Control>()
        .SelectMany(child => new[] { child }.Concat(Children(child)));

    private static void PumpUntil(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow.AddSeconds(5);
        while (!condition() && DateTime.UtcNow < deadline) { Application.DoEvents(); Thread.Sleep(5); }
        Assert.That(condition(), Is.True, "The community creator did not reach the expected state");
    }

    private sealed class GitHost : DependenciesHost
    {
        internal string Outcome { get; set; } = "success";
        internal List<ProcessStartInfo> Requests { get; } = [];
        internal List<GitProcess> Processes { get; } = [];
        internal override TimeSpan ProcessTimeout => TimeSpan.FromMilliseconds(50);
        internal override IDependencyProcess? StartProcess(ProcessStartInfo request)
        {
            Requests.Add(request);
            if (Outcome == "missing") throw new Win32Exception(2, "Git not found");
            var failure = Outcome == "init-failed" || Outcome == "add-failed" && Requests.Count == 2;
            var process = new GitProcess(Outcome == "timeout", failure ? 2 : 0);
            Processes.Add(process);
            return process;
        }
    }

    private sealed class GitProcess(bool stalled, int code) : IDependencyProcess
    {
        internal bool Terminated { get; private set; }
        public int ExitCode => code;
        public bool HasExited => !stalled || Terminated;
        public void Terminate() => Terminated = true;
        public void WaitForExit() { if (stalled) throw new TimeoutException("Unbounded Git wait"); }
        public Task WaitForExitAsync(CancellationToken cancellationToken = default) => stalled && !Terminated
            ? Task.Delay(Timeout.Infinite, cancellationToken) : Task.CompletedTask;
        public string ReadStandardOutput() => "Git output";
        public Task<string> ReadStandardOutputAsync() => Task.FromResult("Git output");
        public Task<string> ReadStandardErrorAsync() => Task.FromResult("Git diagnostic");
        public void Dispose() { }
    }
}
