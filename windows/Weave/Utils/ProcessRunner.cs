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

    public static bool SetEnvironmentVariable(string name, string value, Logger logger)
    {
        try
        {
            Environment.SetEnvironmentVariable(name, value, EnvironmentVariableTarget.Machine);
            logger.Log($"[OK] Set {name}={value}");
            return true;
        }
        catch (Exception ex)
        {
            logger.Log($"[ERROR] Failed to set {name}: {ex.Message}");
            return false;
        }
    }

    public static bool AddToPath(string pathToAdd, Logger logger)
    {
        try
        {
            var currentPath = Environment.GetEnvironmentVariable("PATH", EnvironmentVariableTarget.Machine) ?? "";
            
            if (currentPath.Contains(pathToAdd, StringComparison.OrdinalIgnoreCase))
            {
                logger.Log($"[OK] Already in PATH: {pathToAdd}");
                return true;
            }

            var newPath = $"{pathToAdd};{currentPath}";
            Environment.SetEnvironmentVariable("PATH", newPath, EnvironmentVariableTarget.Machine);
            logger.Log($"[OK] Added to PATH: {pathToAdd}");
            return true;
        }
        catch (Exception ex)
        {
            logger.Log($"[ERROR] Failed to update PATH: {ex.Message}");
            return false;
        }
    }
}