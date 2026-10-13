using System.Diagnostics;
using System.ComponentModel;
using System.Net.Http;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace Weave.Shared;

public sealed class CommunityUpdate
{
    private const string RegistryUrl =
        "https://raw.githubusercontent.com/MayaFlux/community-sources-registry/main/registry.json";

    private readonly HttpClient http;
    private readonly Action<string> report;
    private readonly string? mayafluxRoot;
    private readonly CommunityUpdateHost host;

    public CommunityUpdate(HttpClient http, Action<string> report, string? mayafluxRoot, CommunityUpdateHost? host = null)
    {
        this.http = http;
        this.report = report;
        this.mayafluxRoot = mayafluxRoot;
        this.host = host ?? new CommunityUpdateHost();
    }

    public static IReadOnlyList<string> ParseNames(IEnumerable<string> values)
    {
        var names = new List<string>();
        var seen = new HashSet<string>(StringComparer.Ordinal);

        foreach (var value in values)
        {
            foreach (var part in value.Split(','))
            {
                var name = part.Trim();
                if (name.Length == 0)
                    throw new InvalidOperationException("Empty module name in request");

                if (!Regex.IsMatch(name, @"\A[a-z][a-z0-9_]*\z") ||
                    Regex.IsMatch(name, @"\A(?:con|prn|aux|nul|com[1-9]|lpt[1-9])\z"))
                    throw new InvalidOperationException($"Invalid module name: {name}");

                if (seen.Add(name))
                    names.Add(name);
            }
        }

        if (names.Count == 0)
            throw new InvalidOperationException("At least one module name is required");

        return names;
    }

    private static Version ParseVersion(string text, string label)
    {
        var pattern = @"\A(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\z";
        if (!Regex.IsMatch(text, pattern) || !Version.TryParse(text, out var version))
            throw new InvalidOperationException($"Malformed {label} version: {text}");

        return version;
    }

    private (string Text, Version Parsed) InstalledVersion()
    {
        var root = string.IsNullOrWhiteSpace(mayafluxRoot) ? @"C:\MayaFlux" : mayafluxRoot!;
        var checkedPaths = new List<string>();

        foreach (var library in new[] { "lib", "lib64" })
        {
            var path = Path.Combine(root, library, "cmake", "MayaFlux", "MayaFluxConfigVersion.cmake");
            checkedPaths.Add(path);
            if (!File.Exists(path))
                continue;

            var source = File.ReadAllText(path);
            var match = Regex.Match(source, "set\\(PACKAGE_VERSION\\s+\"([^\"]+)\"\\)");
            if (!match.Success)
                throw new InvalidOperationException($"Malformed MayaFlux version file: {path}. Repair or reinstall MayaFlux, then retry.");

            var text = match.Groups[1].Value;
            try { return (text, ParseVersion(text, "installed MayaFlux")); }
            catch (InvalidOperationException error)
            {
                throw new InvalidOperationException($"{error.Message}. Repair or reinstall MayaFlux at {root}, then retry.", error);
            }
        }

        throw new InvalidOperationException(
            $"MayaFlux version file not found. Repair or reinstall MayaFlux at {root}, then retry. Checked: {string.Join(", ", checkedPaths)}");
    }

    private async Task<Dictionary<string, (string Repo, Version Minimum)>> RegistryAsync()
    {
        report("Fetching registry...");
        string source;
        try { source = await http.GetStringAsync(RegistryUrl); }
        catch (HttpRequestException error)
        {
            throw new InvalidOperationException($"Could not fetch the community registry: {error.Message}. Check your connection and retry.", error);
        }
        catch (TaskCanceledException error)
        {
            throw new InvalidOperationException("Fetching the community registry timed out. Check your connection and retry.", error);
        }
        JsonDocument parsed;
        try { parsed = JsonDocument.Parse(source); }
        catch (JsonException error)
        {
            throw new InvalidOperationException("The community registry contains invalid JSON. Your project was not changed; retry later.", error);
        }
        using var document = parsed;
        if (document.RootElement.ValueKind != JsonValueKind.Array)
            throw new InvalidOperationException("Community registry must be an array");

        var result = new Dictionary<string, (string, Version)>(StringComparer.Ordinal);
        foreach (var entry in document.RootElement.EnumerateArray())
        {
            if (entry.ValueKind != JsonValueKind.Object ||
                !entry.TryGetProperty("name", out var nameValue) || nameValue.ValueKind != JsonValueKind.String ||
                !entry.TryGetProperty("repo", out var repoValue) || repoValue.ValueKind != JsonValueKind.String ||
                !entry.TryGetProperty("min_version", out var minimumValue) || minimumValue.ValueKind != JsonValueKind.String)
            {
                throw new InvalidOperationException("Community registry contains an invalid entry");
            }

            var name = nameValue.GetString()!;
            var repo = repoValue.GetString()!;
            var minimum = minimumValue.GetString()!;

            if (!Regex.IsMatch(name, @"\A[a-z][a-z0-9_]*\z") ||
                !Uri.TryCreate(repo, UriKind.Absolute, out var url) ||
                url.Scheme != Uri.UriSchemeHttps || string.IsNullOrEmpty(url.Host))
            {
                throw new InvalidOperationException($"Invalid registry entry for {name}");
            }

            if (!result.TryAdd(name, (repo, ParseVersion(minimum, $"registry entry for {name}"))))
                throw new InvalidOperationException($"Duplicate registry entry for {name}");
        }

        return result;
    }

    private async Task CloneAsync(string repo, string destination)
    {
        var start = new ProcessStartInfo("git")
        {
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true
        };

        foreach (var argument in new[] { "clone", "--progress", "--depth=1", repo, destination })
            start.ArgumentList.Add(argument);

        Process process;
        try { process = host.StartProcess(start); }
        catch (Win32Exception error)
        {
            throw new InvalidOperationException(error.NativeErrorCode is 2 or 3
                ? "Git was not found. Install Git for Windows, reopen Weave and retry."
                : $"Git could not start: {error.Message}. Check access to Git and retry.", error);
        }
        using var ownedProcess = process;
        using var deadline = new CancellationTokenSource(host.CloneTimeout);

        async Task DrainAsync(StreamReader stream)
        {
            while (await stream.ReadLineAsync(deadline.Token) is { } line)
                report(line.Length > 4096 ? line[..4096] + "…" : line);
        }

        try
        {
            await Task.WhenAll(
                DrainAsync(process.StandardOutput),
                DrainAsync(process.StandardError),
                process.WaitForExitAsync(deadline.Token));
        }
        catch
        {
            if (!process.HasExited)
            {
                process.Kill(true);
                await process.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(5));
            }
            if (deadline.IsCancellationRequested)
                throw new InvalidOperationException("Git clone timed out. Check your connection and retry.");
            throw;
        }

        if (process.ExitCode != 0)
        {
            var name = Path.GetFileName(destination);
            throw new InvalidOperationException($"Git clone failed for {name} (exit {process.ExitCode}). Check the Git output and your connection, then retry");
        }
    }

    public async Task<IReadOnlyList<string>> RunAsync(string projectPath, IEnumerable<string> values)
    {
        var names = ParseNames(values);
        if (string.IsNullOrWhiteSpace(projectPath) || !Path.IsPathFullyQualified(projectPath))
            throw new InvalidOperationException("Select the full path to your project directory and retry.");
        var project = Path.GetFullPath(projectPath);
        EnsureUnlinked(project);
        EnsureUnlinked(Path.Combine(project, "CMakeLists.txt"));
        if (!File.Exists(Path.Combine(project, "CMakeLists.txt")))
            throw new InvalidOperationException($"Not a MayaFlux project: {project}");

        var registry = await RegistryAsync();
        var installed = InstalledVersion();
        var community = Path.Combine(project, "community");
        EnsureUnlinked(community);
        EnsureUnlinked(Path.Combine(project, "community.cmake"));
        var cloned = new List<string>();

        try
        {
            foreach (var name in names)
            {
                if (!registry.ContainsKey(name))
                    throw new InvalidOperationException($"Module {name} is not in the community registry");
                if (installed.Parsed.CompareTo(registry[name].Minimum) < 0)
                    throw new InvalidOperationException($"Module {name} requires MayaFlux >= {registry[name].Minimum}; found {installed.Text}. Update MayaFlux and retry.");
            }

            foreach (var name in names)
            {
                var entry = registry[name];
                var module = Path.Combine(community, name);
                EnsureUnlinked(module);
                report($"Preparing {name}...");

                if (File.Exists(module))
                    throw new InvalidOperationException($"Community path for {name} is not a directory: {module}");

                if (!Directory.Exists(module))
                {
                    Directory.CreateDirectory(community);
                    report($"Cloning {entry.Repo} into {module}");

                    try
                    {
                        await CloneAsync(entry.Repo, module);
                    }
                    catch (Exception error)
                    {
                        if (Directory.Exists(module))
                        {
                            try
                            {
                                EnsureUnlinkedTree(module);
                                host.DeleteIncompleteClone(module);
                                report($"Removed incomplete clone for {name}; you can retry.");
                            }
                            catch (Exception cleanup)
                            {
                                throw new InvalidOperationException($"{error.Message}. Could not clean incomplete clone at {module}: {cleanup.Message}. Review and remove that incomplete directory before retrying.", error);
                            }
                        }
                        throw;
                    }
                    finally
                    {
                        if (Directory.Exists(module))
                            cloned.Add(name);
                    }
                }
                else
                {
                    report($"Using existing community/{name}");
                }

                foreach (var relative in new[] { $"{name}.cmake", "src", "community.json" })
                    EnsureUnlinked(Path.Combine(module, relative));
                if (!File.Exists(Path.Combine(module, $"{name}.cmake")))
                    throw new InvalidOperationException($"Module {name} is missing {name}.cmake");

                if (!Directory.Exists(Path.Combine(module, "src")))
                    throw new InvalidOperationException($"Module {name} is missing src/");

                var manifestPath = Path.Combine(module, "community.json");
                if (!File.Exists(manifestPath))
                    throw new InvalidOperationException($"Module {name} is missing community.json");

                var manifest = CommunityManifest.Parse(await File.ReadAllTextAsync(manifestPath), name);
                var required = ParseVersion(
                    manifest.GetProperty("min_version").GetString()!, $"manifest for {name}");

                if (entry.Minimum.CompareTo(required) > 0)
                    required = entry.Minimum;

                if (installed.Parsed.CompareTo(required) < 0)
                {
                    throw new InvalidOperationException(
                        $"Module {name} requires MayaFlux >= {required}; found {installed.Text}. Update MayaFlux and retry.");
                }

                if (manifest.GetProperty("packages").EnumerateObject().Any() ||
                    manifest.GetProperty("manual_installs").GetArrayLength() > 0 ||
                    manifest.GetProperty("assets").GetArrayLength() > 0)
                {
                    report($"{name} has declared requirements pending review");
                }

                report($"Validated {name}");
            }

            var cmake = Path.Combine(project, "community.cmake");
            var current = File.Exists(cmake) ? await File.ReadAllTextAsync(cmake) : "";
            var existing = current.Split('\n', StringSplitOptions.RemoveEmptyEntries)
                .Select(line => line.Trim())
                .ToHashSet(StringComparer.Ordinal);
            var additions = names.Where(name => !existing.Contains(name)).ToArray();

            if (additions.Length > 0)
            {
                var updated = current;
                if (updated.Length > 0 && !updated.EndsWith('\n'))
                    updated += "\n";

                updated += string.Concat(additions.Select(name => name + "\n"));
                var temporary = cmake + ".weave-" + Guid.NewGuid().ToString("N");

                try
                {
                    await File.WriteAllTextAsync(temporary, updated, new UTF8Encoding(false));
                    host.MoveRegistration(temporary, cmake);
                }
                finally
                {
                    if (File.Exists(temporary))
                        File.Delete(temporary);
                }
            }

            foreach (var name in names)
            {
                report(additions.Contains(name)
                    ? $"Registered {name} in community.cmake"
                    : $"{name} was already registered");
            }

            report("Done. Rebuild the project to include the validated modules.");
            return names;
        }
        catch (Exception error)
        {
            var remaining = cloned.Count == 0 ? "none" : string.Join(", ", cloned);
            throw new InvalidOperationException(
                $"{error.Message}. Clones left in community/: {remaining}. Existing modules were preserved; review or repair any incomplete module before retrying.", error);
        }
    }

    private void EnsureUnlinked(string path)
    {
        for (var current = Path.GetFullPath(path); current != null; current = Path.GetDirectoryName(current))
        {
            try
            {
                if ((host.GetAttributes(current) & FileAttributes.ReparsePoint) != 0)
                    throw new InvalidOperationException($"Project update cannot use a linked path: {current}. Select an ordinary project directory and retry.");
            }
            catch (FileNotFoundException) { }
            catch (DirectoryNotFoundException) { }
        }
    }

    private void EnsureUnlinkedTree(string path)
    {
        EnsureUnlinked(path);
        foreach (var entry in Directory.EnumerateFileSystemEntries(path))
        {
            EnsureUnlinked(entry);
            if (Directory.Exists(entry)) EnsureUnlinkedTree(entry);
        }
    }
}
