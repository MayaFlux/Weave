using System.ComponentModel;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.Net;
using System.Text;
using System.Text.Json;
using System.Windows.Forms;
using NUnit.Framework;
using Weave.Shared;
using Weave.UI.Project;

namespace Weave.Windows.UnitTests;

[TestFixture]
public class ProjectUpdateTests
{
    [Test]
    public void NamesPreserveOrderAndRemoveDuplicates()
    {
        Assert.That(CommunityUpdate.ParseNames([" terrain_field, noise ", "noise,sky"]), Is.EqualTo(new[] { "terrain_field", "noise", "sky" }));
        Assert.Throws<InvalidOperationException>(() => CommunityUpdate.ParseNames([]));
    }

    [TestCase("")]
    [TestCase("terrain_field,")]
    [TestCase("../terrain")]
    [TestCase("con")]
    public void InvalidOrReservedNamesCannotBecomeWindowsModulePaths(string value)
    {
        Assert.Throws<InvalidOperationException>(() => CommunityUpdate.ParseNames([value]));
    }

    [Test]
    public async Task ExistingModulesAndRegistrationArePreservedAcrossRepeatedUpdates()
    {
        using var fixture = new UpdateFixture();
        fixture.Module("terrain_field");
        fixture.Module("noise", requirements: true);
        var original = "# User-owned entries\r\nterrain_field";
        File.WriteAllText(fixture.Registration, original);
        Assert.That(await fixture.Run("terrain_field,noise,noise"), Is.EqualTo(new[] { "terrain_field", "noise" }));
        var expected = original + "\nnoise\n";
        Assert.That(File.ReadAllText(fixture.Registration), Is.EqualTo(expected));
        await fixture.Run("noise");
        Assert.Multiple(() =>
        {
            Assert.That(File.ReadAllText(fixture.Registration), Is.EqualTo(expected));
            Assert.That(File.ReadAllText(Path.Combine(fixture.Project, "src", "user.cpp")), Is.EqualTo("user content"));
            Assert.That(fixture.Messages, Has.Some.Contains("declares requirements").And.Some.Contains("already registered"));
        });
    }

    [TestCase("not-json")]
    [TestCase("{}")]
    [TestCase("duplicate")]
    [TestCase("unsafe-url")]
    [TestCase("bad-version")]
    public void InvalidRegistryCannotChangeProjectFiles(string kind)
    {
        using var fixture = new UpdateFixture();
        fixture.Module("terrain_field");
        fixture.Registry = kind switch
        {
            "duplicate" => UpdateFixture.RegistryFor("terrain_field", "terrain_field"),
            "unsafe-url" => "[{\"name\":\"terrain_field\",\"repo\":\"http://example.invalid/module.git\",\"min_version\":\"0.5.0\"}]",
            "bad-version" => "[{\"name\":\"terrain_field\",\"repo\":\"https://example.invalid/module.git\",\"min_version\":\"0.5\"}]",
            _ => kind
        };
        Assert.CatchAsync(async () => await fixture.Run("terrain_field"));
        Assert.That(File.Exists(fixture.Registration), Is.False);
    }

    [Test]
    public async Task NetworkFailurePreservesRegistrationAndCanBeRetried()
    {
        using var fixture = new UpdateFixture();
        fixture.Module("terrain_field");
        File.WriteAllText(fixture.Registration, "existing\r\n");
        fixture.Status = HttpStatusCode.ServiceUnavailable;
        Assert.CatchAsync(async () => await fixture.Run("terrain_field"));
        Assert.That(File.ReadAllText(fixture.Registration), Is.EqualTo("existing\r\n"));
        fixture.Status = HttpStatusCode.OK;
        await fixture.Run("terrain_field");
        Assert.That(File.ReadAllText(fixture.Registration), Is.EqualTo("existing\r\nterrain_field\n"));
    }

    [TestCase("missing-cmake", "missing terrain_field.cmake")]
    [TestCase("missing-src", "missing src/")]
    [TestCase("bad-manifest", "Invalid JSON")]
    [TestCase("wrong-name", "does not match")]
    [TestCase("newer-version", "requires MayaFlux")]
    public void InvalidModuleLeavesExistingRegistrationAndUserContentUntouched(string kind, string message)
    {
        using var fixture = new UpdateFixture();
        var module = fixture.Module("terrain_field");
        fixture.Module("noise");
        switch (kind)
        {
            case "missing-cmake": File.Delete(Path.Combine(module, "terrain_field.cmake")); break;
            case "missing-src": Directory.Delete(Path.Combine(module, "src")); break;
            case "bad-manifest": File.WriteAllText(Path.Combine(module, "community.json"), "{bad"); break;
            case "wrong-name": File.WriteAllText(Path.Combine(module, "community.json"), "{\"name\":\"other\",\"min_version\":\"0.5.0\"}"); break;
            case "newer-version": File.WriteAllText(Path.Combine(module, "community.json"), "{\"name\":\"terrain_field\",\"min_version\":\"2.0.0\"}"); break;
        }
        File.WriteAllText(fixture.Registration, "user_entry");
        var error = Assert.ThrowsAsync<InvalidOperationException>(async () => await fixture.Run("noise,terrain_field"));
        Assert.Multiple(() =>
        {
            Assert.That(error!.Message, Does.Contain(message));
            Assert.That(File.ReadAllText(fixture.Registration), Is.EqualTo("user_entry"));
            Assert.That(File.ReadAllText(Path.Combine(module, "notes.txt")), Is.EqualTo("keep this"));
        });
    }

    [TestCase("invalid-project")]
    [TestCase("relative")]
    [TestCase("drive-relative")]
    public void InvalidProjectIsRejectedBeforeRegistryAccess(string kind)
    {
        using var fixture = new UpdateFixture();
        fixture.Module("terrain_field");
        var project = kind switch { "relative" => Path.GetRelativePath(Environment.CurrentDirectory, fixture.Project), "drive-relative" => Path.GetPathRoot(fixture.Project)![..2] + Path.GetRelativePath(Path.GetPathRoot(fixture.Project)!, fixture.Project), _ => fixture.Project };
        if (kind == "invalid-project") File.Delete(Path.Combine(project, "CMakeLists.txt"));
        Assert.CatchAsync(async () => await fixture.Update.RunAsync(project, ["terrain_field"]));
        Assert.That(fixture.Requests, Is.Zero);
    }

    [Test]
    public void UnknownModuleDoesNotRegisterAnyRequestedModules()
    {
        using var fixture = new UpdateFixture();
        fixture.Module("terrain_field");
        Assert.ThrowsAsync<InvalidOperationException>(async () => await fixture.Run("terrain_field,unknown"));
        Assert.That(File.Exists(fixture.Registration), Is.False);
    }

    [Test]
    public void InvalidInstalledVersionGivesRepairGuidanceBeforeModuleAcquisition()
    {
        using var fixture = new UpdateFixture();
        File.WriteAllText(fixture.VersionFile, "set(PACKAGE_VERSION \"0.5\")");
        var error = Assert.ThrowsAsync<InvalidOperationException>(async () => await fixture.Run("terrain_field"));
        Assert.That(error!.Message, Does.Contain("MayaFlux").And.Contains("reinstall"));
        Assert.That(Directory.Exists(Path.Combine(fixture.Project, "community")), Is.False);
    }

    [Test]
    public void RegistryCompatibilityIsCheckedBeforeAnyClone()
    {
        var host = new UpdateHost();
        using var fixture = new UpdateFixture(host);
        host.Populate = () => fixture.Module("terrain_field");
        fixture.Registry = UpdateFixture.RegistryFor("terrain_field").Replace("0.5.0", "0.10.0");
        var error = Assert.ThrowsAsync<InvalidOperationException>(async () => await fixture.Run("terrain_field"));
        Assert.Multiple(() =>
        {
            Assert.That(error!.Message, Does.Contain("0.10.0").And.Contains("0.8.0"));
            Assert.That(host.Request, Is.Null);
            Assert.That(Directory.Exists(Path.Combine(fixture.Project, "community")), Is.False);
        });
        host.Dispose();
    }

    [Test]
    [Apartment(ApartmentState.STA)]
    public void GuiReportsNetworkFailureKeepsInputsAndAllowsRetry()
    {
        using var fixture = new UpdateFixture();
        fixture.Module("terrain_field");
        fixture.Status = HttpStatusCode.ServiceUnavailable;
        fixture.ResponseGate = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var notifications = new List<string>();
        var attempts = 0;
        using var form = new Form();
        using var view = new AddCommunityView(log => { attempts++; return fixture.CreateUpdate(log); }, (message, _, _) => notifications.Add(message));
        form.Controls.Add(view);
        form.Show();
        var inputs = Children(view).OfType<TextBox>().ToArray();
        inputs[0].Text = fixture.Project;
        var add = Children(view).OfType<Button>().Single(button => button.Text == "Add Module");
        inputs[1].Text = "con";
        add.PerformClick();
        Assert.That(attempts, Is.Zero);
        Assert.That(notifications.Single(), Does.Contain("Invalid module name"));
        notifications.Clear();
        inputs[1].Text = "terrain_field";
        add.PerformClick();
        Assert.That(add.Enabled || inputs.Any(input => input.Enabled), Is.False);
        fixture.ResponseGate.SetResult();
        PumpUntil(() => add.Enabled);
        Assert.Multiple(() =>
        {
            Assert.That(Children(view).OfType<RichTextBox>().Single().Text, Does.Contain("[ERROR]").And.Contains("retry").IgnoreCase);
            Assert.That(inputs[0].Text, Is.EqualTo(fixture.Project));
            Assert.That(inputs[1].Text, Is.EqualTo("terrain_field"));
            Assert.That(File.Exists(fixture.Registration), Is.False);
            Assert.That(notifications, Is.Empty);
        });
        fixture.Status = HttpStatusCode.OK;
        add.PerformClick();
        PumpUntil(() => notifications.Count == 1);
        Assert.That(attempts, Is.EqualTo(2));
        Assert.That(File.ReadAllText(fixture.Registration), Is.EqualTo("terrain_field\n"));
    }

    private static IEnumerable<Control> Children(Control control) => control.Controls.Cast<Control>()
        .SelectMany(child => new[] { child }.Concat(Children(child)));

    private static void PumpUntil(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow.AddSeconds(5);
        while (!condition() && DateTime.UtcNow < deadline) { Application.DoEvents(); Thread.Sleep(5); }
        Assert.That(condition(), Is.True, "The update screen did not reach the expected state");
    }

    [Test]
    public void CleanupFailureReportsOwnedPartialCloneAndPreservesExistingModules()
    {
        var host = new UpdateHost { Outcome = "failed", FailCleanup = true };
        using var fixture = new UpdateFixture(host);
        var existing = fixture.Module("noise");
        File.WriteAllText(fixture.Registration, "noise\r\n");
        var partial = Path.Combine(fixture.Project, "community", "terrain_field");
        host.Populate = () => { Directory.CreateDirectory(partial); File.WriteAllText(Path.Combine(partial, "partial.txt"), "incomplete"); };
        try
        {
            var error = Assert.ThrowsAsync<InvalidOperationException>(async () => await fixture.Run("terrain_field"));
            Assert.Multiple(() =>
            {
                Assert.That(error!.Message, Does.Contain("exit 2").And.Contains(partial).And.Contains("remove").And.Contains("retry"));
                Assert.That(File.ReadAllText(fixture.Registration), Is.EqualTo("noise\r\n"));
                Assert.That(File.ReadAllText(Path.Combine(existing, "notes.txt")), Is.EqualTo("keep this"));
                Assert.That(File.Exists(Path.Combine(partial, "partial.txt")), Is.True);
                Assert.That(host.Child!.HasExited, Is.True);
            });
        }
        finally { host.Dispose(); }
    }

    [TestCase("community")]
    [TestCase("community.cmake")]
    [TestCase("community/terrain_field/community.json")]
    public void LinkedProjectPathsAreRejectedWithoutWritingThroughThem(string relative)
    {
        var host = new UpdateHost { LinkedSuffix = relative.Replace('/', Path.DirectorySeparatorChar) };
        using var fixture = new UpdateFixture(host);
        fixture.Module("terrain_field");
        File.WriteAllText(fixture.Registration, "keep_entry");
        var error = Assert.ThrowsAsync<InvalidOperationException>(async () => await fixture.Run("terrain_field"));
        Assert.That(error!.Message, Does.Contain("linked"));
        Assert.That(File.ReadAllText(fixture.Registration), Is.EqualTo("keep_entry"));
    }

    [Test]
    public async Task FailedRegistrationPreservesOriginalAndRetryRemovesTemporaryFiles()
    {
        var host = new UpdateHost { FailRegistration = true };
        using var fixture = new UpdateFixture(host);
        fixture.Module("terrain_field");
        File.WriteAllText(fixture.Registration, "keep_entry");
        Assert.ThrowsAsync<InvalidOperationException>(async () => await fixture.Run("terrain_field"));
        Assert.That(File.ReadAllText(fixture.Registration), Is.EqualTo("keep_entry"));
        Assert.That(Directory.GetFiles(fixture.Project, "*.weave-*"), Is.Empty);
        host.FailRegistration = false;
        await fixture.Run("terrain_field");
        Assert.That(File.ReadAllText(fixture.Registration), Is.EqualTo("keep_entry\nterrain_field\n"));
    }

    [TestCase("success")]
    [TestCase("failed")]
    [TestCase("stalled")]
    [TestCase("missing-git")]
    public async Task ControlledGitUsesSeparateArgumentsAndSupportsSafeRetry(string outcome)
    {
        var host = new UpdateHost { Outcome = outcome };
        using var fixture = new UpdateFixture(host);
        host.Populate = () =>
        {
            if (host.Outcome == "success") fixture.Module("terrain_field");
            else
            {
                var module = Path.Combine(fixture.Project, "community", "terrain_field");
                Directory.CreateDirectory(module);
                File.WriteAllText(Path.Combine(module, "partial.txt"), "incomplete clone");
            }
        };
        try
        {
            if (outcome == "success") await fixture.Run("terrain_field");
            else
            {
                var error = Assert.ThrowsAsync<InvalidOperationException>(async () => await fixture.Run("terrain_field"));
                Assert.That(error!.Message, Does.Contain("retry").IgnoreCase);
                Assert.That(File.Exists(fixture.Registration), Is.False);
                Assert.That(Directory.Exists(Path.Combine(fixture.Project, "community", "terrain_field")), Is.False, "An owned failed clone must not prevent retry");
                host.Outcome = "success";
                await fixture.Run("terrain_field");
            }
            Assert.That(File.ReadAllText(fixture.Registration), Is.EqualTo("terrain_field\n"));
            Assert.That(host.Request!.ArgumentList, Is.EqualTo(new[] { "clone", "--progress", "--depth=1", "https://example.invalid/terrain_field.git", Path.Combine(fixture.Project, "community", "terrain_field") }));
            Assert.Multiple(() =>
            {
                Assert.That(host.Request.UseShellExecute, Is.False);
                Assert.That(host.Request.RedirectStandardOutput && host.Request.RedirectStandardError, Is.True);
                Assert.That(host.Child == null || host.Child.HasExited, Is.True);
            });
        }
        finally { host.Dispose(); }
    }

    private sealed class UpdateHost : CommunityUpdateHost, IDisposable
    {
        internal string? LinkedSuffix { get; init; }
        internal bool FailRegistration { get; set; }
        internal bool FailCleanup { get; init; }
        internal string Outcome { get; set; } = "success";
        internal Action? Populate { get; set; }
        internal ProcessStartInfo? Request { get; private set; }
        internal Process? Child { get; private set; }
        public override TimeSpan CloneTimeout => TimeSpan.FromMilliseconds(Outcome == "stalled" ? 500 : 5000);
        public override FileAttributes GetAttributes(string path) => LinkedSuffix != null && path.EndsWith(LinkedSuffix, StringComparison.OrdinalIgnoreCase)
            ? FileAttributes.ReparsePoint : base.GetAttributes(path);
        public override void MoveRegistration(string source, string destination)
        {
            if (FailRegistration) throw new IOException("Registration is locked");
            base.MoveRegistration(source, destination);
        }
        public override void DeleteIncompleteClone(string path)
        {
            if (FailCleanup) throw new IOException("Incomplete clone is locked");
            base.DeleteIncompleteClone(path);
        }
        public override Process StartProcess(ProcessStartInfo request)
        {
            Request = request;
            Assert.That(request.FileName, Is.EqualTo("git"));
            if (Outcome == "missing-git") throw new Win32Exception(2, "Git not found");
            Populate?.Invoke();
            var script = "[Console]::Out.Write(('o' * 262144)); [Console]::Error.WriteLine('fixture Git diagnostic'); "
                + (Outcome == "stalled" ? "Start-Sleep -Seconds 2; exit 0" : Outcome == "failed" ? "exit 2" : "exit 0");
            var start = new ProcessStartInfo("powershell.exe") { UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true, CreateNoWindow = true };
            foreach (var argument in new[] { "-NoProfile", "-NonInteractive", "-EncodedCommand", Convert.ToBase64String(Encoding.Unicode.GetBytes(script)) }) start.ArgumentList.Add(argument);
            var process = Process.Start(start)!;
            Child?.Dispose();
            Child = Process.GetProcessById(process.Id);
            return process;
        }
        public void Dispose()
        {
            if (Child is { HasExited: false }) { Child.Kill(true); Child.WaitForExit(2000); }
            Child?.Dispose();
        }
    }

    internal sealed class UpdateFixture : IDisposable
    {
        private readonly TestDirectory directory = new();
        private readonly HttpClient http;
        internal string Project { get; }
        internal string Root { get; }
        internal string VersionFile { get; }
        internal string Registration => Path.Combine(Project, "community.cmake");
        internal string Registry { get; set; } = RegistryFor("terrain_field", "noise");
        internal HttpStatusCode Status { get; set; } = HttpStatusCode.OK;
        internal TaskCompletionSource? ResponseGate { get; set; }
        internal int Requests { get; private set; }
        internal ConcurrentQueue<string> Messages { get; } = new();
        internal CommunityUpdate Update { get; }

        internal UpdateFixture(CommunityUpdateHost? host = null)
        {
            Project = Path.Combine(directory.PathName, "User Project");
            Root = Path.Combine(directory.PathName, "Maya Flux");
            Directory.CreateDirectory(Path.Combine(Project, "src"));
            File.WriteAllText(Path.Combine(Project, "CMakeLists.txt"), "project(UserProject)");
            File.WriteAllText(Path.Combine(Project, "src", "user.cpp"), "user content");
            VersionFile = Path.Combine(Root, "lib", "cmake", "MayaFlux", "MayaFluxConfigVersion.cmake");
            Directory.CreateDirectory(Path.GetDirectoryName(VersionFile)!);
            File.WriteAllText(VersionFile, "set(PACKAGE_VERSION \"0.8.0\")");
            http = new HttpClient(new RegistryHandler(this));
            Update = new CommunityUpdate(http, Messages.Enqueue, Root, host);
        }

        internal Task<IReadOnlyList<string>> Run(string names) => Update.RunAsync(Project, [names]);
        internal CommunityUpdate CreateUpdate(Action<string> report) => new(http, report, Root);
        internal string Module(string name, bool requirements = false)
        {
            var module = Path.Combine(Project, "community", name);
            Directory.CreateDirectory(Path.Combine(module, "src"));
            File.WriteAllText(Path.Combine(module, name + ".cmake"), "module fixture");
            File.WriteAllText(Path.Combine(module, "notes.txt"), "keep this");
            var manifest = requirements
                ? $"{{\"name\":\"{name}\",\"min_version\":\"0.5.0\",\"schema_version\":1,\"packages\":{{\"windows\":{{\"winget\":[\"Example.Package\"]}}}}}}"
                : $"{{\"name\":\"{name}\",\"min_version\":\"0.5.0\"}}";
            File.WriteAllText(Path.Combine(module, "community.json"), manifest);
            return module;
        }

        internal static string RegistryFor(params string[] names) => JsonSerializer.Serialize(names.Select(name => new { name, repo = "https://example.invalid/" + name + ".git", min_version = "0.5.0" }));
        public void Dispose() { http.Dispose(); directory.Dispose(); }

        private sealed class RegistryHandler(UpdateFixture fixture) : HttpMessageHandler
        {
            protected override async Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
            {
                fixture.Requests++;
                Assert.That(request.Method, Is.EqualTo(HttpMethod.Get));
                if (fixture.ResponseGate != null) await fixture.ResponseGate.Task.WaitAsync(cancellationToken);
                return new HttpResponseMessage(fixture.Status) { Content = new StringContent(fixture.Registry, Encoding.UTF8, "application/json") };
            }
        }
    }
}
