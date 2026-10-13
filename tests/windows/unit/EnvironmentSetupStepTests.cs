using NUnit.Framework;
using Weave.UI.Pages;
using Weave.Utils;

namespace Weave.Windows.UnitTests;

[TestFixture]
public class EnvironmentSetupStepTests
{
    [TestCase("")]
    [TestCase("relative")]
    [TestCase("D:relative")]
    [TestCase("D:\\Maya;Flux")]
    [TestCase("D:\\Maya\nFlux")]
    public async Task UnsafeMayaFluxRootDoesNotChangeEnvironment(string root)
    {
        var (host, config) = FakeEnvironmentSetupHost.Healthy();
        config.MayaFluxRoot = root;
        await new EnvironmentSetupStep(host).SetupEnvironmentAsync(config);
        Assert.That(host.Writes, Is.Empty);
    }

    [TestCase("PATH")]
    public void EmptyVariableGetsOneEntryWithoutEmptySeparators(string variable)
    {
        var host = new FakeEnvironmentSetupHost();
        if (variable == "PATH") ProcessRunner.AddToPath(@"D:\New Dir", host.CreateLogger(), host);
        else ProcessRunner.AppendToEnvironmentVariable(variable, @"D:\New Dir", host.CreateLogger(), host);
        Assert.That(host.Variables[variable], Is.EqualTo(@"D:\New Dir"));
    }

    [Test]
    public async Task HealthySetupPreservesExistingEntriesAndUsesVerifiedCustomRoots()
    {
        var (host, config) = FakeEnvironmentSetupHost.Healthy();
        host.Variables["CPATH"] = @"E:\Existing Include";
        host.Variables["LIBRARY_PATH"] = @"E:\Existing Lib";
        var step = new EnvironmentSetupStep(host);
        await step.SetupEnvironmentAsync(config);
        var previous = new Dictionary<string, string>(host.Variables);
        await step.SetupEnvironmentAsync(config);
        Assert.That(host.Variables, Is.EquivalentTo(previous));
        Assert.Multiple(() =>
        {
            Assert.That(host.Variables["MAYAFLUX_ROOT"], Is.EqualTo(@"D:\Maya Flux"));
            Assert.That(host.Variables["LLVM_ROOT"], Is.EqualTo(@"D:\LLVM"));
            Assert.That(host.Variables["VULKAN_SDK"], Is.EqualTo(@"D:\SDK"));
            Assert.That(host.Variables["FFMPEG_ROOT"], Is.EqualTo(@"D:\FFmpeg"));
            Assert.That(host.Variables["PATH"].Split(';'), Does.Contain(@"C:\Windows\System32"));
            Assert.That(host.Variables["CPATH"].Split(';'), Does.Contain(@"E:\Existing Include"));
            Assert.That(host.Variables["LIBRARY_PATH"].Split(';'), Does.Contain(@"E:\Existing Lib"));
        });
    }

    [TestCase("MAYAFLUX_ROOT")]
    [TestCase("PATH")]
    [TestCase("CPATH")]
    [TestCase("LIBRARY_PATH")]
    [TestCase("VULKAN_SDK")]
    [TestCase("VK_SDK_PATH")]
    [TestCase("LLVM_ROOT")]
    [TestCase("LLVM_DIR")]
    [TestCase("Clang_DIR")]
    [TestCase("FFMPEG_ROOT")]
    public async Task FailedWriteBlocksNextWithoutFalseSuccessMessages(string variable)
    {
        var (host, config) = FakeEnvironmentSetupHost.Healthy();
        host.FailedWrite = variable;
        var result = await RunAsync(host, config);
        result.AssertBlocked();
        Assert.That(result.Messages.Any(message => message.Contains("[ERROR]") && message.Contains(variable)), Is.True);
        Assert.That(result.Messages, Has.None.Contains("=== Environment Setup Complete ==="));
        Assert.That(result.Messages.Where(message => message.Contains("[OK]") &&
            (message.Contains(" " + variable + "=") || message.Contains("to " + variable + ":"))), Is.Empty);
    }

    [TestCase("PATH")]
    [TestCase("CPATH")]
    [TestCase("LIBRARY_PATH")]
    public async Task FailedReadBlocksNext(string variable)
    {
        var (host, config) = FakeEnvironmentSetupHost.Healthy();
        host.FailedRead = variable;
        (await RunAsync(host, config)).AssertBlocked();
    }

    [TestCase(@"D:\Tools\binary", @"D:\Tools\bin")]
    [TestCase(@"D:\Tools\bin-old", @"D:\Tools\bin")]
    [TestCase(@"D:\Prefix\D:\Tools\bin", @"D:\Tools\bin")]
    public void PathSubstringDoesNotSuppressRequiredEntry(string existing, string added)
    {
        var host = new FakeEnvironmentSetupHost();
        host.Variables["PATH"] = existing;
        Assert.That(ProcessRunner.AddToPath(added, host.CreateLogger(), host), Is.True);
        Assert.That(host.Variables["PATH"].Split(';'), Does.Contain(added));
        Assert.That(host.Variables["PATH"].Split(';'), Does.Contain(existing));
    }

    [TestCase("PATH")]
    [TestCase("CPATH")]
    [TestCase("LIBRARY_PATH")]
    public void ExistingPathVariantsArePreservedWithoutDuplication(string name)
    {
        var host = new FakeEnvironmentSetupHost();
        host.Variables["TOOLS_ROOT"] = @"d:\tools";
        host.Variables[name] = @" ""%TOOLS_ROOT%\other\..\BIN\"" ";
        var existing = host.Variables[name];
        if (name == "PATH") ProcessRunner.AddToPath(@"D:\Tools\bin", host.CreateLogger(), host);
        else ProcessRunner.AppendToEnvironmentVariable(name, @"D:\Tools\bin", host.CreateLogger(), host);
        Assert.That(host.Variables[name], Is.EqualTo(existing));
    }

    [TestCase("")]
    [TestCase("relative\\bin")]
    [TestCase("D:relative")]
    [TestCase("D:\\Bad;Injected")]
    [TestCase("D:\\Bad\nInjected")]
    public void InvalidPathEntryIsRejectedWithoutMutation(string path)
    {
        var host = new FakeEnvironmentSetupHost();
        host.Variables["PATH"] = @"C:\Windows";
        Assert.That(ProcessRunner.AddToPath(path, host.CreateLogger(), host), Is.False);
        Assert.That(host.Variables["PATH"], Is.EqualTo(@"C:\Windows"));
    }

    [TestCase("CPATH", "relative")]
    [TestCase("CPATH", "D:\\Bad;Injected")]
    [TestCase("LIBRARY_PATH", "relative")]
    [TestCase("LIBRARY_PATH", "D:\\Bad\nInjected")]
    public void InvalidIncludeOrLibraryEntryDoesNotChangeExistingValue(string name, string path)
    {
        var host = new FakeEnvironmentSetupHost();
        host.Variables[name] = @"E:\Existing";
        Assert.That(ProcessRunner.AppendToEnvironmentVariable(name, path, host.CreateLogger(), host), Is.False);
        Assert.That(host.Variables[name], Is.EqualTo(@"E:\Existing"));
    }

    [TestCase("LLVM.LLVM")]
    [TestCase("KhronosGroup.VulkanSDK")]
    [TestCase("Gyan.FFmpeg.Shared")]
    public async Task UnsafeSelectedRootIsNotWritten(string package)
    {
        var (host, config) = FakeEnvironmentSetupHost.Healthy();
        config.DependencyExecutablePaths[package] = @"D:\Unsafe;Injected\bin\tool.exe";
        host.Directories.Add(@"D:\Unsafe;Injected");
        await new EnvironmentSetupStep(host).SetupEnvironmentAsync(config);
        Assert.That(host.Variables.Values, Has.None.Contains("Injected"));
    }

    [Test]
    public async Task ExplicitDependencySkipKeepsMissingOptionalToolsRecoverable()
    {
        var (host, config) = FakeEnvironmentSetupHost.Healthy();
        config.SkipDependencies = true;
        config.DependencyExecutablePaths.Remove("LLVM.LLVM");
        config.DependencyExecutablePaths.Remove("KhronosGroup.VulkanSDK");
        host.Variables["LLVM_ROOT"] = @"E:\Previous LLVM";
        host.Variables["VULKAN_SDK"] = @"E:\Previous SDK";
        var step = new EnvironmentSetupStep(host);
        bool? ready = null;
        step.ProceedChanged += value => ready = value;
        await step.SetupEnvironmentAsync(config);
        var previous = new Dictionary<string, string>(host.Variables);
        await step.SetupEnvironmentAsync(config);
        Assert.That(host.Variables, Is.EquivalentTo(previous));
        Assert.Multiple(() =>
        {
            Assert.That(ready, Is.True);
            Assert.That(host.Variables["LLVM_ROOT"], Is.EqualTo(@"E:\Previous LLVM"));
            Assert.That(host.Variables["VULKAN_SDK"], Is.EqualTo(@"E:\Previous SDK"));
        });
    }

    [TestCase(@"D:\Maya Flux\bin")]
    [TestCase(@"D:\Maya Flux\include")]
    [TestCase(@"D:\Maya Flux\lib")]
    [TestCase(@"D:\LLVM")]
    [TestCase(@"D:\SDK")]
    [TestCase(@"D:\FFmpeg")]
    public async Task MissingRequiredDirectoryBlocksNext(string path)
    {
        var (host, config) = FakeEnvironmentSetupHost.Healthy();
        host.Directories.Remove(path);
        (await RunAsync(host, config)).AssertBlocked();
    }

    [Test]
    public async Task VulkanFallbackChoosesNewestNumericVersion()
    {
        var (host, config) = FakeEnvironmentSetupHost.Healthy();
        config.DependencyExecutablePaths.Remove("KhronosGroup.VulkanSDK");
        host.Directories.Add(@"C:\VulkanSDK");
        host.VulkanVersions = [@"C:\VulkanSDK\1.9.0", @"C:\VulkanSDK\1.10.0"];
        foreach (var root in host.VulkanVersions)
            foreach (var path in new[] { root, Path.Combine(root, "Bin"), Path.Combine(root, "Include") }) host.Directories.Add(path);
        await new EnvironmentSetupStep(host).SetupEnvironmentAsync(config);
        Assert.That(host.Variables["VULKAN_SDK"], Is.EqualTo(@"C:\VulkanSDK\1.10.0"));
    }

    [Test]
    public async Task RetryAfterFailureSucceedsWithoutDuplicatingEntries()
    {
        var (host, config) = FakeEnvironmentSetupHost.Healthy();
        var step = new EnvironmentSetupStep(host);
        bool? ready = null;
        step.ProceedChanged += value => ready = value;
        host.FailedWrite = "FFMPEG_ROOT";
        await step.SetupEnvironmentAsync(config);
        host.FailedWrite = null;
        await step.SetupEnvironmentAsync(config);
        Assert.That(ready, Is.True);
        Assert.That(host.Variables["PATH"].Split(';').Count(p => p == @"D:\Maya Flux\bin"), Is.EqualTo(1));
        host.FailedWrite = "FFMPEG_ROOT";
        await step.SetupEnvironmentAsync(config);
        Assert.That(ready, Is.False);
    }
    private static Task<StepObservation> RunAsync(FakeEnvironmentSetupHost host, Weave.Shared.Models.InstallationConfig config) =>
        StepObservation.CaptureReadinessAsync(async (log, ready) =>
        {
            var step = new EnvironmentSetupStep(host);
            step.MessageLogged += log;
            step.ProceedChanged += ready;
            await step.SetupEnvironmentAsync(config);
        });
}
