using System.Windows.Forms;
using NUnit.Framework;
using Weave.Shared.Models;
using Weave.Modes;
using Weave.UI.Layout;
using Weave.UI.Pages;
using Weave.Utils;

namespace Weave.Windows.UnitTests;

[TestFixture]
public class InstallationRecoveryTests
{
    private static readonly string[] CoreFiles = [@"lib\MayaFluxLib.lib", @"bin\MayaFluxLib.dll", @"include\MayaFlux\MayaFlux.hpp", @"lib\cmake\MayaFlux\MayaFluxConfig.cmake", @"lib\cmake\MayaFlux\MayaFluxConfigVersion.cmake"];

    [Test]
    public void CorePayloadVerificationRequiresNonemptyFilesAndReportsRepairGuidance()
    {
        using var directory = new TestDirectory();
        var logger = new Logger(Path.Combine(directory.PathName, "install.log"));
        Populate(directory.PathName);
        Assert.That(FileOperations.VerifyInstallation(directory.PathName, logger), Is.True);
        File.WriteAllText(Path.Combine(directory.PathName, @"lib\MayaFluxLib.lib"), "");
        Assert.That(FileOperations.VerifyInstallation(directory.PathName, logger), Is.False);
        foreach (var relative in CoreFiles) File.Delete(Path.Combine(directory.PathName, relative));
        Assert.That(FileOperations.VerifyInstallation(directory.PathName, logger), Is.False);
        Assert.Multiple(() =>
        {
            foreach (var relative in CoreFiles) Assert.That(logger.GetLogs(), Has.Some.Contains(relative));
            Assert.That(logger.GetLogs(), Has.Some.Contains("reinstall"));
        });
    }

    [Test]
    public void ReopeningLoggerPreservesFailureAndRetryDiagnostics()
    {
        using var directory = new TestDirectory();
        var path = Path.Combine(directory.PathName, "install.log");
        new Logger(path).Log("Download failed: connection lost");
        new Logger(path).Log("Retry completed");
        Assert.That(File.ReadAllText(path), Does.Contain("connection lost").And.Contain("Retry completed"));
    }

    [TestCase("dependencies")]
    [TestCase("environment")]
    [TestCase("templates")]
    [Apartment(ApartmentState.STA)]
    public void LaterScreensPersistProgressThroughInstallationCallback(string kind)
    {
        using var directory = new TestDirectory();
        var logger = new Logger(Path.Combine(directory.PathName, "install.log"));
        var (environment, config) = FakeEnvironmentSetupHost.Healthy();
        IInstallationStep step = kind switch
        {
            "dependencies" => new DependenciesStep(new FakeDependenciesHost()),
            "environment" => new EnvironmentSetupStep(environment),
            _ => new TemplatesInstallStep(new FakeResourceExtractionHost())
        };
        using var screen = new InstallationScreen(step, config, logger.Log);
        PumpUntil(() => screen.Button("Next >").Enabled);
        Assert.That(File.ReadAllText(logger.GetLogFile()), Does.Contain(kind switch
        {
            "dependencies" => "All dependencies are usable",
            "environment" => "Environment Setup Complete",
            _ => "Template & Tool Installation Complete"
        }));
    }

    [TestCase(false)]
    [TestCase(true)]
    [Apartment(ApartmentState.STA)]
    public void FailedScreenOffersRetryAndPersistsBothAttempts(bool download)
    {
        using var directory = new TestDirectory();
        var logger = new Logger(Path.Combine(directory.PathName, "install.log"));
        var config = new InstallationConfig { MayaFluxRoot = FakeMayaFluxDownloadHost.InstallRoot, TempDirectory = FakeMayaFluxDownloadHost.TemporaryRoot };
        var systemHost = new FakeSystemCheckHost { SevenZipPresent = false, DownloadError = new IOException("Connection lost") };
        var downloadHost = new FakeMayaFluxDownloadHost { MetadataError = new IOException("Connection lost") };
        IInstallationStep step = download ? new MayaFluxDownloadStep(ReleaseType.Stable, downloadHost) : new SystemCheckStep(systemHost);
        using var screen = new InstallationScreen(step, config, logger.Log);
        var next = screen.Button("Next >");
        PumpUntil(() => screen.Messages.Contains("Connection lost"));
        var retry = screen.Button("Retry");
        PumpUntil(() => retry.Enabled);
        Assert.That(next.Enabled, Is.False);
        systemHost.DownloadError = null;
        downloadHost.MetadataError = null;
        retry.PerformClick();
        PumpUntil(() => next.Enabled);
        Assert.That(File.ReadAllText(logger.GetLogFile()), Does.Contain("Connection lost").And.Contain(download ? "Download Complete" : "All prerequisites met"));
    }

    [TestCase(false)]
    [TestCase(true)]
    [Apartment(ApartmentState.STA)]
    public void LiveMicrosoftInstallerKeepsRetryDisabledUntilItFinishes(bool waitError)
    {
        var lifetime = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var host = new FakeSystemCheckHost { MsvcPresent = false, InstallMsvcFiles = true, MsvcWaitCompleted = false, MsvcLifetime = lifetime.Task, MsvcExitCode = 3010 };
        if (waitError) host.MsvcWaitError = new IOException("Could not observe installer wait");
        var config = new InstallationConfig();
        var step = new SystemCheckStep(host);
        var finished = false;
        step.ProceedChanged += _ => finished = true;
        using var screen = new InstallationScreen(step, config, _ => { });
        try
        {
            PumpUntil(() => finished);
            var retry = screen.Button("Retry");
            Assert.That(retry.Enabled, Is.False, screen.Messages);
            Assert.That(step.PerformChecksAsync(new InstallationConfig()).GetAwaiter().GetResult(), Is.False);
            Assert.That(host.Processes.Count(request => request.FileName == "winget"), Is.EqualTo(1));
            host.MsvcPresent = true;
            lifetime.SetResult();
            PumpUntil(() => retry.Enabled);
            retry.PerformClick();
            PumpUntil(() => screen.Button("Next >").Enabled);
            Assert.That(config.NeedsReboot, Is.True);
            Assert.That(host.Processes.Count(request => request.FileName == "winget"), Is.EqualTo(1));
        }
        finally { lifetime.TrySetResult(); Application.DoEvents(); }
    }
    private sealed class InstallationScreen : IDisposable
    {
        private readonly Form form = new();
        private readonly Panel panel = new() { Dock = DockStyle.Fill };

        public InstallationScreen(IInstallationStep step, InstallationConfig config, Action<string> log)
        {
            form.Controls.Add(panel);
            step.BuildUI(new LayoutManager(panel), config, log, () => { });
            form.Show();
        }

        public Button Button(string text) => Children(panel).OfType<Button>().Single(button => button.Text == text);
        public string Messages => string.Join("\n", Children(panel).OfType<TextBox>().Select(box => box.Text));
        public void Dispose() => form.Dispose();

        private static IEnumerable<Control> Children(Control control) => control.Controls.Cast<Control>()
            .SelectMany(child => new[] { child }.Concat(Children(child)));
    }

    private static void PumpUntil(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow.AddSeconds(5);
        while (!condition() && DateTime.UtcNow < deadline) { Application.DoEvents(); Thread.Sleep(5); }
        Assert.That(condition(), Is.True, "The installation screen did not reach the expected state");
    }
    private static void Populate(string root)
    {
        foreach (var relative in CoreFiles)
        {
            var path = Path.Combine(root, relative);
            Directory.CreateDirectory(Path.GetDirectoryName(path)!);
            File.WriteAllText(path, "fixture payload");
        }
    }
}
