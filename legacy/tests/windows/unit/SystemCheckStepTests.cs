using System.ComponentModel;
using System.Diagnostics;
using System.Net;
using System.Runtime.InteropServices;
using NUnit.Framework;
using Weave.Shared.Models;
using Weave.UI.Pages;

namespace Weave.Windows.UnitTests;

[TestFixture]
public sealed class SystemCheckStepTests
{
    [Test]
    public async Task InstalledPrerequisitesAllowProceedingWithoutDownloadsOrInstallers()
    {
        var host = new FakeSystemCheckHost();
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.True);
        Assert.That(result.Readiness, Is.EqualTo(new[] { true }));
        Assert.That(host.Downloads, Is.Empty);
        Assert.That(host.Writes, Is.Empty);
        Assert.That(host.Processes.All(p => p.FileName.EndsWith("vswhere.exe")), Is.True);
    }

    [Test]
    public async Task MissingAdministratorPrivilegesBlockAllInstallationEffects()
    {
        var host = new FakeSystemCheckHost { Administrator = false, SevenZipPresent = false, MsvcPresent = false };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(result.Readiness, Is.EqualTo(new[] { false }));
        Assert.That(result.Messages.Any(m => m.Contains("administrator", StringComparison.OrdinalIgnoreCase)), Is.True);
        Assert.That(host.Downloads, Is.Empty);
        Assert.That(host.Writes, Is.Empty);
        Assert.That(host.Processes, Is.Empty);
    }

    [Test]
    public async Task AdministratorProbeFailureBlocksInstallation()
    {
        var host = new FakeSystemCheckHost { AdministratorError = new UnauthorizedAccessException("Token denied") };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(host.Processes, Is.Empty);
        Assert.That(host.Writes, Is.Empty);
    }

    [TestCase(Architecture.X86)]
    [TestCase(Architecture.Arm64)]
    public async Task UnsupportedArchitectureBlocksInstallersBeforeHostMutation(Architecture architecture)
    {
        var host = new FakeSystemCheckHost
        {
            ArchitectureKind = architecture,
            SevenZipPresent = false,
            MsvcPresent = false
        };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(result.Readiness, Is.EqualTo(new[] { false }));
        Assert.That(host.Downloads, Is.Empty);
        Assert.That(host.Writes, Is.Empty);
        Assert.That(host.Processes, Is.Empty);
    }

    [Test]
    public async Task SevenZipProbeAccessFailureIsReportedWithoutEscapingTheStep()
    {
        var host = new FakeSystemCheckHost { SevenZipProbeError = new UnauthorizedAccessException("Tool path denied") };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(result.Messages.Any(m => m.Contains("denied", StringComparison.OrdinalIgnoreCase)), Is.True);
        Assert.That(host.Downloads, Is.Empty);
    }

    [TestCase(HttpStatusCode.Forbidden)]
    public async Task FailedSevenZipResponseDoesNotExecuteAPayload(HttpStatusCode status)
    {
        var host = new FakeSystemCheckHost { SevenZipPresent = false, DownloadStatus = status };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(host.Writes, Is.Empty);
        Assert.That(host.Processes.Any(IsSevenZipInstaller), Is.False);
        Assert.That(result.Messages.Any(m => m.Contains("Download failed")), Is.True);
    }

    [Test]
    public async Task SevenZipNetworkFailureBlocksProceeding()
    {
        var host = new FakeSystemCheckHost
        {
            SevenZipPresent = false,
            DownloadError = new HttpRequestException("Network unavailable")
        };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(result.Messages.Any(m => m.Contains("Network unavailable")), Is.True);
        Assert.That(host.Processes.Any(IsSevenZipInstaller), Is.False);
    }

    [Test]
    public async Task SevenZipDownloadTimeoutDoesNotExecuteAnInstaller()
    {
        var host = new FakeSystemCheckHost
        {
            SevenZipPresent = false,
            DownloadError = new TaskCanceledException("Download timed out")
        };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(result.Messages.Any(m => m.Contains("timed out")), Is.True);
        Assert.That(host.Processes.Any(IsSevenZipInstaller), Is.False);
    }

    [TestCase(true)]
    [TestCase(false)]
    public async Task EmptyOrNonExecutableDownloadIsNeverExecuted(bool empty)
    {
        var host = new FakeSystemCheckHost
        {
            SevenZipPresent = false,
            DownloadBytes = empty ? [] : "this is an HTML error page"u8.ToArray()
        };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(host.Processes.Any(IsSevenZipInstaller), Is.False);
    }

    [Test]
    public async Task InterruptedDownloadCannotExecuteItsPartialFile()
    {
        var host = new FakeSystemCheckHost { SevenZipPresent = false, InterruptDownload = true };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(host.Processes.Any(IsSevenZipInstaller), Is.False);
        Assert.That(result.Messages.Any(m => m.StartsWith("[ERROR]")), Is.True);
    }

    [Test]
    public async Task InstallerWriteFailureDoesNotStartAProcess()
    {
        var host = new FakeSystemCheckHost
        {
            SevenZipPresent = false,
            WriteError = new IOException("Disk full")
        };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(result.Messages.Any(m => m.Contains("Disk full")), Is.True);
        Assert.That(host.Processes.Any(IsSevenZipInstaller), Is.False);
    }

    [Test]
    public async Task InstallerInATemporaryDirectoryWithSpacesUsesTheCompleteExecutablePath()
    {
        var host = new FakeSystemCheckHost { SevenZipPresent = false, TemporaryDirectory = @"D:\Test User\Temp" };
        var result = await RunAsync(host);
        var installer = host.Processes.Single(IsSevenZipInstaller);

        Assert.That(result.Passed, Is.True);
        Assert.That(installer.FileName, Is.EqualTo(@"D:\Test User\Temp\7z-install.exe"));
        Assert.That(installer.Arguments, Is.EqualTo("/S"));
        Assert.That(host.Downloads.Single().Url, Does.StartWith("https://www.7-zip.org/"));
        Assert.That(host.Downloads.Single().Timeout, Is.GreaterThan(TimeSpan.Zero));
    }

    [Test]
    public async Task PreexistingTemporaryInstallerIsNotOverwrittenOrExecuted()
    {
        var host = new FakeSystemCheckHost { SevenZipPresent = false };
        host.Files[@"D:\Test User\Temp\7z-install.exe"] = "unrelated existing file"u8.ToArray();
        await RunAsync(host);

        Assert.That(host.Files, Does.ContainKey(@"D:\Test User\Temp\7z-install.exe"));
        Assert.That(host.Files[@"D:\Test User\Temp\7z-install.exe"], Is.EqualTo("unrelated existing file"u8.ToArray()));
        Assert.That(host.Processes.Any(p => p.FileName == @"D:\Test User\Temp\7z-install.exe"), Is.False);
    }

    [TestCase(@"C:\")]
    [TestCase("relative directory")]
    [TestCase(@"\\server\share")]
    public async Task UnsafeTemporaryDestinationIsRejectedBeforeWritingAnInstaller(string directory)
    {
        var host = new FakeSystemCheckHost { SevenZipPresent = false, TemporaryDirectory = directory };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(host.Writes, Is.Empty);
        Assert.That(host.Processes.Any(IsSevenZipInstaller), Is.False);
    }

    [Test]
    public async Task SevenZipInstallerStartFailureIsReported()
    {
        var host = new FakeSystemCheckHost { SevenZipPresent = false, SevenZipStartError = new Win32Exception("Executable cannot start") };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(result.Messages.Any(m => m.Contains("cannot start")), Is.True);
    }

    [Test]
    public async Task NullSevenZipProcessDoesNotCountAsInstallation()
    {
        var host = new FakeSystemCheckHost { SevenZipPresent = false, NullSevenZipProcess = true };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(result.Messages.Any(m => m.Contains("Failed to start")), Is.True);
    }

    [TestCase(1603)]
    public async Task SevenZipInstallerFailureCannotBeMaskedByFilesAppearing(int exitCode)
    {
        var host = new FakeSystemCheckHost { SevenZipPresent = false, SevenZipExitCode = exitCode };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(result.Readiness, Is.EqualTo(new[] { false }));
    }

    [Test]
    public async Task SevenZipInstallerTimeoutCannotBecomeSuccess()
    {
        var host = new FakeSystemCheckHost { SevenZipPresent = false, SevenZipWaitCompleted = false };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(result.Messages.Any(m => m.Contains("time", StringComparison.OrdinalIgnoreCase)), Is.True);
    }

    [Test]
    public async Task SevenZipInstallerSuccessWithoutInstalledFilesIsRejected()
    {
        var host = new FakeSystemCheckHost { SevenZipPresent = false, InstallSevenZipFiles = false };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(result.Readiness, Is.EqualTo(new[] { false }));
    }

    [Test]
    public async Task TemporaryInstallerCleanupFailureDoesNotInvalidateInstalledPrerequisites()
    {
        var host = new FakeSystemCheckHost { SevenZipPresent = false, DeleteError = new IOException("File still locked") };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.True);
        Assert.That(host.DeletedFiles, Has.Count.EqualTo(1));
    }

    [TestCase(21999, "Microsoft.VisualStudio.2022.BuildTools")]
    [TestCase(22000, "Microsoft.VisualStudio.BuildTools")]
    public async Task WindowsVersionSelectsTheBuildToolsPackageAndRequiredComponents(int build, string package)
    {
        var host = new FakeSystemCheckHost { MsvcPresent = false, WindowsBuildNumber = build, InstallMsvcFiles = true };
        var result = await RunAsync(host);
        var installer = host.Processes.Single(p => p.FileName == "winget");

        Assert.That(result.Passed, Is.True);
        Assert.That(installer.Arguments, Does.StartWith("install " + package + " "));
        Assert.That(host.Waits.Where(wait => wait.FileName == "winget").All(wait => wait.Milliseconds > 0), Is.True);
        Assert.That(installer.Arguments, Does.Contain("--override \"--wait --passive --norestart "));
        Assert.That(installer.Arguments, Does.Contain("--add Microsoft.VisualStudio.Component.VC.Tools.x86.x64"));
        Assert.That(installer.Arguments, Does.Contain("--add Microsoft.VisualStudio.Component.Windows11SDK.22621"));
        Assert.That(installer.Arguments, Does.Contain("--add Microsoft.VisualStudio.Component.VC.Redist.14.Latest\""));
    }

    [TestCase(" \r\n\t")]
    public async Task EmptyVswhereOutputTriggersBuildToolsAcquisition(string output)
    {
        var host = new FakeSystemCheckHost { MsvcPresent = false, VswhereOutput = output, InstallMsvcFiles = true };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.True);
        Assert.That(host.Processes.Count(p => p.FileName == "winget"), Is.EqualTo(1));
    }

    [Test]
    public async Task MicrosoftPrerequisitesAreExplainedBeforeThePassiveInstallerStarts()
    {
        var host = new FakeSystemCheckHost { MsvcPresent = false, InstallMsvcFiles = true };
        var step = new SystemCheckStep(host);
        var messages = new List<string>();
        step.MessageLogged += messages.Add;
        host.BeforeMsvcStart = () => Assert.That(messages.Any(message =>
            message.Contains("Microsoft C++ libraries and SDK files") &&
            message.Contains("even when LLVM is used") &&
            message.Contains("Weave will continue when it finishes")), Is.True);

        Assert.That(await step.PerformChecksAsync(new InstallationConfig()), Is.True);
        var arguments = host.Processes.Single(process => process.FileName == "winget").Arguments;
        Assert.That(arguments, Does.Contain("--wait --passive --norestart"));
        Assert.That(arguments, Does.Not.Contain("--quiet"));
        Assert.That(arguments, Does.Contain("Microsoft.VisualStudio.Component.VC.Redist.14.Latest"));
        Assert.That(arguments, Does.Not.Contain("--includeRecommended"));
        Assert.That(arguments, Does.Not.Contain("--includeOptional"));
    }

    [Test]
    public async Task MicrosoftInstallerTimeoutBlocksProceedingEvenIfCompilerFilesAppear()
    {
        var host = new FakeSystemCheckHost
        {
            MsvcPresent = false,
            InstallMsvcFiles = true,
            MsvcWaitCompleted = false
        };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(result.Readiness, Is.EqualTo(new[] { false }));
        Assert.That(result.Messages.Any(message => message.Contains("timed out") &&
            message.Contains("let it finish before retrying")), Is.True);
    }

    [TestCase("not a Visual Studio version", 0)]
    [TestCase("18.0.12345", 1)]
    public async Task InvalidOrFailedVswhereOutputCannotProveAUsableToolchain(string output, int exitCode)
    {
        var host = new FakeSystemCheckHost
        {
            MsvcPresent = false,
            VswhereOutput = output,
            VswhereExitCode = exitCode,
            MsvcExitCode = 1603
        };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(host.Processes.Any(p => p.FileName == "winget"), Is.True);
    }

    [Test]
    public async Task VswhereStartFailureFallsBackToAcquisition()
    {
        var host = new FakeSystemCheckHost
        {
            MsvcPresent = false,
            VswhereStartError = new Win32Exception("vswhere unavailable"),
            InstallMsvcFiles = true
        };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.True);
        Assert.That(host.Processes.Any(p => p.FileName == "winget"), Is.True);
    }

    [Test]
    public async Task VswhereFindsMultipleToolsetVersionsInACustomInstallation()
    {
        var first = @"D:\Developer Tools\VC\Tools\MSVC\14.49\bin\Hostx64\x64\cl.exe";
        var second = @"D:\Developer Tools\VC\Tools\MSVC\14.50\bin\Hostx64\x64\cl.exe";
        var host = new FakeSystemCheckHost { MsvcPresent = false, VswhereOutput = first + "\r\n" + second };
        host.Files[first] = [];
        host.Files[second] = [];
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.True);
        Assert.That(host.Processes.Any(process => process.FileName == "winget"), Is.False);
        var query = host.Processes.Single();
        Assert.That(query.FileName, Is.EqualTo(@"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe"));
        Assert.That(query.Arguments, Does.Contain("-requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64"));
        Assert.That(query.Arguments, Does.Contain("-find VC\\Tools\\MSVC\\**\\bin\\Hostx64\\x64\\cl.exe"));
    }

    [Test]
    public async Task VswhereOutputTimeoutFallsBackWithoutReportingASuccessfulProbe()
    {
        var host = new FakeSystemCheckHost
        {
            MsvcPresent = false,
            VswhereReadError = new TimeoutException("vswhere output timed out"),
            MsvcExitCode = 1603
        };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(host.Processes.Any(p => p.FileName == "winget"), Is.True);
    }

    [Test]
    public async Task EmptyMsvcDirectoryCannotProveACompilerIsInstalled()
    {
        var host = new FakeSystemCheckHost { VswherePresent = false, MsvcPresent = false, MsvcExitCode = 1603 };
        host.Directories.Add(@"C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC");
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(host.Processes.Any(p => p.FileName == "winget"), Is.True);
    }

    [TestCase(1603)]
    public async Task FailedBuildToolsInstallBlocksProceedingAndReportsTheExitCode(int exitCode)
    {
        var host = new FakeSystemCheckHost { MsvcPresent = false, MsvcExitCode = exitCode };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(result.Readiness, Is.EqualTo(new[] { false }));
        Assert.That(result.Messages.Any(m => m.Contains(exitCode.ToString())), Is.True);
    }

    [Test]
    public async Task MissingWingetProducesARecoverablePrerequisiteFailure()
    {
        var host = new FakeSystemCheckHost { MsvcPresent = false, MsvcStartError = new Win32Exception("winget not found") };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(result.Messages.Any(m => m.Contains("winget not found")), Is.True);
    }

    [Test]
    public async Task NullBuildToolsProcessDoesNotCountAsInstallation()
    {
        var host = new FakeSystemCheckHost { MsvcPresent = false, NullMsvcProcess = true };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(result.Readiness, Is.EqualTo(new[] { false }));
    }

    [Test]
    public async Task BuildToolsWaitFailureBlocksProceeding()
    {
        var host = new FakeSystemCheckHost { MsvcPresent = false, MsvcWaitError = new TimeoutException("Build tools timed out") };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(result.Messages.Any(m => m.Contains("timed out")), Is.True);
    }

    [Test]
    public async Task BuildToolsSuccessWithoutAUsableToolchainIsRejected()
    {
        var host = new FakeSystemCheckHost { MsvcPresent = false };
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.False);
        Assert.That(result.Readiness, Is.EqualTo(new[] { false }));
    }

    [Test]
    public async Task RebootRequiredSurvivesTheSystemCheckInSharedInstallationState()
    {
        var host = new FakeSystemCheckHost { MsvcPresent = false, MsvcExitCode = 3010, InstallMsvcFiles = true };
        var config = new InstallationConfig();
        var result = await RunAsync(host, config);

        Assert.That(result.Passed, Is.True);
        Assert.That(config.NeedsReboot, Is.True);
        Assert.That(result.Messages.Any(m => m.Contains("restart", StringComparison.OrdinalIgnoreCase)), Is.True);
    }

    [Test]
    public async Task SuccessfulRetryClearsThePreviousFailureAndEnablesProceeding()
    {
        var host = new FakeSystemCheckHost { Administrator = false };
        var step = new SystemCheckStep(host);
        var readiness = new List<bool>();
        step.ProceedChanged += readiness.Add;
        var config = new InstallationConfig();

        Assert.That(await step.PerformChecksAsync(config), Is.False);
        host.Administrator = true;
        Assert.That(await step.PerformChecksAsync(config), Is.True);
        Assert.That(readiness, Is.EqualTo(new[] { false, true }));
    }

    private static bool IsSevenZipInstaller(ProcessStartInfo process) =>
        process.FileName.EndsWith("7z-install.exe", StringComparison.OrdinalIgnoreCase);

    private static Task<StepObservation> RunAsync(FakeSystemCheckHost host, InstallationConfig? config = null) =>
        StepObservation.CaptureAsync(async (log, ready) =>
        {
            var step = new SystemCheckStep(host);
            step.MessageLogged += log;
            step.ProceedChanged += ready;
            return await step.PerformChecksAsync(config ?? new InstallationConfig());
        });
}
