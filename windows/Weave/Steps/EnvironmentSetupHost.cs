using Weave.Utils;
using System.Diagnostics;
using Microsoft.Win32;

namespace Weave.UI.Pages;

internal class EnvironmentSetupHost : WindowsInstallationHost
{
    internal virtual Logger CreateLogger() => new();

    internal virtual async Task<WindowsCompilerPaths> GetMicrosoftCompilerPathsAsync()
    {
        var vswhere = new[] { Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86), Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles) }
            .Select(root => Path.Combine(root, "Microsoft Visual Studio", "Installer", "vswhere.exe")).FirstOrDefault(FileExists)
            ?? throw new FileNotFoundException("Visual Studio discovery tool is missing. Run prerequisite setup again.");
        var request = new ProcessStartInfo(vswhere) { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true };
        foreach (var argument in new[] { "-latest", "-products", "*", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath" }) request.ArgumentList.Add(argument);
        using var process = Process.Start(request) ?? throw new IOException("Could not start Visual Studio discovery.");
        var output = process.StandardOutput.ReadToEndAsync();
        var errors = process.StandardError.ReadToEndAsync();
        try { await process.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(30)); }
        catch (TimeoutException) { process.Kill(true); throw new TimeoutException("Visual Studio discovery timed out. Retry environment setup."); }
        var installation = (await output).Trim();
        if (process.ExitCode != 0 || !WindowsInstallationPaths.IsSafeEnvironmentPath(installation) || !DirectoryExists(installation))
            throw new IOException($"Could not find Microsoft x64 C++ tools. Run prerequisite setup again. {await errors}");
        var version = File.ReadAllText(Path.Combine(installation, "VC", "Auxiliary", "Build", "Microsoft.VCToolsVersion.default.txt")).Trim();
        if (!Version.TryParse(version, out _)) throw new IOException("Microsoft C++ tools report an invalid default version.");
        var tools = Path.Combine(installation, "VC", "Tools", "MSVC", version);
        if (!FileExists(Path.Combine(tools, "lib", "x64", "oldnames.lib")) || !FileExists(Path.Combine(tools, "lib", "x64", "msvcrtd.lib")))
            throw new IOException("Microsoft x64 runtime libraries are missing. Run prerequisite setup again.");
        using var registry = RegistryKey.OpenBaseKey(RegistryHive.LocalMachine, RegistryView.Registry64);
        using var kits = registry.OpenSubKey(@"SOFTWARE\Microsoft\Windows Kits\Installed Roots");
        var sdk = kits?.GetValue("KitsRoot10") as string;
        if (sdk == null || !WindowsInstallationPaths.IsSafeEnvironmentPath(sdk) || !DirectoryExists(Path.Combine(sdk, "Lib")))
            throw new IOException("Windows SDK libraries are missing. Run prerequisite setup again.");
        var sdkVersion = WindowsInstallationPaths.OrderVersionDirectories(GetDirectories(Path.Combine(sdk, "Lib")))
            .Select(Path.GetFileName).FirstOrDefault(candidate =>
                FileExists(Path.Combine(sdk, "Lib", candidate!, "um", "x64", "kernel32.lib")) &&
                FileExists(Path.Combine(sdk, "Lib", candidate!, "ucrt", "x64", "ucrt.lib")) &&
                FileExists(Path.Combine(sdk, "Include", candidate!, "um", "Windows.h")) &&
                FileExists(Path.Combine(sdk, "Include", candidate!, "ucrt", "stdio.h")))
            ?? throw new IOException("A complete Windows x64 SDK was not found. Run prerequisite setup again.");
        var includes = new[] { Path.Combine(tools, "include") }.Concat(new[] { "ucrt", "shared", "um", "winrt", "cppwinrt" }
            .Select(folder => Path.Combine(sdk, "Include", sdkVersion, folder)).Where(DirectoryExists)).ToArray();
        return new(includes, [Path.Combine(tools, "lib", "x64"), Path.Combine(sdk, "Lib", sdkVersion, "ucrt", "x64"), Path.Combine(sdk, "Lib", sdkVersion, "um", "x64")]);
    }
}

internal sealed record WindowsCompilerPaths(string[] Includes, string[] Libraries);
