using System.Diagnostics;
using NUnit.Framework;
using Weave.Shared.Models;
using Weave.UI.Pages;
using Weave.Utils;

namespace Weave.Windows.UnitTests;

[TestFixture]
public class CompletionStepTests
{
    private sealed class Host : CompletionHost
    {
        internal override string LogPath => @"D:\User Logs\weave install.log";
        internal bool Exists = true;
        internal bool Starts = true;
        internal bool Throws;
        internal bool ReadThrows;
        internal ProcessStartInfo? Request;
        internal override bool FileExists(string path)
        {
            if (ReadThrows) throw new UnauthorizedAccessException("Log access denied");
            return Exists;
        }
        internal override bool StartProcess(ProcessStartInfo info)
        {
            Request = info;
            if (Throws) throw new IOException("Viewer start failed");
            return Starts;
        }
    }

    [Test]
    public void OpeningLogPassesSpacedPathAsOneArgument()
    {
        var host = new Host();
        Assert.That(new CompletionStep(host).OpenLog(out _), Is.True);
        Assert.That(host.Request, Is.Not.Null);
        Assert.That(host.Request!.FileName, Is.EqualTo("notepad.exe"));
        Assert.That(host.Request.ArgumentList.ToArray(), Is.EqualTo(new[] { @"D:\User Logs\weave install.log" }));
    }

    [TestCase("missing")]
    [TestCase("start")]
    [TestCase("throw")]
    [TestCase("read")]
    public void LogViewerFailuresReturnUnderstandableError(string failure)
    {
        var host = new Host { Exists = failure != "missing", Starts = failure != "start", Throws = failure == "throw", ReadThrows = failure == "read" };
        var step = new CompletionStep(host);
        string? error = null;
        bool success = true;
        Assert.That(() => success = step.OpenLog(out error), Throws.Nothing);
        Assert.That(success, Is.False);
        Assert.That(error, Is.Not.Null.And.Not.Empty);
    }

    [Test]
    public void CompletionDoesNotOverwriteExistingInstallationLog()
    {
        using var directory = new TestDirectory();
        var path = Path.Combine(directory.PathName, "install.log");
        File.WriteAllText(path, "Existing installation diagnostics");
        _ = new CompletionStep(new LogFileHost(path));
        Assert.That(File.ReadAllText(path), Is.EqualTo("Existing installation diagnostics"));
    }

    private sealed class LogFileHost(string path) : CompletionHost
    {
        internal override string LogPath => path;
    }

    [Test]
    public void InstructionsUseSupportedWindowsGuiAndPackagedBuildPresets()
    {
        var instructions = CompletionStep.Instructions(new InstallationConfig());
        Assert.Multiple(() =>
        {
            Assert.That(instructions, Does.Not.Contain("weave --version"));
            Assert.That(instructions, Does.Not.Contain("weave new"));
            Assert.That(instructions, Does.Contain("cmake --preset release"));
            Assert.That(instructions, Does.Contain("cmake --build --preset release"));
            Assert.That(instructions, Does.Contain("Projects"));
            Assert.That(instructions, Does.Contain("developer shell"));
        });
    }

    [Test]
    public void RebootInstructionsDoNotSuggestImmediatelyBuilding()
    {
        var instructions = CompletionStep.Instructions(new InstallationConfig { NeedsReboot = true });
        Assert.That(instructions, Does.Contain("Restart Windows"));
    }

    [Test]
    public void SkippedDependenciesAreDisclosedBeforeNextActions()
    {
        var config = new InstallationConfig { SkipDependencies = true };
        Assert.That(CompletionStep.Subtitle(config), Does.Contain("Dependencies were skipped"));
        Assert.That(CompletionStep.Instructions(config), Does.Contain("required dependencies"));
    }

    [Test]
    public void ExistingInstallationIsDescribedAsConfigured()
    {
        var config = new InstallationConfig { SkipDownload = true };
        Assert.That(CompletionStep.Subtitle(config), Does.Contain("configured"));
    }
}
