using Weave.Utils;

namespace Weave.UI.Pages;

internal static class MayaFluxReleaseValidation
{
    internal static Version ParseVersion(string value, out bool development)
    {
        var number = value.StartsWith('v') ? value[1..] : value;
        development = number.EndsWith("-dev", StringComparison.Ordinal);
        if (development) number = number[..^4];
        if (!Version.TryParse(number, out var version) || version.Build < 0)
            throw new InvalidDataException($"Invalid MayaFlux version: {value}");
        return version;
    }

    internal static void ValidateDirectory(string path, MayaFluxDownloadHost host)
    {
        if (!Path.IsPathFullyQualified(path) || path.StartsWith(@"\\"))
            throw new IOException("Installation and temporary paths must be local absolute directories");
        var full = Path.TrimEndingDirectorySeparator(Path.GetFullPath(path));
        if (full.Equals(Path.TrimEndingDirectorySeparator(Path.GetPathRoot(full)!), StringComparison.OrdinalIgnoreCase))
            throw new IOException("A drive root cannot be an installation or temporary directory");
        foreach (var segment in path[3..].Split(new[] { '\\', '/' }, StringSplitOptions.RemoveEmptyEntries))
            if (segment.EndsWith('.') || segment.EndsWith(' ') || segment.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0)
                throw new IOException("Installation or temporary path contains an unsafe directory name");
        foreach (var folder in new[] { Environment.SpecialFolder.Windows, Environment.SpecialFolder.System,
            Environment.SpecialFolder.ProgramFiles, Environment.SpecialFolder.ProgramFilesX86, Environment.SpecialFolder.UserProfile })
        {
            var protectedPath = Environment.GetFolderPath(folder);
            if (!string.IsNullOrEmpty(protectedPath) && full.Equals(Path.TrimEndingDirectorySeparator(protectedPath), StringComparison.OrdinalIgnoreCase))
                throw new IOException("A system or profile directory cannot be an installation or temporary directory");
        }
        WindowsInstallationPaths.EnsureNoReparseAncestors(full, host);
    }

    internal static bool ContainsPath(string parent, string child) => child.Equals(parent, StringComparison.OrdinalIgnoreCase) ||
        child.StartsWith(Path.TrimEndingDirectorySeparator(parent) + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase);
}
