using System.Diagnostics;
using System.IO.Enumeration;
using Weave.UI.Pages;

namespace Weave.Windows.UnitTests;

internal sealed class FakeDependenciesHost : DependenciesHost
{
    internal const int NoUpgrade = -1978335189;
    internal const string ToolDirectory = @"D:\Installed Tools";
    internal string? MachinePath { get; set; } = ToolDirectory;
    internal string? UserPath { get; set; } = @"D:\User Tools";
    internal string? ProcessPath { get; set; }
    internal Dictionary<(string Name, EnvironmentVariableTarget Target), string> EnvironmentVariables { get; } = [];
    internal string SourceOutput { get; set; } = "Name Argument Explicit\r\n------------------------------------\r\nwinget https://cdn.winget.microsoft.com/cache false\r\nmsstore https://storeedgefd.dsx.mp.microsoft.com/v9.0 true\r\n";
    internal int SourceExitCode { get; set; }
    internal bool AutoInstall { get; set; } = true;
    internal bool NullSourceProcess { get; set; }
    internal bool NullInstallerProcess { get; set; }
    internal string? FaultPackage { get; set; }
    internal TimeSpan? InstallTimeoutOverride { get; set; }
    internal override TimeSpan InstallTimeout => InstallTimeoutOverride ?? base.InstallTimeout;
    internal Exception? SourceStartError { get; set; }
    internal Exception? SourceReadError { get; set; }
    internal Exception? InstallStartError { get; set; }
    internal Exception? InstallWaitError { get; set; }
    internal Exception? OutputError { get; set; }
    internal Exception? EnvironmentError { get; set; }
    internal Exception? ProbeError { get; set; }
    internal Exception? MoveError { get; set; }
    internal Task? InstallWaitBarrier { get; set; }
    internal TaskCompletionSource<bool> InstallerEntered { get; } = new(TaskCreationOptions.RunContinuationsAsynchronously);
    internal string InstallOutput { get; set; } = "Package installation finished";
    internal string InstallError { get; set; } = "Installer diagnostic";
    internal List<ProcessStartInfo> Processes { get; } = [];
    internal List<(string Package, bool Cancellable, bool Installer)> Waits { get; } = [];
    internal List<(string Source, string Destination)> Moves { get; } = [];
    internal HashSet<string> Files { get; } = new(StringComparer.OrdinalIgnoreCase);
    internal HashSet<string> Directories { get; } = new(StringComparer.OrdinalIgnoreCase);
    internal Dictionary<string, int> ExitCodes { get; } = [];
    internal Dictionary<string, int> ToolExitCodes { get; } = [];
    internal Dictionary<string, int> ToolExitCodesByPath { get; } = new(StringComparer.OrdinalIgnoreCase);
    internal Dictionary<string, string> ToolOutputs { get; } = [];

    internal FakeDependenciesHost()
    {
        foreach (var id in new[] { "Kitware.CMake", "Git.Git", "Ninja-build.Ninja", "LLVM.LLVM", "KhronosGroup.VulkanSDK" })
            AddPackage(id);
        AddFFmpeg("9.0.2");
    }
    internal void AddPackage(string id)
    {
        switch (id)
        {
            case "Kitware.CMake": Files.Add(Path.Combine(ToolDirectory, "cmake.exe")); break;
            case "Git.Git": Files.Add(Path.Combine(ToolDirectory, "git.exe")); break;
            case "Ninja-build.Ninja": Files.Add(Path.Combine(ToolDirectory, "ninja.exe")); break;
            case "LLVM.LLVM": Files.Add(@"C:\Program Files\LLVM\bin\clang.exe"); break;
            case "Gyan.FFmpeg.Shared": AddFFmpeg("9.0.2"); break;
            case "KhronosGroup.VulkanSDK":
                Directories.Add(@"C:\VulkanSDK");
                Directories.Add(@"C:\VulkanSDK\1.4.321.1");
                Files.Add(@"C:\VulkanSDK\1.4.321.1\Bin\glslc.exe");
                Files.Add(@"C:\VulkanSDK\1.4.321.1\Lib\vulkan-1.lib");
                Files.Add(@"C:\VulkanSDK\1.4.321.1\Include\vulkan\vulkan.h");
                break;
        }
    }
    internal string AddFFmpeg(string name, bool executable = true, bool shared = true)
    {
        Directories.Add(DependenciesStep.FFmpegInstallRoot);
        var root = Path.Combine(DependenciesStep.FFmpegInstallRoot, name);
        Directories.Add(root);
        if (executable) Files.Add(Path.Combine(root, "bin", "ffmpeg.exe"));
        if (shared)
        {
            foreach (var component in new[] { "avcodec", "avformat", "avutil", "swresample", "swscale", "avdevice" })
            {
                Files.Add(Path.Combine(root, "bin", component + "-62.dll"));
                Files.Add(Path.Combine(root, "lib", component + ".lib"));
                Files.Add(Path.Combine(root, "include", "lib" + component, component + ".h"));
            }
        }
        return root;
    }
    internal void RemovePackage(string id)
    {
        if (id == "KhronosGroup.VulkanSDK")
        {
            Directories.RemoveWhere(path => path.StartsWith(@"C:\VulkanSDK", StringComparison.OrdinalIgnoreCase));
            Files.RemoveWhere(path => path.StartsWith(@"C:\VulkanSDK", StringComparison.OrdinalIgnoreCase));
        }
        else if (id == "Gyan.FFmpeg.Shared")
        {
            Directories.RemoveWhere(path => path.StartsWith(DependenciesStep.FFmpegInstallRoot, StringComparison.OrdinalIgnoreCase));
            Files.RemoveWhere(path => path.StartsWith(DependenciesStep.FFmpegInstallRoot, StringComparison.OrdinalIgnoreCase));
        }
        else
        {
            var suffix = id switch { "Kitware.CMake" => "cmake.exe", "Git.Git" => "git.exe", "Ninja-build.Ninja" => "ninja.exe", "LLVM.LLVM" => "clang.exe", _ => throw new ArgumentException(id) };
            Files.RemoveWhere(path => path.EndsWith("\\" + suffix, StringComparison.OrdinalIgnoreCase));
        }
    }
    internal override string? GetEnvironmentVariable(string name, EnvironmentVariableTarget target)
    {
        if (EnvironmentError != null) throw EnvironmentError;
        if (EnvironmentVariables.TryGetValue((name, target), out var value)) return value;
        if (name != "PATH") return null;
        return target switch { EnvironmentVariableTarget.Machine => MachinePath, EnvironmentVariableTarget.User => UserPath, _ => ProcessPath };
    }
    internal override bool FileExists(string path)
    {
        if (ProbeError != null) throw ProbeError;
        return Files.Contains(path);
    }
    internal override bool DirectoryExists(string path) => Directories.Contains(path);
    internal override string[] GetDirectories(string path, string pattern = "*") => Directories
        .Where(directory => string.Equals(Path.GetDirectoryName(directory), path, StringComparison.OrdinalIgnoreCase) &&
            FileSystemName.MatchesSimpleExpression(pattern, Path.GetFileName(directory), ignoreCase: true)).ToArray();
    internal override string[] GetFileSystemEntries(string path) => Files.Concat(Directories)
        .Where(entry => string.Equals(Path.GetDirectoryName(entry), path, StringComparison.OrdinalIgnoreCase)).ToArray();
    internal override void MoveDirectory(string source, string destination)
    {
        Moves.Add((source, destination));
        if (MoveError != null) throw MoveError;
        if (!Directories.Contains(source) || Directories.Contains(destination)) throw new IOException("Cannot move FFmpeg directory");
        Directories.Remove(source);
        Directories.Add(destination);
        foreach (var file in Files.Where(path => path.StartsWith(source + "\\", StringComparison.OrdinalIgnoreCase)).ToArray())
        {
            Files.Remove(file);
            Files.Add(destination + file[source.Length..]);
        }
    }
    internal override IDependencyProcess? StartProcess(ProcessStartInfo startInfo)
    {
        Processes.Add(startInfo);
        if (startInfo.FileName != "winget")
        {
            var executable = Path.GetFileName(startInfo.FileName).ToLowerInvariant();
            var (package, output) = executable switch
            {
                "cmake.exe" => ("Kitware.CMake", "cmake version 3.31.8"),
                "git.exe" => ("Git.Git", "git version 2.51.0.windows.1"),
                "ninja.exe" => ("Ninja-build.Ninja", "1.13.1"),
                "clang.exe" => ("LLVM.LLVM", "clang version 21.1.0\r\nTarget: x86_64-pc-windows-msvc"),
                "ffmpeg.exe" => ("Gyan.FFmpeg.Shared", "ffmpeg version 9.0.2-full_build-shared"),
                "glslc.exe" => ("KhronosGroup.VulkanSDK", "shaderc v2023.8 v2025.3-10-gc7e73e8\r\nspirv-tools v2025.4 v2022.4-970-g19042c89\r\nglslang 11.1.0-1302-gd213562e\r\n\r\nTarget: SPIR-V 1.0"),
                _ => throw new InvalidOperationException("Unexpected executable: " + startInfo.FileName)
            };
            return new FakeProcess(this, package, ToolExitCodesByPath.GetValueOrDefault(startInfo.FileName, ToolExitCodes.GetValueOrDefault(package)), ToolOutputs.GetValueOrDefault(package, output), null, null);
        }
        var args = startInfo.ArgumentList.Count > 0 ? startInfo.ArgumentList.ToArray() : startInfo.Arguments.Split(' ');
        var sourceQuery = args[0] == "source";
        if (sourceQuery)
        {
            if (SourceStartError != null) throw SourceStartError;
            if (NullSourceProcess) return null;
            return new FakeProcess(this, "source", SourceExitCode, SourceOutput, SourceReadError, null);
        }
        var id = args[Array.IndexOf(args, "--id") + 1];
        var faultApplies = FaultPackage == null || FaultPackage == id;
        if (faultApplies && InstallStartError != null) throw InstallStartError;
        if (faultApplies && NullInstallerProcess) return null;
        var code = ExitCodes.GetValueOrDefault(id);
        return new FakeProcess(this, id, code, InstallOutput, faultApplies ? OutputError : null, () =>
        {
            if (AutoInstall && (code == 0 || code == NoUpgrade)) AddPackage(id);
        });
    }
    private sealed class FakeProcess(FakeDependenciesHost host, string package, int code, string output, Exception? readError, Action? installed) : IDependencyProcess
    {
        public int ExitCode => code;
        public bool HasExited => installed == null || host.InstallWaitBarrier?.IsCompleted != false;
        public void Terminate() { }
        public string ReadStandardOutput()
        {
            if (readError != null) throw readError;
            return output;
        }
        public Task<string> ReadStandardOutputAsync() => readError == null ? Task.FromResult(output) : Task.FromException<string>(readError);
        public Task<string> ReadStandardErrorAsync() => Task.FromResult(host.InstallError);
        public void WaitForExit() { host.Waits.Add((package, false, installed != null)); installed?.Invoke(); }
        public async Task WaitForExitAsync(CancellationToken cancellationToken = default)
        {
            host.Waits.Add((package, cancellationToken.CanBeCanceled, installed != null));
            if (installed != null)
            {
                host.InstallerEntered.TrySetResult(true);
                if (host.InstallWaitBarrier != null) await host.InstallWaitBarrier.WaitAsync(cancellationToken);
            }
            if (installed != null && (host.FaultPackage == null || host.FaultPackage == package) && host.InstallWaitError != null)
                throw host.InstallWaitError;
            installed?.Invoke();
        }
        public void Dispose() { }
    }
}
