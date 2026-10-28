using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Threading.Tasks;
using Weave.Utils;

namespace Weave.Utils;

public static class ProcessRunner
{
    public static async Task<int> RunPowerShellScriptAsync(string scriptPath, List<string> args, Logger logger, Action<string>? outputCallback = null)
    {
        try
        {
            if (!File.Exists(scriptPath))
            {
                logger.Log($"[ERROR] Script not found: {scriptPath}");
                return 1;
            }

            var argsString = string.Join(" ", args.Select(a => $"\"{a}\""));
            var arguments = $"-ExecutionPolicy Bypass -NoProfile -File \"{scriptPath}\" {argsString}";

            logger.Log($"Running PowerShell script: {Path.GetFileName(scriptPath)}");

            var psi = new ProcessStartInfo
            {
                FileName = "powershell.exe",
                Arguments = arguments,
                UseShellExecute = false,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                CreateNoWindow = true
            };

            using (var process = Process.Start(psi))
            {
                if (process == null)
                {
                    logger.Log("[ERROR] Failed to start PowerShell process");
                    return 1;
                }

                while (!process.StandardOutput.EndOfStream)
                {
                    var line = process.StandardOutput.ReadLine();
                    if (!string.IsNullOrEmpty(line))
                    {
                        logger.Log(line);
                        outputCallback?.Invoke(line);
                    }
                }

                var errorOutput = process.StandardError.ReadToEnd();
                if (!string.IsNullOrEmpty(errorOutput))
                {
                    logger.Log($"[STDERR] {errorOutput}");
                    outputCallback?.Invoke(errorOutput);
                }

                process.WaitForExit();
                return process.ExitCode;
            }
        }
        catch (Exception ex)
        {
            logger.Log($"[ERROR] PowerShell execution failed: {ex.Message}");
            return 1;
        }
    }

    public static bool SetEnvironmentVariable(string name, string value, Logger logger) => SetEnvironmentVariable(name, value, logger, new WindowsInstallationHost());

    internal static bool SetEnvironmentVariable(string name, string value, Logger logger, WindowsInstallationHost host)
    {
        try
        {
            host.SetEnvironmentVariable(name, value, EnvironmentVariableTarget.Machine);
            logger.Log($"[OK] Set {name}={value}");
            return true;
        }
        catch (Exception ex)
        {
            logger.Log($"[ERROR] Failed to set {name}: {ex.Message}");
            return false;
        }
    }

    public static bool AddToPath(string pathToAdd, Logger logger) => AddToPath(pathToAdd, logger, new WindowsInstallationHost());

    internal static bool AddToPath(string pathToAdd, Logger logger, WindowsInstallationHost host)
    {
        try
        {
            if (!WindowsInstallationPaths.IsSafeEnvironmentPath(pathToAdd)) throw new ArgumentException("PATH entry must be an absolute path without separators or control characters.");
            var currentPath = host.GetEnvironmentVariable("PATH", EnvironmentVariableTarget.Machine) ?? "";
            
            if (ContainsPath(currentPath, pathToAdd, host))
            {
                logger.Log($"[OK] Already in PATH: {pathToAdd}");
                return true;
            }

            var newPath = string.IsNullOrEmpty(currentPath) ? pathToAdd : $"{pathToAdd};{currentPath}";
            host.SetEnvironmentVariable("PATH", newPath, EnvironmentVariableTarget.Machine);
            logger.Log($"[OK] Added to PATH: {pathToAdd}");
            return true;
        }
        catch (Exception ex)
        {
            logger.Log($"[ERROR] Failed to update PATH: {ex.Message}");
            return false;
        }
    }

    public static void AppendToEnvironmentVariable(string name, string path, Logger logger) => AppendToEnvironmentVariable(name, path, logger, new WindowsInstallationHost());

    internal static bool PreferPath(string path, Logger logger, WindowsInstallationHost host)
    {
        try
        {
            if (!WindowsInstallationPaths.IsSafeEnvironmentPath(path)) throw new ArgumentException("The selected tool directory must be a safe absolute path.");
            var current = host.GetEnvironmentVariable("PATH", EnvironmentVariableTarget.Machine) ?? "";
            var entries = current.Split(';').Where(entry => !ContainsPath(entry, path, host));
            var updated = string.Join(';', new[] { path }.Concat(entries.Where(entry => !string.IsNullOrWhiteSpace(entry))));
            if (updated != current) host.SetEnvironmentVariable("PATH", updated, EnvironmentVariableTarget.Machine);
            logger.Log($"[OK] Selected tool directory takes precedence in PATH: {path}");
            return true;
        }
        catch (Exception error)
        {
            logger.Log($"[ERROR] Failed to select tool directory in PATH: {error.Message}");
            return false;
        }
    }

    internal static bool AppendToEnvironmentVariable(string name, string path, Logger logger, WindowsInstallationHost host)
    {
        try
        {
            if (!WindowsInstallationPaths.IsSafeEnvironmentPath(path)) throw new ArgumentException($"Invalid path for {name}.");
            var existing = host.GetEnvironmentVariable(name, EnvironmentVariableTarget.Machine) ?? "";
            if (ContainsPath(existing, path, host)) return true;
            var newValue = string.IsNullOrEmpty(existing) ? path : $"{existing};{path}";
            return SetEnvironmentVariable(name, newValue, logger, host);
        }
        catch (Exception ex)
        {
            logger.Log($"[ERROR] Failed to update {name}: {ex.Message}");
            return false;
        }
    }

    internal static bool MergeEnvironmentPaths(string name, IEnumerable<string> paths, Logger logger, WindowsInstallationHost host)
    {
        try
        {
            var additions = paths.ToArray();
            if (additions.Any(path => !WindowsInstallationPaths.IsSafeEnvironmentPath(path) || !host.DirectoryExists(path)))
                throw new ArgumentException($"Required {name} directories are missing or unsafe.");
            var existing = host.GetEnvironmentVariable(name, EnvironmentVariableTarget.Machine) ?? "";
            var entries = new List<string>();
            foreach (var entry in existing.Split(';').Select(entry => entry.Trim()).Where(entry => entry.Length > 0).Concat(additions))
                if (!ContainsPath(string.Join(';', entries), entry, host)) entries.Add(entry);
            var value = string.Join(';', entries);
            return value == existing || SetEnvironmentVariable(name, value, logger, host);
        }
        catch (Exception error)
        {
            logger.Log($"[ERROR] Failed to update {name}: {error.Message}");
            return false;
        }
    }

    public static void AddIncludeDirectory(string path, Logger logger) => AddIncludeDirectory(path, logger, new WindowsInstallationHost());

    internal static bool AddIncludeDirectory(string path, Logger logger, WindowsInstallationHost host)
    {
        return host.DirectoryExists(path) && AppendToEnvironmentVariable("CPATH", path, logger, host);
    }

    public static void AddLibraryDirectory(string path, Logger logger) => AddLibraryDirectory(path, logger, new WindowsInstallationHost());

    internal static bool AddLibraryDirectory(string path, Logger logger, WindowsInstallationHost host)
    {
        return host.DirectoryExists(path) && AppendToEnvironmentVariable("LIBRARY_PATH", path, logger, host);
    }

    private static bool ContainsPath(string existing, string path, WindowsInstallationHost host)
    {
        string Normalize(string entry)
        {
            entry = entry.Trim().Trim('"');
            entry = System.Text.RegularExpressions.Regex.Replace(entry, "%([^%]+)%", match =>
                host.GetEnvironmentVariable(match.Groups[1].Value, EnvironmentVariableTarget.Machine) ?? match.Value);
            return Path.IsPathFullyQualified(entry) ? Path.GetFullPath(entry).TrimEnd('\\', '/') : entry;
        }
        var normalized = Normalize(path);
        return existing.Split(';').Any(entry => Normalize(entry).Equals(normalized, StringComparison.OrdinalIgnoreCase));
    }
}
