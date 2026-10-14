namespace Weave.Utils;

public static class ResourceExtractor
{
    internal static readonly string[] RequiredTemplates =
    [
        "CMakeLists.txt", "CMakePresets.json", "main.cpp", "user_project.hpp", ".gitignore", "validate.ps1", "run-report.ps1",
        "cmake\\mayaflux.cmake", "cmake\\shaders.cmake", "cmake\\build_community.cmake",
        "community\\module.cmake", "community\\community.json", "community\\CMakePresets.json",
        "community\\test\\CMakeLists.txt", "vscode\\settings.json", "vscode\\tasks.json", "vscode\\launch.json"
    ];

    public static void ExtractAllResources(string destinationDir) => ExtractAllResources(destinationDir, new ResourceExtractionHost());
    public static void ExtractTemplates(string destinationDir) => ExtractTemplates(destinationDir, new ResourceExtractionHost());
    public static void ExtractScripts(string destinationDir) => ExtractScripts(destinationDir, new ResourceExtractionHost());
    internal static void ExtractAllResources(string destinationDir, ResourceExtractionHost host) => Extract(destinationDir, host, true, true);
    internal static void ExtractTemplates(string destinationDir, ResourceExtractionHost host) => Extract(destinationDir, host, true, false);
    internal static void ExtractScripts(string destinationDir, ResourceExtractionHost host) => Extract(destinationDir, host, false, true);

    private static void Extract(string destinationDir, ResourceExtractionHost host, bool templates, bool scripts)
    {
        if (!WindowsInstallationPaths.IsSafeEnvironmentPath(destinationDir)) throw new ArgumentException("An absolute, safe installation directory is required.");
        var root = Path.GetFullPath(destinationDir).TrimEnd('\\', '/');
        if (root.Equals(Path.GetPathRoot(root)?.TrimEnd('\\', '/'), StringComparison.OrdinalIgnoreCase))
            throw new ArgumentException("Cannot install templates at a drive or share root.");

        var resources = new List<(string Name, string Destination, string Relative, bool Template)>();
        foreach (var name in host.ResourceNames)
        {
            bool isTemplate = templates && name.StartsWith("Weave.Resources.templates.", StringComparison.OrdinalIgnoreCase);
            bool isScript = scripts && name.StartsWith("Weave.Resources.scripts.", StringComparison.OrdinalIgnoreCase);
            if (!isTemplate && !isScript) continue;
            var prefix = isTemplate ? "Weave.Resources.templates." : "Weave.Resources.scripts.";
            var relative = ExtractResourceFilename(name[prefix.Length..]);
            var directory = Path.Combine(root, "share", "weave", isTemplate ? "templates" : "scripts");
            var destination = Path.GetFullPath(Path.Combine(directory, relative));
            if (!destination.StartsWith(directory + "\\", StringComparison.OrdinalIgnoreCase))
                throw new IOException($"Resource escapes its destination: {name}");
            resources.Add((name, destination, relative, isTemplate));
        }

        if (resources.Select(r => r.Destination).Distinct(StringComparer.OrdinalIgnoreCase).Count() != resources.Count)
            throw new IOException("Packaged resources contain conflicting Windows paths.");
        if (templates)
        {
            var missing = RequiredTemplates.Where(relative => !resources.Any(r => r.Template && r.Relative.Equals(relative, StringComparison.OrdinalIgnoreCase))).ToArray();
            if (missing.Length > 0) throw new IOException($"Required templates are missing from Weave: {string.Join(", ", missing)}");
        }

        WindowsInstallationPaths.EnsureNoReparseAncestors(root, host);
        foreach (var resource in resources) WindowsInstallationPaths.EnsureNoReparseAncestors(resource.Destination, host);
        var payloads = new List<(string Destination, byte[] Bytes)>();
        foreach (var resource in resources)
        {
            using var source = host.OpenResource(resource.Name) ?? throw new IOException($"Cannot read packaged resource: {resource.Name}");
            using var buffer = new MemoryStream();
            source.CopyTo(buffer);
            if (buffer.Length == 0) throw new IOException($"Packaged resource is empty: {resource.Name}");
            payloads.Add((resource.Destination, buffer.ToArray()));
        }

        foreach (var payload in payloads)
        {
            WindowsInstallationPaths.EnsureNoReparseAncestors(payload.Destination, host);
            host.CreateDirectory(Path.GetDirectoryName(payload.Destination)!);
            var temporary = payload.Destination + "." + Guid.NewGuid().ToString("N") + ".tmp";
            try
            {
                using (var file = host.CreateFile(temporary)) file.Write(payload.Bytes, 0, payload.Bytes.Length);
                host.MoveFile(temporary, payload.Destination);
            }
            catch (Exception failure)
            {
                try { if (host.FileExists(temporary)) host.DeleteFile(temporary); }
                catch (Exception cleanup) { throw new AggregateException($"Resource installation failed; temporary file could not be removed: {temporary}", failure, cleanup); }
                throw;
            }
        }
    }

    private static string ExtractResourceFilename(string name)
    {
        if (name == ".gitignore") return name;
        if (string.IsNullOrWhiteSpace(name) || name.StartsWith('.') || name.Contains("..") ||
            name.Any(c => char.IsControl(c) || c is '\\' or '/' or ':' or ';' or '"' or '<' or '>' or '|' or '*' or '?'))
            throw new IOException($"Unsafe packaged resource name: {name}");
        var lastDot = name.LastIndexOf('.');
        if (lastDot <= 0 || lastDot == name.Length - 1) throw new IOException($"Invalid packaged resource name: {name}");
        var relative = name[..lastDot].Replace('.', '\\') + name[lastDot..];
        foreach (var component in relative.Split('\\'))
        {
            if (component.EndsWith(' ') || component.EndsWith('.') ||
                System.Text.RegularExpressions.Regex.IsMatch(component, @"^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(\.|$)", System.Text.RegularExpressions.RegexOptions.IgnoreCase))
                throw new IOException($"Unsafe Windows resource filename: {name}");
        }
        return relative;
    }
}
