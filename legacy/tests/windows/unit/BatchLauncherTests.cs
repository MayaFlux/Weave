using System.Diagnostics;
using NUnit.Framework;
using Weave.UI.Project;
using Weave.Utils;

namespace Weave.Windows.UnitTests;

[TestFixture]
public class BatchLauncherTests
{
    [TestCase("validate", false)]
    [TestCase("validate", true)]
    [TestCase("run-report", false)]
    [TestCase("run-report", true)]
    public async Task GeneratedLaunchersForwardArgumentsEnvironmentAndExitCode(string name, bool delayedExpansion)
    {
        using var directory = new TestDirectory();
        var project = CreateProject(directory);
        var launcher = Path.Combine(project, name + ".bat");
        Assert.That(File.Exists(launcher), Is.True, "Project creation must provide " + name + ".bat");
        File.WriteAllText(Path.Combine(project, name + ".ps1"), "param([string]$Value)\r\nWrite-Output ('argument=' + $Value)\r\nWrite-Output ('environment=' + $env:WEAVE_BATCH_VALUE)\r\nWrite-Output ('script=' + $PSScriptRoot)\r\nexit 37\r\n");
        var result = await Run(launcher, directory.PathName, delayedExpansion);
        Assert.Multiple(() =>
        {
            Assert.That(result.Code, Is.EqualTo(37), result.Output + result.Errors);
            Assert.That(result.Output, Does.Contain("argument=quoted argument").And.Contain("environment=terminal value").And.Contain("script=" + project));
            Assert.That(result.Errors, Is.Empty);
        });
    }

    [TestCase("validate")]
    [TestCase("run-report")]
    public async Task MissingSiblingScriptFailsClearly(string name)
    {
        using var directory = new TestDirectory();
        var project = CreateProject(directory);
        var launcher = Path.Combine(project, name + ".bat");
        Assert.That(File.Exists(launcher), Is.True);
        File.Delete(Path.Combine(project, name + ".ps1"));
        var result = await Run(launcher, directory.PathName, false);
        Assert.That(result.Code, Is.Not.Zero);
        Assert.That(result.Output + result.Errors, Does.Contain(name + ".ps1"));
    }

    private static string CreateProject(TestDirectory directory)
    {
        var install = Path.Combine(directory.PathName, "Installation");
        ResourceExtractor.ExtractTemplates(install);
        var parent = Path.Combine(directory.PathName, "Project folders [test] & spaces !");
        Directory.CreateDirectory(parent);
        return new ProjectCreationService(Path.Combine(install, "share", "weave", "templates"), new ProjectCreationHost(), _ => { })
            .Create("LauncherProject", parent, withVscode: false);
    }

    private static async Task<(int Code, string Output, string Errors)> Run(string launcher, string workingDirectory, bool delayedExpansion)
    {
        var request = new ProcessStartInfo("cmd.exe")
        {
            UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true,
            WorkingDirectory = workingDirectory,
            Arguments = delayedExpansion ? "/d /v:on /s /c \"\"!WEAVE_BATCH_FILE!\" -Value \"quoted argument\"\"" : "/d /v:off /s /c \"\"%WEAVE_BATCH_FILE%\" -Value \"quoted argument\"\""
        };
        request.Environment["WEAVE_BATCH_FILE"] = launcher;
        request.Environment["WEAVE_BATCH_VALUE"] = "terminal value";
        using var process = Process.Start(request)!;
        var stdout = process.StandardOutput.ReadToEndAsync();
        var stderr = process.StandardError.ReadToEndAsync();
        try { await process.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(30)); }
        catch { process.Kill(true); throw; }
        return (process.ExitCode, await stdout, await stderr);
    }
}
