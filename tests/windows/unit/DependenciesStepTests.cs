using System.ComponentModel;
using NUnit.Framework;
using Weave.Shared.Models;
using Weave.UI.Pages;

namespace Weave.Windows.UnitTests;

[TestFixture]
public sealed class DependenciesStepTests
{
    private static readonly string[] PackageIds =
    [
        "Kitware.CMake", "Git.Git", "Ninja-build.Ninja", "LLVM.LLVM", "Gyan.FFmpeg.Shared", "KhronosGroup.VulkanSDK"
    ];
    private static readonly string[] PathPackageIds = ["Kitware.CMake", "Git.Git", "Ninja-build.Ninja"];
    [Test]
    public async Task InstalledToolsArePreservedWhileFFmpegStillChecksForTheLatestRelease()
    {
        var host = new FakeDependenciesHost();
        var result = await RunAsync(host);
        Assert.That(result.Passed, Is.True);
        Assert.That(result.Readiness.Last(), Is.True);
        Assert.That(Installers(host), Has.Count.EqualTo(1));
        Assert.That(Arguments(Installers(host).Single()), Does.Contain("--id Gyan.FFmpeg.Shared"));
        Assert.That(host.Moves, Is.Empty);
    }

    [TestCaseSource(nameof(PackageIds))]
    public async Task MissingToolsAreInstalledWithExactIdsAndAgreementFlags(string id)
    {
        var host = new FakeDependenciesHost();
        host.RemovePackage(id);
        var result = await RunAsync(host);
        Assert.That(result.Passed, Is.True);
        var installer = Installers(host).Single(process => Arguments(process).Contains("--id " + id));
        Assert.That(installer.FileName, Is.EqualTo("winget"));
        Assert.That(installer.UseShellExecute, Is.False);
        Assert.That(installer.RedirectStandardOutput, Is.True);
        Assert.That(installer.RedirectStandardError, Is.True);
        Assert.That(Arguments(installer), Does.Contain("--exact"));
        Assert.That(Arguments(installer), Does.Contain("--accept-package-agreements"));
        Assert.That(Arguments(installer), Does.Contain("--accept-source-agreements"));
        Assert.That(host.Waits.Single(wait => wait.Package == id && wait.Installer).Cancellable, Is.True);
    }

    [TestCase("winget")]
    [TestCase("Microsoft.Winget.Source")]
    public async Task SourceDiscoveryUsesTheRegisteredSourceNameRatherThanASubstring(string source)
    {
        var host = new FakeDependenciesHost { SourceOutput = $"Name Argument Explicit\r\n----------------------------------\r\n{source} https://cdn.winget.microsoft.com/cache false\r\n" };
        await RunAsync(host);
        Assert.That(Arguments(Installers(host).Single()), Does.Contain("--source " + source));
    }

    [TestCase("Error: winget could not list sources", 0)]
    [TestCase("Name Argument\r\nmy-winget-mirror https://packages.example\r\n", 0)]
    [TestCase("Name Argument\r\nwinget https://cdn.winget.microsoft.com/cache\r\n", 1)]
    public async Task FailedOrUnrelatedSourceOutputIsNotPassedAsARegisteredSource(string output, int code)
    {
        var host = new FakeDependenciesHost { SourceOutput = output, SourceExitCode = code };
        await RunAsync(host);
        Assert.That(Arguments(Installers(host).Single()), Does.Not.Contain("--source"));
    }

    [Test]
    public async Task SourceProbeFailureFallsBackWithoutExecutingAShellCommand()
    {
        var host = new FakeDependenciesHost { SourceStartError = new Win32Exception("Source query unavailable") };
        var result = await RunAsync(host);
        Assert.That(result.Passed, Is.True);
        Assert.That(Arguments(Installers(host).Single()), Does.Not.Contain("--source"));
        Assert.That(host.Processes.All(process => !process.UseShellExecute && process.FileName is not "cmd.exe" and not "powershell.exe"), Is.True);
    }

    [Test]
    public async Task MissingWingetBlocksAutomaticProceedingWithAClearError()
    {
        var host = new FakeDependenciesHost
        {
            SourceStartError = new Win32Exception("winget not found"),
            InstallStartError = new Win32Exception("winget not found")
        };
        host.RemovePackage("Gyan.FFmpeg.Shared");
        var result = await RunAsync(host);
        result.AssertBlocked();
        Assert.That(result.Messages.Any(message => message.Contains("winget not found")), Is.True);
    }

    [Test]
    public async Task NoUpgradeIsSuccessfulOnlyWhenVerificationSucceeds()
    {
        var host = new FakeDependenciesHost();
        host.ExitCodes["Gyan.FFmpeg.Shared"] = FakeDependenciesHost.NoUpgrade;
        var result = await RunAsync(host);
        Assert.That(result.Passed, Is.True);
        Assert.That(result.Messages.Any(message => message == "[FFmpeg] OK"), Is.True);
    }

    [Test]
    public async Task EnvironmentReadFailureCannotSilentlySkipTheEntireStep()
    {
        var result = await RunAsync(new FakeDependenciesHost { EnvironmentError = new UnauthorizedAccessException("PATH access denied") });
        result.AssertBlocked();
        Assert.That(result.Messages.Any(message => message.Contains("PATH access denied")), Is.True);
    }

    [Test]
    public async Task ToolProbeFailureCannotEnableNext()
    {
        var result = await RunAsync(new FakeDependenciesHost { ProbeError = new IOException("Tool path inaccessible") });
        result.AssertBlocked();
        Assert.That(result.Messages.Any(message => message.Contains("Tool path inaccessible")), Is.True);
    }

    [TestCaseSource(nameof(PathPackageIds))]
    public async Task RelativePathEntryCannotProveARequiredToolIsInstalled(string id)
    {
        var host = new FakeDependenciesHost { MachinePath = "relative", UserPath = null };
        host.Files.Add(Path.Combine("relative", Path.GetFileName(PackageExecutable(id))));
        await RunAsync(host);
        AssertInstallerRequested(host, id);
    }

    [TestCaseSource(nameof(PackageIds))]
    public async Task DirectoryNamedLikeARequiredExecutableIsNotAnInstalledTool(string id)
    {
        var host = new FakeDependenciesHost();
        host.Files.Remove(PackageExecutable(id));
        host.Directories.Add(PackageExecutable(id));
        await RunAsync(host);
        AssertInstallerRequested(host, id);
    }

    [Test]
    public async Task EmptyVulkanSdkDirectoryDoesNotProveAnInstalledSdk()
    {
        var host = new FakeDependenciesHost();
        host.RemovePackage("KhronosGroup.VulkanSDK");
        host.Directories.Add(@"C:\VulkanSDK");
        await RunAsync(host);
        Assert.That(Installers(host).Any(process => Arguments(process).Contains("--id KhronosGroup.VulkanSDK")), Is.True);
    }

    [Test]
    public async Task FFmpegLocationIsQuotedAndForceIsLimitedToMissingInstallations()
    {
        var healthy = new FakeDependenciesHost();
        await RunAsync(healthy);
        var healthyArguments = Arguments(Installers(healthy).Single());
        Assert.That(healthyArguments, Does.Contain("--location \"C:\\Program Files\\FFmpeg_Shared\""));
        Assert.That(healthyArguments, Does.Not.Contain("--force"));
        var missing = new FakeDependenciesHost();
        missing.RemovePackage("Gyan.FFmpeg.Shared");
        await RunAsync(missing);
        Assert.That(Arguments(Installers(missing).Single()), Does.Contain("--force"));
    }

    [Test]
    public async Task VersionedFFmpegWrapperIsFlattenedWithoutMovingOlderVersions()
    {
        var host = new FakeDependenciesHost { AutoInstall = false };
        var old = Path.Combine(DependenciesStep.FFmpegInstallRoot, "9.0.2");
        var wrapper = host.AddFFmpeg("ffmpeg-9.1.0-full_build-shared");
        host.AddFFmpeg("ffmpeg-not-a-version-full_build-shared");
        var result = await RunAsync(host);
        Assert.That(result.Passed, Is.True);
        Assert.That(host.Moves, Is.EqualTo(new[] { (wrapper, Path.Combine(DependenciesStep.FFmpegInstallRoot, "9.1.0")) }));
        Assert.That(host.Files.Contains(Path.Combine(old, "bin", "ffmpeg.exe")), Is.True);
        Assert.That(DependenciesStep.FindFFmpegRoot(host), Is.EqualTo(Path.Combine(DependenciesStep.FFmpegInstallRoot, "9.1.0")));
    }

    [Test]
    public async Task ExistingNormalizedVersionIsNotOverwrittenByAWrapper()
    {
        var host = new FakeDependenciesHost();
        var wrapper = host.AddFFmpeg("ffmpeg-9.0.2-full_build-shared");
        await RunAsync(host);
        Assert.That(host.Moves, Is.Empty);
        Assert.That(host.Directories.Contains(wrapper), Is.True);
    }

    [Test]
    public async Task FFmpegVersionSelectionUsesNumericOrderingAndIgnoresIncompleteVersions()
    {
        var host = new FakeDependenciesHost();
        host.AddFFmpeg("9.9.0");
        var expected = host.AddFFmpeg("9.10.0");
        host.AddFFmpeg("99.0.0", executable: false);
        host.AddFFmpeg("98.0.0", shared: false);
        host.AddFFmpeg("not-a-version");
        Assert.That(DependenciesStep.FindFFmpegRoot(host), Is.EqualTo(expected));
    }

    [Test]
    public async Task FFmpegExecutableAloneCannotProveASharedDevelopmentPackage()
    {
        var host = new FakeDependenciesHost();
        host.RemovePackage("Gyan.FFmpeg.Shared");
        host.AddFFmpeg("9.0.2", shared: false);
        Assert.That(DependenciesStep.FindFFmpegRoot(host), Is.Null);
    }

    [Test]
    public async Task FFmpegRenameFailureIsReportedAndCannotEnableNext()
    {
        var host = new FakeDependenciesHost { MoveError = new IOException("FFmpeg directory locked") };
        host.RemovePackage("Gyan.FFmpeg.Shared");
        host.AddFFmpeg("9.0.2", shared: false);
        host.AddFFmpeg("ffmpeg-9.1.0-full_build-shared");
        var result = await RunAsync(host);
        result.AssertBlocked();
        Assert.That(result.Messages.Any(message => message.Contains("FFmpeg directory locked")), Is.True);
        Assert.That(host.Files.Contains(Path.Combine(DependenciesStep.FFmpegInstallRoot, "9.0.2", "bin", "ffmpeg.exe")), Is.True);
    }

    [Test]
    public async Task ExplicitDependencySkipDoesNotStartInstallers()
    {
        var host = new FakeDependenciesHost();
        var config = new InstallationConfig();
        var navigations = 0;
        new DependenciesStep(host).SkipDependencies(config, () => navigations++);
        Assert.That(config.SkipDependencies, Is.True);
        Assert.That(navigations, Is.EqualTo(1));
        var result = await RunAsync(host, config);
        Assert.That(result.Passed, Is.True);
        Assert.That(host.Processes, Is.Empty);
        Assert.That(host.Moves, Is.Empty);
        Assert.That(result.Messages.Any(message => message.Contains("skip", StringComparison.OrdinalIgnoreCase)), Is.True);
    }

    [Test]
    public async Task SkipCannotNavigateIntoEnvironmentSetupWhileAnInstallerIsStillRunning()
    {
        var releaseInstaller = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
        var host = new FakeDependenciesHost { InstallWaitBarrier = releaseInstaller.Task };
        host.RemovePackage("Kitware.CMake");
        var step = new DependenciesStep(host);
        var config = new InstallationConfig();
        var navigations = 0;
        var installation = step.InstallDependenciesAsync(config);
        try
        {
            await host.InstallerEntered.Task.WaitAsync(TimeSpan.FromSeconds(5));
            step.SkipDependencies(config, () => navigations++);
            Assert.That(navigations, Is.Zero);
        }
        finally
        {
            releaseInstaller.TrySetResult(true);
            await installation.WaitAsync(TimeSpan.FromSeconds(5));
        }
    }

    [TestCaseSource(nameof(PackageIds))]
    public async Task ToolMustExecuteSuccessfullyAndReturnItsOwnVersionBanner(string id)
    {
        var host = new FakeDependenciesHost { AutoInstall = false };
        host.ToolExitCodes[id] = 1;
        (await RunAsync(host)).AssertBlocked();
        host.ToolExitCodes[id] = 0;
        host.ToolOutputs[id] = "Unrelated output from an executable with the same filename";
        (await RunAsync(host)).AssertBlocked();
    }

    [TestCase("null")]
    [TestCase("exit")]
    [TestCase("start")]
    [TestCase("timeout")]
    [TestCase("output")]
    [TestCase("missing files")]
    [TestCase("no upgrade")]
    public async Task SharedInstallerFailuresBlockNextAndReportDiagnostics(string failure)
    {
        const string id = "LLVM.LLVM";
        var host = new FakeDependenciesHost { FaultPackage = id };
        host.RemovePackage(id);
        string? diagnostic = null;
        switch (failure)
        {
            case "null": host.NullInstallerProcess = true; break;
            case "exit":
                host.ExitCodes[id] = 1603;
                diagnostic = host.InstallError = "Package access denied by installer";
                break;
            case "start": host.InstallStartError = new Win32Exception(diagnostic = "Installer cannot start"); break;
            case "timeout": host.InstallWaitError = new TimeoutException(diagnostic = "Installation timed out"); break;
            case "output": host.OutputError = new IOException(diagnostic = "Output pipe failed"); break;
            case "missing files": host.AutoInstall = false; break;
            case "no upgrade": host.AutoInstall = false; host.ExitCodes[id] = FakeDependenciesHost.NoUpgrade; break;
        }
        var result = await RunAsync(host);
        AssertInstallerRequested(host, id);
        result.AssertBlocked();
        if (diagnostic != null) Assert.That(result.Messages.Any(message => message.Contains(diagnostic)), Is.True);
        if (failure == "exit") Assert.That(result.Messages.Any(message => message.Contains("1603")), Is.True);
    }
    [TestCase("quoted")]
    [TestCase("user")]
    [TestCase("expanded")]
    [TestCase("process")]
    public async Task SupportedPathSourcesProvideToolsWithoutReinstallation(string source)
    {
        var host = new FakeDependenciesHost { MachinePath = null, UserPath = null };
        switch (source)
        {
            case "quoted": host.MachinePath = "\"" + FakeDependenciesHost.ToolDirectory + "\""; break;
            case "user": host.UserPath = FakeDependenciesHost.ToolDirectory; break;
            case "expanded":
                host.MachinePath = "%TOOLS_HOME%";
                host.EnvironmentVariables[("TOOLS_HOME", EnvironmentVariableTarget.Machine)] = FakeDependenciesHost.ToolDirectory;
                break;
            case "process": host.ProcessPath = FakeDependenciesHost.ToolDirectory; break;
        }
        Assert.That((await RunAsync(host)).Passed, Is.True);
        Assert.That(Installers(host), Has.Count.EqualTo(1));
        Assert.That(Arguments(Installers(host).Single()), Does.Contain("--id Gyan.FFmpeg.Shared"));
    }
    private static void AssertInstallerRequested(FakeDependenciesHost host, string id) =>
        Assert.That(Installers(host).Any(process => Arguments(process).Contains("--id " + id)), Is.True);

    [Test]
    public async Task WorkingDependenciesRemainUsableWhenWinGetIsUnavailable()
    {
        var host = new FakeDependenciesHost
        {
            SourceStartError = new Win32Exception("winget unavailable"),
            InstallStartError = new Win32Exception("winget unavailable")
        };
        var result = await RunAsync(host);
        Assert.That(result.Passed, Is.True);
        Assert.That(result.Readiness.Last(), Is.True);
        Assert.That(result.Messages.Any(message => message.Contains("winget unavailable")), Is.True);
    }

    [Test]
    public async Task FailedFFmpegRefreshPreservesTheWorkingVersionAndAllowsProceeding()
    {
        var host = new FakeDependenciesHost();
        host.ExitCodes["Gyan.FFmpeg.Shared"] = 1603;
        var result = await RunAsync(host);
        Assert.That(result.Passed, Is.True);
        Assert.That(result.Messages.Any(message => message.Contains("1603")), Is.True);
        Assert.That(host.Files.Contains(@"C:\Program Files\FFmpeg_Shared\9.0.2\bin\ffmpeg.exe"), Is.True);
    }

    [Test]
    public async Task FailedFFmpegRenameDoesNotBlockAnExistingUsableSharedVersion()
    {
        var host = new FakeDependenciesHost { MoveError = new IOException("New FFmpeg directory locked") };
        host.AddFFmpeg("ffmpeg-9.1.0-full_build-shared");
        var result = await RunAsync(host);
        Assert.That(result.Passed, Is.True);
        Assert.That(result.Messages.Any(message => message.Contains("New FFmpeg directory locked")), Is.True);
        Assert.That(host.Files.Contains(@"C:\Program Files\FFmpeg_Shared\9.0.2\bin\ffmpeg.exe"), Is.True);
    }

    [Test]
    public async Task WorkingLegacyFFmpegRootIsPreservedWhenRefreshIsUnavailable()
    {
        var host = new FakeDependenciesHost { InstallStartError = new Win32Exception("Offline") };
        host.RemovePackage("Gyan.FFmpeg.Shared");
        var legacy = host.AddFFmpeg("legacy-shared");
        host.EnvironmentVariables[("FFMPEG_ROOT", EnvironmentVariableTarget.User)] = legacy;
        var config = new InstallationConfig();
        var result = await RunAsync(host, config);
        Assert.That(result.Passed, Is.True);
        Assert.That(config.DependencyExecutablePaths["Gyan.FFmpeg.Shared"], Is.EqualTo(Path.Combine(legacy, "bin", "ffmpeg.exe")));
    }

    [Test]
    public async Task BrokenNewerFFmpegExecutableCannotDisplaceTheWorkingSelectedVersion()
    {
        var host = new FakeDependenciesHost();
        var newer = host.AddFFmpeg("99.0.0");
        host.ToolExitCodesByPath[Path.Combine(newer, "bin", "ffmpeg.exe")] = 1;
        var config = new InstallationConfig();
        var result = await RunAsync(host, config);
        Assert.That(result.Passed, Is.True);
        Assert.That(config.DependencyExecutablePaths["Gyan.FFmpeg.Shared"], Is.EqualTo(@"C:\Program Files\FFmpeg_Shared\9.0.2\bin\ffmpeg.exe"));
    }

    [TestCase("Kitware.CMake", "cmake version 4.3.0-rc1\r\nCMake suite maintained and supported by Kitware")]
    [TestCase("Git.Git", "git version 2.39.5.windows.1")]
    [TestCase("Ninja-build.Ninja", "1.10.2.git.kitware.jobserver-1")]
    [TestCase("LLVM.LLVM", "Vendor LLVM clang version 20.1.8\r\nTarget: x86_64-pc-windows-msvc")]
    [TestCase("Gyan.FFmpeg.Shared", "ffmpeg version N-123456-gabcdef Copyright")]
    [TestCase("KhronosGroup.VulkanSDK", "shaderc 2023.2\r\nspirv-tools v2023.2\r\nglslang 11.13.0")]
    public async Task WorkingVendorAndOlderVersionBannersRemainAccepted(string id, string banner)
    {
        var host = new FakeDependenciesHost { AutoInstall = false };
        host.ToolOutputs[id] = banner;
        Assert.That((await RunAsync(host)).Passed, Is.True);
    }

    [Test]
    public async Task EnvironmentSetupKeepsTheCustomRootsVerifiedByDependencySetup()
    {
        var host = new FakeDependenciesHost();
        host.RemovePackage("LLVM.LLVM");
        host.RemovePackage("KhronosGroup.VulkanSDK");
        host.MachinePath += @";D:\LLVM Custom\bin";
        host.Files.Add(@"D:\LLVM Custom\bin\clang.exe");
        host.EnvironmentVariables[("VULKAN_SDK", EnvironmentVariableTarget.User)] = @"D:\SDK Custom\1.3.0";
        host.Files.Add(@"D:\SDK Custom\1.3.0\Bin\glslc.exe");
        var config = new InstallationConfig();
        Assert.That((await RunAsync(host, config)).Passed, Is.True);
        Assert.That(Installers(host), Has.Count.EqualTo(1));
        Assert.That(EnvironmentSetupStep.SelectedDependencyRoot(config, "LLVM.LLVM"), Is.EqualTo(@"D:\LLVM Custom"));
        Assert.That(EnvironmentSetupStep.SelectedDependencyRoot(config, "KhronosGroup.VulkanSDK"), Is.EqualTo(@"D:\SDK Custom\1.3.0"));
    }

    [Test]
    public async Task TimedOutLiveInstallerPreventsMoreInstallersAndDefersSkipUntilItExits()
    {
        var release = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
        var navigation = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
        var host = new FakeDependenciesHost { InstallWaitBarrier = release.Task, InstallTimeoutOverride = TimeSpan.FromMilliseconds(20) };
        host.RemovePackage("Kitware.CMake");
        var step = new DependenciesStep(host);
        var config = new InstallationConfig();
        try
        {
            Assert.That(await step.InstallDependenciesAsync(config).WaitAsync(TimeSpan.FromSeconds(5)), Is.False);
            step.SkipDependencies(config, () => navigation.TrySetResult(true));
            Assert.That(navigation.Task.IsCompleted, Is.False);
            Assert.That(Installers(host), Has.Count.EqualTo(1));
            release.TrySetResult(true);
            Assert.That(await navigation.Task.WaitAsync(TimeSpan.FromSeconds(5)), Is.True);
        }
        finally { release.TrySetResult(true); }
    }

    [Test]
    public async Task CMakeTooOldForGeneratedProjectsCannotCountAsAUsableDependency()
    {
        var host = new FakeDependenciesHost { AutoInstall = false };
        host.ToolOutputs["Kitware.CMake"] = "cmake version 3.25.0";
        (await RunAsync(host)).AssertBlocked();
    }

    [Test]
    public async Task SuccessfulRetryAfterAStartFailureRestoresReadiness()
    {
        var host = new FakeDependenciesHost { FaultPackage = "LLVM.LLVM", InstallStartError = new Win32Exception("Installer unavailable") };
        host.RemovePackage("LLVM.LLVM");
        var step = new DependenciesStep(host);
        var config = new InstallationConfig();
        var readiness = new List<bool>();
        step.ProceedChanged += readiness.Add;
        Assert.That(await step.InstallDependenciesAsync(config), Is.False);
        Assert.That(readiness.Last(), Is.False);
        host.InstallStartError = null;
        Assert.That(await step.InstallDependenciesAsync(config), Is.True);
        Assert.That(readiness.Last(), Is.True);
        host.RemovePackage("LLVM.LLVM");
        host.InstallStartError = new Win32Exception("Installer unavailable again");
        Assert.That(await step.InstallDependenciesAsync(config), Is.False);
        Assert.That(readiness.Last(), Is.False);
    }

    private static string PackageExecutable(string id) => id switch
    {
        "Kitware.CMake" => @"D:\Installed Tools\cmake.exe",
        "Git.Git" => @"D:\Installed Tools\git.exe",
        "Ninja-build.Ninja" => @"D:\Installed Tools\ninja.exe",
        "LLVM.LLVM" => @"C:\Program Files\LLVM\bin\clang.exe",
        "Gyan.FFmpeg.Shared" => @"C:\Program Files\FFmpeg_Shared\9.0.2\bin\ffmpeg.exe",
        "KhronosGroup.VulkanSDK" => @"C:\VulkanSDK\1.4.321.1\Bin\glslc.exe",
        _ => throw new ArgumentException(id)
    };

    private static List<System.Diagnostics.ProcessStartInfo> Installers(FakeDependenciesHost host) =>
        host.Processes.Where(process => Arguments(process).StartsWith("install ")).ToList();
    private static string Arguments(System.Diagnostics.ProcessStartInfo process) => process.ArgumentList.Count == 0
        ? process.Arguments : string.Join(" ", process.ArgumentList.Select(argument => argument.Contains(' ') ? "\"" + argument + "\"" : argument));
    private static Task<StepObservation> RunAsync(FakeDependenciesHost host, InstallationConfig? config = null) =>
        StepObservation.CaptureAsync(async (log, ready) =>
        {
            var step = new DependenciesStep(host);
            step.MessageLogged += log;
            step.ProceedChanged += ready;
            return await step.InstallDependenciesAsync(config ?? new InstallationConfig());
        });
}
