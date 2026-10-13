using System.Diagnostics;
using System.ComponentModel;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using Weave.Shared;
using Weave.UI.Pages;
using Weave.Utils;

namespace Weave.UI.Project;

internal sealed class CommunityCreationService(string templatesDir, ProjectCreationHost host, DependenciesHost git, Action<string> report)
{
    internal async Task<string> CreateAsync(string name, string destination, string description = "", string minimum = "", bool needsLila = false)
    {
        if (!Regex.IsMatch(name, @"\A[a-z][a-z0-9_]*\z") || Regex.IsMatch(name, @"\A(?:con|prn|aux|nul|com[1-9]|lpt[1-9])\z"))
            throw new ArgumentException("Module name must be snake_case; Windows device names are not allowed.");
        if (!WindowsInstallationPaths.IsSafeEnvironmentPath(destination) || !host.DirectoryExists(destination))
            throw new ArgumentException($"Module destination must be an existing absolute directory: {destination}");
        var module = Path.Combine(Path.GetFullPath(destination), name);
        if (host.DirectoryExists(module) || host.FileExists(module)) throw new IOException($"Destination already exists: {module}. Choose another name or directory; existing files were preserved.");
        WindowsInstallationPaths.EnsureNoReparseAncestors(module, host);
        WindowsInstallationPaths.EnsureNoReparseAncestors(templatesDir, host);
        var templates = new Dictionary<string, string>();
        foreach (var relative in new[] { ".gitignore", @"community\community.json", @"community\CMakePresets.json", @"community\module.cmake", @"community\test\CMakeLists.txt" })
        {
            var path = Path.Combine(templatesDir, relative);
            WindowsInstallationPaths.EnsureNoReparseAncestors(path, host);
            if (!host.FileExists(path)) throw new FileNotFoundException($"Required template missing: {relative}. Use Weave installation to reinstall its templates, then retry.");
            var text = host.ReadAllText(path);
            if (string.IsNullOrWhiteSpace(text) || Regex.Matches(text, @"@[A-Z][A-Z0-9_]*@").Any(marker => marker.Value != "@MODULE_NAME@"))
                throw new IOException($"Required template is empty or unfinished: {relative}. Use Weave installation to reinstall its templates, then retry.");
            templates[relative] = text;
        }
        try { using var presets = JsonDocument.Parse(templates[@"community\CMakePresets.json"]); }
        catch (JsonException error) { throw new IOException("Community CMake presets template contains invalid JSON. Use Weave installation to reinstall its templates, then retry.", error); }
        var manifestSource = templates[@"community\community.json"].Replace("@MODULE_NAME@", name);
        try { CommunityManifest.Parse(manifestSource, name); }
        catch (ManifestException error) { throw new IOException($"Community manifest template is invalid: {error.Message}. Reinstall Weave templates and retry.", error); }
        var json = JsonNode.Parse(manifestSource)!;
        if (description.Length > 0) json["description"] = description;
        if (minimum.Length > 0) json["min_version"] = minimum;
        json["needs_lila"] = needsLila;
        var manifest = CommunityManifest.Parse(json.ToJsonString(), name);
        var cmake = templates[@"community\module.cmake"].Replace("@MODULE_NAME@", name);
        cmake = Regex.Replace(cmake, "set\\(MF_NEEDS_LILA (?:ON|OFF)\\)", $"set(MF_NEEDS_LILA {(needsLila ? "ON" : "OFF")})");
        cmake = Regex.Replace(cmake, "set\\(MF_MIN_VERSION \"[^\"\\n]+\"\\)", $"set(MF_MIN_VERSION \"{manifest.GetProperty("min_version").GetString()}\")");
        var settings = $"set(MF_MIN_VERSION \"{manifest.GetProperty("min_version").GetString()}\")\nset(MF_NEEDS_LILA {(needsLila ? "ON" : "OFF")})";
        var testCmake = templates[@"community\test\CMakeLists.txt"].Replace("@MODULE_NAME@", name);
        testCmake = Regex.Replace(testCmake, @"(?m)^cmake_minimum_required\([^\r\n]*\)", match => match.Value + "\n\n" + settings);
        var created = false;
        try
        {
            host.CreateDirectory(module);
            created = true;
            host.CreateDirectory(Path.Combine(module, "src"));
            host.CreateDirectory(Path.Combine(module, "test"));
            host.WriteAllText(Path.Combine(module, ".gitignore"), templates[".gitignore"]);
            host.WriteAllText(Path.Combine(module, "CMakePresets.json"), templates[@"community\CMakePresets.json"]);
            host.WriteAllText(Path.Combine(module, "CMakeLists.txt"), $"cmake_minimum_required(VERSION 3.25)\nproject({name} LANGUAGES C CXX)\nenable_testing()\nadd_subdirectory(test)\n");
            host.WriteAllText(Path.Combine(module, name + ".cmake"), cmake);
            host.WriteAllText(Path.Combine(module, "community.json"), JsonSerializer.Serialize(manifest, new JsonSerializerOptions { WriteIndented = true }) + "\n");
            host.WriteAllText(Path.Combine(module, "src", name + ".cpp"), $"#include <MayaFlux/MayaFlux.hpp>\n\nnamespace {name} {{\n}}\n");
            host.WriteAllText(Path.Combine(module, "test", "CMakeLists.txt"), testCmake);
            host.WriteAllText(Path.Combine(module, "test", "test_" + name + ".cpp"), "int main()\n{\n    return 0;\n}\n");
            host.WriteAllText(Path.Combine(module, "README.md"), $"# {name}\n\nEdit src/{name}.cpp and add checks in test/test_{name}.cpp.\n\nFrom this directory, with MayaFlux and its required build tools installed:\n\n```powershell\ncmake --preset release\ncmake --build --preset release\nctest --preset release\n```\n\nUse the debug presets for a debug build.\n\nGit is optional for building. If repository setup needs attention, review the files and run:\n\n```powershell\ngit init -b main\ngit add -A\n```\n");
        }
        catch (Exception error)
        {
            if (created && host.DirectoryExists(module))
            {
                try { WindowsInstallationPaths.EnsureNoReparseAncestors(module, host); host.DeleteDirectory(module); }
                catch (Exception cleanup) { throw new AggregateException($"Module creation failed: {error.Message}. Incomplete module could not be removed: {module}. {cleanup.Message}. Review this directory before retrying.", error, cleanup); }
            }
            throw;
        }
        report($"Created community module at: {module}");
        try
        {
            await RunGitAsync(module, "init", "-b", "main");
            await RunGitAsync(module, "add", "-A");
            report("Initialized git repository");
        }
        catch (Exception error)
        {
            report($"WARNING: Module files are ready, but git setup needs attention: {error.Message}. From {module}, review the files and run git init -b main, then git add -A. You can build the module now.");
        }
        return module;
    }

    private async Task RunGitAsync(string directory, params string[] arguments)
    {
        var request = new ProcessStartInfo("git") { WorkingDirectory = directory, UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true, CreateNoWindow = true };
        foreach (var argument in arguments) request.ArgumentList.Add(argument);
        IDependencyProcess process;
        try { process = git.StartProcess(request) ?? throw new InvalidOperationException("Git could not start. Install Git for Windows or initialize the repository later."); }
        catch (Win32Exception error) when (error.NativeErrorCode is 2 or 3)
        {
            throw new InvalidOperationException("Git was not found. Install Git for Windows or initialize the repository later.", error);
        }
        using var ownedProcess = process;
        using var deadline = new CancellationTokenSource(git.ProcessTimeout);
        try
        {
            await process.WaitForExitAsync(deadline.Token);
            var output = await process.ReadStandardOutputAsync().WaitAsync(deadline.Token);
            var error = await process.ReadStandardErrorAsync().WaitAsync(deadline.Token);
            if (process.ExitCode != 0) throw new IOException($"git {arguments[0]} failed (exit {process.ExitCode}): {Tail(error)} {Tail(output)}");
        }
        catch
        {
            if (!process.HasExited)
            {
                process.Terminate();
                await process.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(5));
            }
            if (deadline.IsCancellationRequested) throw new TimeoutException($"git {arguments[0]} timed out; its process was stopped");
            throw;
        }
    }

    private static string Tail(string text) => text.Length > 4096 ? text[^4096..] : text;
}
