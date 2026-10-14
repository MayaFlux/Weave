using Weave.UI.Pages;
using Weave.Utils;

namespace Weave.Windows.UnitTests;

internal sealed class FakeEnvironmentSetupHost : EnvironmentSetupHost
{
    internal readonly Dictionary<string, string> Variables = new(StringComparer.OrdinalIgnoreCase);
    internal readonly HashSet<string> Directories = new(StringComparer.OrdinalIgnoreCase);
    internal readonly List<string> Writes = [];
    internal string? FailedWrite;
    internal string? FailedRead;
    internal string[] VulkanVersions = [];
    internal WindowsCompilerPaths CompilerPaths = new(
        [@"D:\Microsoft Tools\include", @"D:\Windows SDK\Include\10.0.26100.0\ucrt", @"D:\Windows SDK\Include\10.0.26100.0\shared", @"D:\Windows SDK\Include\10.0.26100.0\um", @"D:\Windows SDK\Include\10.0.26100.0\winrt", @"D:\Windows SDK\Include\10.0.26100.0\cppwinrt"],
        [@"D:\Microsoft Tools\lib\x64", @"D:\Windows SDK\Lib\10.0.26100.0\ucrt\x64", @"D:\Windows SDK\Lib\10.0.26100.0\um\x64"]);
    internal override Task<WindowsCompilerPaths> GetMicrosoftCompilerPathsAsync() => Task.FromResult(CompilerPaths);
    internal override Logger CreateLogger() => new("NUL");
    internal override string? GetEnvironmentVariable(string name, EnvironmentVariableTarget target)
    {
        if (name.Equals(FailedRead, StringComparison.OrdinalIgnoreCase)) throw new UnauthorizedAccessException("Environment read denied");
        return Variables.GetValueOrDefault(name);
    }
    internal override void SetEnvironmentVariable(string name, string value, EnvironmentVariableTarget target)
    {
        if (target != EnvironmentVariableTarget.Machine) throw new InvalidOperationException("Unexpected environment scope");
        Writes.Add(name);
        if (name.Equals(FailedWrite, StringComparison.OrdinalIgnoreCase)) throw new UnauthorizedAccessException("Environment write denied");
        Variables[name] = value;
    }
    internal override bool DirectoryExists(string path) => Directories.Contains(path);
    internal override bool FileExists(string path) => Directories.Contains(Path.GetDirectoryName(path)!) && path.EndsWith(".exe", StringComparison.OrdinalIgnoreCase);
    internal override string[] GetDirectories(string path, string pattern = "*") => path == @"C:\VulkanSDK" ? VulkanVersions : [];
    internal override string[] GetFileSystemEntries(string path) => [];

    internal static (FakeEnvironmentSetupHost Host, Weave.Shared.Models.InstallationConfig Config) Healthy()
    {
        var host = new FakeEnvironmentSetupHost();
        var config = new Weave.Shared.Models.InstallationConfig { MayaFluxRoot = @"D:\Maya Flux" };
        foreach (var path in new[] { @"D:\Maya Flux", @"D:\Maya Flux\bin", @"D:\Maya Flux\include", @"D:\Maya Flux\lib", @"D:\LLVM", @"D:\LLVM\bin", @"D:\SDK", @"D:\SDK\Bin", @"D:\SDK\Include", @"D:\FFmpeg", @"D:\FFmpeg\bin" }) host.Directories.Add(path);
        config.DependencyExecutablePaths["LLVM.LLVM"] = @"D:\LLVM\bin\clang.exe";
        config.DependencyExecutablePaths["KhronosGroup.VulkanSDK"] = @"D:\SDK\Bin\glslc.exe";
        config.DependencyExecutablePaths["Gyan.FFmpeg.Shared"] = @"D:\FFmpeg\bin\ffmpeg.exe";
        host.Variables["PATH"] = @"C:\Windows\System32";
        foreach (var path in host.CompilerPaths.Includes.Concat(host.CompilerPaths.Libraries)) host.Directories.Add(path);
        host.Directories.Add(@"D:\FFmpeg\include");
        host.Directories.Add(@"D:\FFmpeg\lib");
        return (host, config);
    }
}
