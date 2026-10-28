using System.Diagnostics;
using NUnit.Framework;
using Weave.Utils;

namespace Weave.Windows.UnitTests;

[TestFixture]
public class RunReportScriptTests
{
    private TestDirectory tools = null!;
    private string fixtureExecutable = null!;

    [OneTimeSetUp]
    public async Task CompileControlledApplication()
    {
        tools = new TestDirectory();
        fixtureExecutable = Path.Combine(tools.PathName, "ReportFixture.exe");
        var source = Path.Combine(tools.PathName, "fixture.cs");
        File.WriteAllText(source, """
            using System;
            using System.IO;
            public class ReportFixture {
                public static int Main(string[] args) {
                    Console.WriteLine("cwd=" + Directory.GetCurrentDirectory());
                    Console.WriteLine("environment=" + Environment.GetEnvironmentVariable("WEAVE_REPORT_FIXTURE"));
                    Console.WriteLine("config=" + (args.Length == 2 && args[0] == "--config" ? args[1] : "INVALID"));
                    Console.WriteLine("stdout: diagnostic output");
                    Console.Error.WriteLine("stderr: crash details");
                    if (Environment.GetEnvironmentVariable("WEAVE_REPORT_NO_LOG") != "1") {
                        Directory.CreateDirectory(Path.Combine("data", "log"));
                        File.AppendAllText(Path.Combine("data", "log", "mayaflux_report.log"), "fresh application report");
                    }
                    return int.Parse(Environment.GetEnvironmentVariable("WEAVE_REPORT_EXIT") ?? "0");
                }
            }
            """);
        var compiler = Path.Combine(tools.PathName, "compile.ps1");
        File.WriteAllText(compiler, "Add-Type -Path " + Quote(source) + " -OutputAssembly " + Quote(fixtureExecutable) + " -OutputType ConsoleApplication -ErrorAction Stop");
        var result = await PowerShell(compiler, tools.PathName, environment: new Dictionary<string, string?> { ["LIB"] = null });
        Assert.That(result.Code, Is.Zero, result.Output + result.Errors);
    }

    [OneTimeTearDown]
    public void RemoveControlledApplication() => tools?.Dispose();

    [TestCase("build")]
    [TestCase("build/Debug")]
    [TestCase("build/Release")]
    [TestCase("build/RelWithDebInfo")]
    [TestCase("build/MinSizeRel")]
    public async Task RunsStandardLayoutsWithInheritedEnvironmentAndBothOutputChannels(string layout)
    {
        using var fixture = CreateFixture(layout);
        var result = await Run(fixture);
        Assert.That(result.Code, Is.Zero, result.Output + result.Errors);
        var capture = File.ReadAllText(Path.Combine(fixture.Project, "mayaflux_run.txt"));
        Assert.Multiple(() =>
        {
            Assert.That(result.Code, Is.Zero, result.Output + result.Errors);
            Assert.That(capture, Does.Contain("cwd=" + fixture.Project).And.Contain("environment=terminal value"));
            Assert.That(capture, Does.Contain("config=" + fixture.Config).And.Contain("stdout: diagnostic output").And.Contain("stderr: crash details"));
            Assert.That(File.ReadAllText(fixture.Report), Is.EqualTo("fresh application report"));
            Assert.That(result.Output, Does.Contain(fixture.Report).And.Contain(Path.Combine(fixture.Project, "mayaflux_run.txt")));
            Assert.That(File.ReadAllText(Path.Combine(fixture.Project, "mayaflux.json")), Is.EqualTo("{\"custom\":true}"));
        });
    }

    [Test]
    public async Task GeneratedProjectNameRunsWithoutACmakeCache()
    {
        using var fixture = CreateFixture();
        var script = Path.Combine(fixture.Project, "run-report.ps1");
        File.WriteAllText(script, File.ReadAllText(script).Replace("@PROJECT_NAME@", "ReportFixture"));
        File.Delete(Path.Combine(fixture.Project, "build", "CMakeCache.txt"));
        var result = await Run(fixture);
        Assert.That(result.Code, Is.Zero, result.Output + result.Errors);
        Assert.That(File.ReadAllText(fixture.Report), Is.EqualTo("fresh application report"));
    }

    [Test]
    public async Task PreservesPreviousLogsAndReturnsApplicationFailure()
    {
        using var fixture = CreateFixture();
        Directory.CreateDirectory(Path.GetDirectoryName(fixture.Report)!);
        File.WriteAllText(fixture.Report, "previous report");
        File.WriteAllText(Path.Combine(fixture.Project, "mayaflux_run.txt"), "previous console");
        var result = await Run(fixture, exitCode: 37);
        Assert.Multiple(() =>
        {
            Assert.That(result.Code, Is.EqualTo(37), result.Output + result.Errors);
            Assert.That(File.ReadAllText(fixture.Report), Is.EqualTo("fresh application report"));
            Assert.That(Directory.GetFiles(Path.GetDirectoryName(fixture.Report)!, "mayaflux_report.log.previous-*").Select(File.ReadAllText), Is.EqualTo(new[] { "previous report" }));
            Assert.That(Directory.GetFiles(fixture.Project, "mayaflux_run.txt.previous-*").Select(File.ReadAllText), Is.EqualTo(new[] { "previous console" }));
            Assert.That(File.ReadAllText(Path.Combine(fixture.Project, "mayaflux_run.txt")), Does.Contain("stderr: crash details"));
            Assert.That(result.Output, Does.Contain("37").And.Contain(fixture.Report));
        });
    }

    [Test]
    public async Task AmbiguousLayoutsRequireSelectionWithoutTouchingOldLogs()
    {
        using var fixture = CreateFixture("build/Debug");
        CopyApplication(fixture.Project, "build/Release");
        File.WriteAllText(Path.Combine(fixture.Project, "mayaflux_run.txt"), "previous console");
        var result = await Run(fixture);
        Assert.Multiple(() =>
        {
            Assert.That(result.Code, Is.EqualTo(1));
            Assert.That(result.Output, Does.Contain("multiple").And.Contain("-Executable"));
            Assert.That(File.ReadAllText(Path.Combine(fixture.Project, "mayaflux_run.txt")), Is.EqualTo("previous console"));
        });
        result = await Run(fixture, arguments: ["-Configuration", "Release"]);
        Assert.That(result.Code, Is.Zero, result.Output + result.Errors);
    }

    [Test]
    public async Task ExplicitExecutableSupportsCustomBuildFoldersWithoutCache()
    {
        using var fixture = CreateFixture("custom output");
        File.Delete(Path.Combine(fixture.Project, "build", "CMakeCache.txt"));
        var result = await Run(fixture, arguments: ["-Executable", @"custom output\ReportFixture.exe", "-MayaFluxRoot", fixture.Install], setRoot: false);
        Assert.That(result.Code, Is.Zero, result.Output + result.Errors);
        Assert.That(File.ReadAllText(Path.Combine(fixture.Project, "mayaflux_run.txt")), Does.Contain("config=" + fixture.Config));
    }

    [TestCase("missing-executable", "Build the project")]
    [TestCase("missing-config", "report configuration")]
    [TestCase("malformed-config", "JSON")]
    [TestCase("unsafe-name", "project name")]
    [TestCase("drive-relative-executable", "drive-relative")]
    [TestCase("missing-explicit-executable", "executable")]
    public async Task InvalidInputsFailClearlyWithoutLaunchingOrRotatingLogs(string scenario, string message)
    {
        using var fixture = CreateFixture();
        string[] arguments = [];
        switch (scenario)
        {
            case "missing-executable": File.Delete(Path.Combine(fixture.Project, "build", "ReportFixture.exe")); break;
            case "missing-config": File.Delete(fixture.Config); break;
            case "malformed-config": File.WriteAllText(fixture.Config, "{broken"); break;
            case "unsafe-name": arguments = ["-ProjectName", "../outside"]; break;
            case "drive-relative-executable": arguments = ["-Executable", "C:outside.exe"]; break;
            case "missing-explicit-executable": arguments = ["-Executable", "missing.exe"]; break;
        }
        File.WriteAllText(Path.Combine(fixture.Project, "mayaflux_run.txt"), "previous console");
        var result = await Run(fixture, arguments: arguments);
        Assert.Multiple(() =>
        {
            Assert.That(result.Code, Is.EqualTo(1), result.Output + result.Errors);
            Assert.That(result.Output, Does.Contain(message));
            Assert.That(File.Exists(fixture.Report), Is.False);
            Assert.That(File.ReadAllText(Path.Combine(fixture.Project, "mayaflux_run.txt")), Is.EqualTo("previous console"));
        });
    }

    [Test]
    public async Task FailedLaunchReportsAnErrorAndKeepsTheConsoleCapture()
    {
        using var fixture = CreateFixture();
        File.WriteAllText(Path.Combine(fixture.Project, "build", "ReportFixture.exe"), "not an executable");
        var result = await Run(fixture);
        Assert.That(result.Code, Is.EqualTo(1));
        Assert.That(result.Output, Does.Contain("[ERROR]"));
        Assert.That(File.ReadAllText(Path.Combine(fixture.Project, "mayaflux_run.txt")), Does.Contain("[ERROR]"));
    }

    [Test]
    public async Task EarlyCrashKeepsOutputAndExplainsTheMissingApplicationReport()
    {
        using var fixture = CreateFixture();
        var result = await Run(fixture, exitCode: 23, noLog: true);
        Assert.That(result.Code, Is.EqualTo(23), result.Output + result.Errors);
        Assert.That(result.Output, Does.Contain("not created").And.Contain("mayaflux_run.txt"));
        Assert.That(File.ReadAllText(Path.Combine(fixture.Project, "mayaflux_run.txt")), Does.Contain("stderr: crash details"));
    }

    private Fixture CreateFixture(string layout = "build")
    {
        var fixture = new Fixture();
        Directory.CreateDirectory(Path.Combine(fixture.Project, "build"));
        Directory.CreateDirectory(Path.GetDirectoryName(fixture.Config)!);
        File.WriteAllText(fixture.Config, "{\"journal\":{}}");
        File.WriteAllText(Path.Combine(fixture.Project, "mayaflux.json"), "{\"custom\":true}");
        File.WriteAllText(Path.Combine(fixture.Project, "build", "CMakeCache.txt"), "CMAKE_PROJECT_NAME:STATIC=ReportFixture\r\n");
        using var resource = typeof(ResourceExtractor).Assembly.GetManifestResourceStream("Weave.Resources.templates.run-report.ps1");
        Assert.That(resource, Is.Not.Null, "run-report.ps1 must be embedded in Weave");
        using (var file = File.Create(Path.Combine(fixture.Project, "run-report.ps1"))) resource!.CopyTo(file);
        CopyApplication(fixture.Project, layout);
        return fixture;
    }

    private void CopyApplication(string project, string layout)
    {
        var output = Path.Combine(project, layout.Replace('/', Path.DirectorySeparatorChar));
        Directory.CreateDirectory(output);
        File.Copy(fixtureExecutable, Path.Combine(output, "ReportFixture.exe"));
    }

    private static Task<Result> Run(Fixture fixture, int exitCode = 0, string[]? arguments = null, bool setRoot = true, bool noLog = false)
        => PowerShell(Path.Combine(fixture.Project, "run-report.ps1"), fixture.Directory.PathName, arguments,
            new Dictionary<string, string?> { ["MAYAFLUX_ROOT"] = setRoot ? fixture.Install : null, ["WEAVE_REPORT_FIXTURE"] = "terminal value", ["WEAVE_REPORT_EXIT"] = exitCode.ToString(), ["WEAVE_REPORT_NO_LOG"] = noLog ? "1" : "0" });

    private static async Task<Result> PowerShell(string script, string workingDirectory, string[]? arguments = null, Dictionary<string, string?>? environment = null)
    {
        var request = new ProcessStartInfo("powershell.exe") { WorkingDirectory = workingDirectory, UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true, CreateNoWindow = true };
        foreach (var argument in new[] { "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", script }.Concat(arguments ?? [])) request.ArgumentList.Add(argument);
        if (environment != null) foreach (var variable in environment) request.Environment[variable.Key] = variable.Value;
        using var process = Process.Start(request)!;
        var stdout = process.StandardOutput.ReadToEndAsync();
        var stderr = process.StandardError.ReadToEndAsync();
        try { await process.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(30)); }
        catch { process.Kill(true); throw; }
        return new Result(process.ExitCode, await stdout, await stderr);
    }

    private static string Quote(string value) => "'" + value.Replace("'", "''") + "'";
    private record Result(int Code, string Output, string Errors);

    private sealed class Fixture : IDisposable
    {
        public TestDirectory Directory { get; } = new();
        public string Project => Path.Combine(Directory.PathName, "User's Project [test] & spaces");
        public string Install => Path.Combine(Directory.PathName, "Maya Flux [install]");
        public string Config => Path.Combine(Install, "share", "MayaFlux", "report", "mayaflux.report.json");
        public string Report => Path.Combine(Project, "data", "log", "mayaflux_report.log");
        public void Dispose() => Directory.Dispose();
    }
}
