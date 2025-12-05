using System.Reflection;
using Weave.Shared;
using System.IO;

namespace Weave.Utils;

public static class ResourceExtractor
{
    public static void ExtractAllResources(string destinationDir)
    {
        ExtractTemplates(destinationDir);
        ExtractScripts(destinationDir);
    }

    public static void ExtractTemplates(string destinationDir)
    {
        var templatesDir = Path.Combine(destinationDir, "share", "weave", "templates");
        Directory.CreateDirectory(templatesDir);

        var assembly = Assembly.GetExecutingAssembly();
        var resourceNames = assembly.GetManifestResourceNames();

        var templateResources = resourceNames
            .Where(r => r.StartsWith("Weave.Resources.templates", StringComparison.OrdinalIgnoreCase))
            .ToList();

        foreach (var resourceName in templateResources)
        {
            var relativePath = ExtractResourceFilename(resourceName, "Weave.Resources.templates");
            var destPath = Path.Combine(templatesDir, relativePath);

            var destDir = Path.GetDirectoryName(destPath);
            if (!string.IsNullOrEmpty(destDir))
            {
                Directory.CreateDirectory(destDir);
            }

            using (var stream = assembly.GetManifestResourceStream(resourceName))
            {
                if (stream != null)
                {
                    using (var fileStream = File.Create(destPath))
                    {
                        stream.CopyTo(fileStream);
                    }
                }
            }
        }
    }

    public static void ExtractScripts(string destinationDir)
    {
        var scriptsDir = Path.Combine(destinationDir, "share", "weave", "scripts");
        Directory.CreateDirectory(scriptsDir);

        var assembly = Assembly.GetExecutingAssembly();
        var resourceNames = assembly.GetManifestResourceNames();

        var scriptResources = resourceNames
            .Where(r => r.StartsWith("Weave.Resources.scripts", StringComparison.OrdinalIgnoreCase))
            .ToList();

        foreach (var resourceName in scriptResources)
        {
            var relativePath = ExtractResourceFilename(resourceName, "Weave.Resources.scripts");
            var destPath = Path.Combine(scriptsDir, relativePath);

            using (var stream = assembly.GetManifestResourceStream(resourceName))
            {
                if (stream != null)
                {
                    using (var fileStream = File.Create(destPath))
                    {
                        stream.CopyTo(fileStream);
                    }
                }
            }
        }
    }

    private static string ExtractResourceFilename(string fullResourceName, string prefix)
    {
        var withoutPrefix = fullResourceName.Substring(prefix.Length).TrimStart('.');
        var lastDot = withoutPrefix.LastIndexOf('.');
        if (lastDot > 0 && lastDot < withoutPrefix.Length - 1)
        {
            var nameWithoutExt = withoutPrefix.Substring(0, lastDot);
            var ext = withoutPrefix.Substring(lastDot);
            nameWithoutExt = nameWithoutExt.Replace('.', Path.DirectorySeparatorChar);
            return nameWithoutExt + ext;
        }
        return withoutPrefix.Replace('.', Path.DirectorySeparatorChar);
    }
}