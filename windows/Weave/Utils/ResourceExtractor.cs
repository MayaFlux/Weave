using System.Reflection;
using Weave.Shared;

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
        var resourceNames = assembly.GetManifestResourceNames()
            .Where(r => r.StartsWith(WeaveConstants.TEMPLATES_RESOURCE_PREFIX));

        foreach (var resourceName in resourceNames)
        {
            var filename = ExtractResourceFilename(resourceName, WeaveConstants.TEMPLATES_RESOURCE_PREFIX);
            var destPath = Path.Combine(templatesDir, filename);
            
            // Create subdirectories if needed
            Directory.CreateDirectory(Path.GetDirectoryName(destPath) ?? templatesDir);
            
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
        var resourceNames = assembly.GetManifestResourceNames()
            .Where(r => r.StartsWith(WeaveConstants.SCRIPTS_RESOURCE_PREFIX));

        foreach (var resourceName in resourceNames)
        {
            var filename = ExtractResourceFilename(resourceName, WeaveConstants.SCRIPTS_RESOURCE_PREFIX);
            var destPath = Path.Combine(scriptsDir, filename);

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

    public static string ExtractResourceAsText(string resourceName)
    {
        var assembly = Assembly.GetExecutingAssembly();
        using (var stream = assembly.GetManifestResourceStream(resourceName))
        {
            if (stream == null)
                throw new FileNotFoundException($"Resource not found: {resourceName}");

            using (var reader = new StreamReader(stream))
            {
                return reader.ReadToEnd();
            }
        }
    }

    private static string ExtractResourceFilename(string fullResourceName, string prefix)
    {
        var withoutPrefix = fullResourceName.Substring(prefix.Length).TrimStart('.');
        return withoutPrefix.Replace('.', Path.DirectorySeparatorChar);
    }
}