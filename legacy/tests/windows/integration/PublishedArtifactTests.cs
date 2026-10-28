using System.Diagnostics;
using System.Net;
using System.Reflection.PortableExecutable;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using NUnit.Framework;

namespace Weave.Windows.IntegrationTests;

[TestFixture, NonParallelizable]
public class PublishedArtifactTests
{
    private PublishedBundle bundle = null!;
    [OneTimeSetUp] public void OpenArtifact() => bundle = new PublishedBundle();
    [OneTimeTearDown] public void CloseArtifact() => bundle?.Dispose();

    [Test]
    public void DistributionContainsAnX64GuiAndItsOwnDotNet8Runtime()
    {
        using var pe = bundle.ReadExecutable();
        Assert.Multiple(() =>
        {
            Assert.That(pe.PEHeaders.CoffHeader.Machine, Is.EqualTo(Machine.Amd64));
            Assert.That(pe.PEHeaders.PEHeader!.Magic, Is.EqualTo(PEMagic.PE32Plus));
            Assert.That(pe.PEHeaders.PEHeader.Subsystem, Is.EqualTo(Subsystem.WindowsGui));
            Assert.That(bundle.Names, Is.SupersetOf(new[] { "System.Private.CoreLib.dll", "System.Windows.Forms.dll", "Newtonsoft.Json.dll" }));
            Assert.That(NativeExports(pe), Is.SupersetOf(new[] { "DotNetRuntimeInfo", "g_CLREngineMetrics", "CLRJitAttachState" }),
                "The Windows single-file host must contain the statically linked CLR and JIT.");
        });
        using var runtime = JsonDocument.Parse(bundle.Read("Weave.runtimeconfig.json"));
        var frameworks = runtime.RootElement.GetProperty("runtimeOptions").GetProperty("includedFrameworks");
        Assert.That(frameworks.EnumerateArray().Select(value => value.GetProperty("name").GetString()),
            Is.EquivalentTo(new[] { "Microsoft.NETCore.App", "Microsoft.WindowsDesktop.App" }));
        Assert.That(frameworks.EnumerateArray().All(value => value.GetProperty("version").GetString()!.StartsWith("8.0.")), Is.True);
        using var deps = JsonDocument.Parse(bundle.Read("Weave.deps.json"));
        Assert.That(deps.RootElement.GetProperty("runtimeTarget").GetProperty("name").GetString(), Does.EndWith("/win-x64"));
    }

    private static List<string> NativeExports(PEReader pe)
    {
        var directory = pe.GetSectionData(pe.PEHeaders.PEHeader!.ExportTableDirectory.RelativeVirtualAddress).GetReader();
        directory.Offset = 24;
        var count = directory.ReadInt32();
        directory.Offset = 32;
        var names = pe.GetSectionData(directory.ReadInt32()).GetReader();
        var exports = new List<string>();
        for (var index = 0; index < count; index++)
        {
            var reader = pe.GetSectionData(names.ReadInt32()).GetReader();
            var name = new StringBuilder();
            byte next;
            while ((next = reader.ReadByte()) != 0) name.Append((char)next);
            exports.Add(name.ToString());
        }
        return exports;
    }

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] private static extern nint LoadLibraryEx(string file, nint handle, uint flags);
    [DllImport("kernel32.dll")] private static extern nint FindResource(nint module, nint name, nint type);
    [DllImport("kernel32.dll")] private static extern uint SizeofResource(nint module, nint resource);
    [DllImport("kernel32.dll")] private static extern nint LoadResource(nint module, nint resource);
    [DllImport("kernel32.dll")] private static extern nint LockResource(nint resource);
    [DllImport("kernel32.dll")] private static extern bool FreeLibrary(nint module);

    [Test]
    public void ProjectOperationsDoNotRequireAdministratorElevation()
    {
        var module = LoadLibraryEx(bundle.Executable, 0, 0x22);
        Assert.That(module, Is.Not.EqualTo(0));
        try
        {
            var resource = FindResource(module, 1, 24);
            Assert.That(resource, Is.Not.EqualTo(0));
            var bytes = new byte[SizeofResource(module, resource)];
            Marshal.Copy(LockResource(LoadResource(module, resource)), bytes, 0, bytes.Length);
            var manifest = System.Xml.Linq.XDocument.Parse(Encoding.UTF8.GetString(bytes).TrimEnd('\0'));
            Assert.That(manifest.Descendants().Single(element => element.Name.LocalName == "requestedExecutionLevel").Attribute("level")!.Value,
                Is.EqualTo("asInvoker"));
        }
        finally { FreeLibrary(module); }
    }

    [Test]
    public void PackagedTemplatesExtractWithoutDeveloperInstalledResources()
    {
        var root = bundle.PrepareInstallation();
        var templates = Path.Combine(root, "share", "weave", "templates");
        string[] required = ["CMakeLists.txt", "CMakePresets.json", "main.cpp", "user_project.hpp", ".gitignore", "validate.ps1", "run-report.ps1",
            @"cmake\mayaflux.cmake", @"cmake\shaders.cmake", @"cmake\build_community.cmake", @"community\module.cmake",
            @"community\community.json", @"community\CMakePresets.json", @"community\test\CMakeLists.txt",
            @"vscode\settings.json", @"vscode\tasks.json", @"vscode\launch.json"];
        Assert.Multiple(() =>
        {
            foreach (var relative in required)
            {
                var path = Path.Combine(templates, relative);
                Assert.That(File.Exists(path), Is.True, relative);
                if (File.Exists(path)) Assert.That(new FileInfo(path).Length, Is.GreaterThan(0), relative);
            }
            Assert.That(bundle.ApplicationAssembly.GetManifestResourceNames(), Does.Contain("Weave.Resources.weave.ico"));
        });
    }

    [Test]
    public void AuthenticodeIntegrityIsValidWhenSigned()
    {
        var start = new ProcessStartInfo(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.System), @"WindowsPowerShell\v1.0\powershell.exe"))
        { UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true, CreateNoWindow = true };
        start.Environment["WEAVE_SIGNED_FILE"] = bundle.Executable;
        using var pe = bundle.ReadExecutable();
        start.Environment["WEAVE_HAS_SIGNATURE"] = pe.PEHeaders.PEHeader!.CertificateTableDirectory.Size > 0 ? "1" : "0";
        start.Environment["PSModulePath"] = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.System), @"WindowsPowerShell\v1.0\Modules");
        var script = "$ErrorActionPreference = 'Stop'; $s = Get-AuthenticodeSignature -LiteralPath $env:WEAVE_SIGNED_FILE; if ($s.Status -eq 'NotSigned' -and $env:WEAVE_HAS_SIGNATURE -eq '0') { Write-Output 'Unsigned artifact: integrity checks apply when signed'; exit 0 }; if ($s.Status -ne 'Valid' -or $null -eq $s.SignerCertificate) { Write-Error ('Invalid Authenticode signature: ' + $s.Status); exit 1 }; Write-Output ('Valid signature: ' + $s.SignerCertificate.Subject)";
        foreach (var argument in new[] { "-NoProfile", "-NonInteractive", "-EncodedCommand", Convert.ToBase64String(Encoding.Unicode.GetBytes(script)) }) start.ArgumentList.Add(argument);
        using var process = Process.Start(start)!;
        var output = process.StandardOutput.ReadToEndAsync();
        var errors = process.StandardError.ReadToEndAsync();
        if (!process.WaitForExit(30000)) { process.Kill(true); Assert.Fail("Authenticode verification timed out."); }
        TestContext.Out.WriteLine(output.GetAwaiter().GetResult());
        Assert.That(process.ExitCode, Is.Zero, errors.GetAwaiter().GetResult());
    }

    [Test]
    public void PackagedGuiExplainsMissingCriticalTemplatesAndAllowsRepairAndRetry()
    {
        var root = bundle.PrepareInstallation();
        var template = Path.Combine(root, "share", "weave", "templates", "main.cpp");
        var original = File.ReadAllBytes(template);
        File.Delete(template);
        var destination = Path.Combine(bundle.Root, "Repair destinations");
        Directory.CreateDirectory(destination);
        using var gui = new PackagedGui(bundle, root);
        var window = gui.Open("Create Project");
        var inputs = gui.Inputs(window, 2);
        PackagedGui.Set(inputs[0], "RepairedProject");
        PackagedGui.Set(inputs[1], destination);
        var create = gui.Control(window, "Create Project");
        PackagedGui.Click(create);
        Assert.That(gui.Dismiss("Error"), Does.Contain("main.cpp").And.Contain("reinstall").IgnoreCase);
        Assert.That(Directory.Exists(Path.Combine(destination, "RepairedProject")), Is.False);
        Assert.That(PackagedGui.Enabled(create), Is.True);
        File.WriteAllBytes(template, original);
        PackagedGui.Click(create);
        Assert.That(gui.Dismiss("Success"), Does.Contain("RepairedProject"));
        gui.WaitForExit();
        Assert.That(File.Exists(Path.Combine(destination, "RepairedProject", "src", "main.cpp")), Is.True);
    }

    [Test]
    public void PackagedGuiCreatesAProjectAndPreservesAnExistingDestinationOnRetry()
    {
        var installation = bundle.PrepareInstallation();
        var destination = Path.Combine(bundle.Root, "Project destinations");
        Directory.CreateDirectory(destination);
        var project = Path.Combine(destination, "PackagedProject");
        using (var gui = new PackagedGui(bundle, installation))
        {
            var window = gui.Open("Create Project");
            var inputs = gui.Inputs(window, 2);
            Assert.That(inputs, Has.Length.EqualTo(2));
            PackagedGui.Set(inputs[0], "PackagedProject");
            PackagedGui.Set(inputs[1], destination);
            PackagedGui.Click(gui.Control(window, "Create Project"));
            Assert.That(gui.Dismiss("Success"), Does.Contain(project));
            gui.WaitForExit();
        }
        Assert.Multiple(() =>
        {
            foreach (var relative in new[] { "CMakeLists.txt", "CMakePresets.json", "validate.ps1", "run-report.ps1", "validate.bat", "run-report.bat", @"src\main.cpp", @"src\user_project.hpp", @".vscode\launch.json" })
                Assert.That(File.Exists(Path.Combine(project, relative)), Is.True, relative);
            Assert.That(File.ReadAllText(Path.Combine(project, "CMakeLists.txt")), Does.Contain("PackagedProject").And.Not.Contain("@PROJECT_NAME@"));
            Assert.That(File.ReadAllText(Path.Combine(project, "run-report.ps1")), Does.Contain("$ProjectName = 'PackagedProject'").And.Not.Contain("@PROJECT_NAME@"));
        });
        var launcherRequest = new ProcessStartInfo("cmd.exe")
        {
            UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true,
            WorkingDirectory = bundle.Root, Arguments = "/d /s /c \"\"%WEAVE_REPORT_LAUNCHER%\"\""
        };
        launcherRequest.Environment["WEAVE_REPORT_LAUNCHER"] = Path.Combine(project, "run-report.bat");
        launcherRequest.Environment["MAYAFLUX_ROOT"] = installation;
        launcherRequest.Environment["PATH"] = Environment.GetFolderPath(Environment.SpecialFolder.System);
        using (var launcher = Process.Start(launcherRequest)!)
        {
            var output = launcher.StandardOutput.ReadToEndAsync();
            var errors = launcher.StandardError.ReadToEndAsync();
            if (!launcher.WaitForExit(30000)) { launcher.Kill(true); Assert.Fail("The packaged report launcher timed out."); }
            Assert.That(launcher.ExitCode, Is.EqualTo(1), errors.GetAwaiter().GetResult());
            Assert.That(output.GetAwaiter().GetResult(), Does.Contain("Build the project first"));
            Assert.That(File.Exists(Path.Combine(project, "mayaflux_run.txt")), Is.False);
        }
        var main = Path.Combine(project, "src", "main.cpp");
        File.WriteAllText(main, "User-owned changes");
        using var retry = new PackagedGui(bundle, installation);
        var retryWindow = retry.Open("Create Project");
        var retryInputs = retry.Inputs(retryWindow, 2);
        PackagedGui.Set(retryInputs[0], "PackagedProject");
        PackagedGui.Set(retryInputs[1], destination);
        PackagedGui.Click(retry.Control(retryWindow, "Create Project"));
        Assert.That(retry.Dismiss("Error"), Does.Contain("already exists"));
        Assert.That(File.ReadAllText(main), Is.EqualTo("User-owned changes"));
        Assert.That(PackagedGui.Enabled(retry.Control(retryWindow, "Create Project")), Is.True);
    }

    [Test]
    public void PackagedGuiCreatesAModuleWhenOptionalGitIsMissing()
    {
        var installation = bundle.PrepareInstallation();
        var destination = Path.Combine(bundle.Root, "Module destinations");
        Directory.CreateDirectory(destination);
        using var gui = new PackagedGui(bundle, installation);
        var window = gui.Open("Create Community Module");
        var inputs = gui.Inputs(window, 4);
        Assert.That(inputs, Has.Length.EqualTo(4));
        PackagedGui.Set(inputs[0], "packaged_module");
        PackagedGui.Set(inputs[1], "Packaged module fixture");
        PackagedGui.Set(inputs[2], "0.5.0");
        PackagedGui.Set(inputs[3], destination);
        PackagedGui.Click(gui.Control(window, "Create Module"));
        Assert.That(gui.Dismiss("Success"), Does.Contain("git").IgnoreCase.And.Contain("ready"));
        gui.WaitForExit();
        var module = Path.Combine(destination, "packaged_module");
        Assert.Multiple(() =>
        {
            foreach (var relative in new[] { "community.json", "packaged_module.cmake", "CMakePresets.json", @"src\packaged_module.cpp", @"test\test_packaged_module.cpp" })
                Assert.That(File.Exists(Path.Combine(module, relative)), Is.True, relative);
            Assert.That(Directory.Exists(Path.Combine(module, ".git")), Is.False);
        });
    }

    [Test]
    public void PackagedUpdateGuiReportsInvalidProjectsAndRemainsUsable()
    {
        var root = bundle.PrepareInstallation();
        var invalid = Path.Combine(bundle.Root, "Not a project");
        Directory.CreateDirectory(invalid);
        using var gui = new PackagedGui(bundle, root);
        var window = gui.Open("Update Project (Community Modules)");
        var inputs = gui.Inputs(window, 2);
        Assert.That(inputs, Has.Length.EqualTo(2));
        PackagedGui.Set(inputs[0], invalid);
        PackagedGui.Set(inputs[1], "packaged_module");
        PackagedGui.Click(gui.Control(window, "Add Module"));
        Assert.That(gui.Dismiss("Validation Error"), Does.Contain("no CMakeLists.txt"));
        Assert.That(PackagedGui.Enabled(gui.Control(window, "Add Module")), Is.True);
        Assert.That(Directory.GetFileSystemEntries(invalid), Is.Empty);
    }

    [Test]
    public async Task PublishedUpdateImplementationRegistersExistingModulesOfflineAndIsIdempotent()
    {
        var root = bundle.PrepareInstallation();
        var version = Path.Combine(root, "lib", "cmake", "MayaFlux", "MayaFluxConfigVersion.cmake");
        Directory.CreateDirectory(Path.GetDirectoryName(version)!);
        File.WriteAllText(version, "set(PACKAGE_VERSION \"0.8.0\")");
        var project = Path.Combine(bundle.Root, "Offline update project");
        var module = Path.Combine(project, "community", "packaged_module");
        Directory.CreateDirectory(Path.Combine(module, "src"));
        File.WriteAllText(Path.Combine(project, "CMakeLists.txt"), "project(OfflineUpdate)");
        File.WriteAllText(Path.Combine(module, "packaged_module.cmake"), "# Fixture module");
        File.WriteAllText(Path.Combine(module, "community.json"), "{\"name\":\"packaged_module\",\"min_version\":\"0.5.0\"}");
        File.WriteAllText(Path.Combine(module, "src", "user.cpp"), "User-owned module source");
        var messages = new List<string>();
        using var handler = new OfflineRegistry();
        using var http = new HttpClient(handler);
        var type = bundle.SharedAssembly.GetType("Weave.Shared.CommunityUpdate", throwOnError: true)!;
        var update = Activator.CreateInstance(type, [http, new Action<string>(messages.Add), root, null])!;
        for (var index = 0; index < 2; index++)
        {
            var task = (Task)type.GetMethod("RunAsync")!.Invoke(update, [project, new[] { "packaged_module" }])!;
            await task;
        }
        Assert.Multiple(() =>
        {
            Assert.That(File.ReadAllText(Path.Combine(project, "community.cmake")), Is.EqualTo("packaged_module\n"));
            Assert.That(File.ReadAllText(Path.Combine(module, "src", "user.cpp")), Is.EqualTo("User-owned module source"));
            Assert.That(messages, Has.Some.Contains("already registered"));
            Assert.That(handler.Requests, Is.EqualTo(2));
        });
    }

    private sealed class OfflineRegistry : HttpMessageHandler
    {
        internal int Requests { get; private set; }
        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken token)
        {
            Requests++;
            Assert.That(request.RequestUri!.AbsoluteUri, Is.EqualTo("https://raw.githubusercontent.com/MayaFlux/community-sources-registry/main/registry.json"));
            return Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK)
            { Content = new StringContent("[{\"name\":\"packaged_module\",\"repo\":\"https://example.invalid/packaged_module.git\",\"min_version\":\"0.5.0\"}]") });
        }
    }
}
