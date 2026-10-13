using System;
using System.Diagnostics;
using System.IO;
using System.Net.Http;
using System.Threading.Tasks;
using Weave.Utils;

namespace Weave.Utils;

public static class FileOperations
{
    public static async Task<bool> DownloadFileAsync(string url, string destinationPath, Logger logger, Action<long, long>? progressCallback = null)
    {
        try
        {
            logger.Log($"Downloading from: {url}");
            
            Directory.CreateDirectory(Path.GetDirectoryName(destinationPath) ?? ".");

            using (var client = new HttpClient())
            {
                using (var response = await client.GetAsync(url, HttpCompletionOption.ResponseHeadersRead))
                {
                    response.EnsureSuccessStatusCode();

                    var totalBytes = response.Content.Headers.ContentLength ?? -1L;
                    var canReportProgress = totalBytes != -1 && progressCallback != null;

                    using (var contentStream = await response.Content.ReadAsStreamAsync())
                    {
                        using (var fileStream = new FileStream(destinationPath, FileMode.Create, FileAccess.Write, FileShare.None, 8192, true))
                        {
                            var totalRead = 0L;
                            var buffer = new byte[8192];
                            int read;

                            while ((read = await contentStream.ReadAsync(buffer, 0, buffer.Length)) != 0)
                            {
                                await fileStream.WriteAsync(buffer, 0, read);
                                totalRead += read;

                                if (canReportProgress)
                                {
                                    progressCallback?.Invoke(totalRead, totalBytes);
                                }
                            }
                        }
                    }
                }
            }

            var fileInfo = new FileInfo(destinationPath);
            logger.Log($"[OK] Downloaded: {FormatBytes(fileInfo.Length)}");
            return true;
        }
        catch (Exception ex)
        {
            logger.Log($"[ERROR] Download failed: {ex.Message}");
            return false;
        }
    }

    public static bool Extract7z(string archivePath, string destinationPath, Logger logger)
        => Extract7z(archivePath, destinationPath, logger, new ArchiveExtractionHost());

    internal static bool Extract7z(string archivePath, string destinationPath, Logger logger, ArchiveExtractionHost host, Action<string>? report = null)
        => Extract7zAsync(archivePath, destinationPath, logger, host, report).GetAwaiter().GetResult();

    private static async Task<bool> Extract7zAsync(string archivePath, string destinationPath, Logger logger, ArchiveExtractionHost host, Action<string>? report)
    {
        void Log(string message) { if (report != null) report(message); else logger.Log(message); }
        string? temporary = null;
        var owned = false;
        using var deadline = new CancellationTokenSource(host.Timeout);
        try
        {
            Log($"Extracting archive: {archivePath}");
            var sevenZip = Find7zPath(host) ?? throw new FileNotFoundException("7-Zip was not found. Install it or rerun the system checks.");
            WindowsInstallationPaths.EnsureNoReparseAncestors(archivePath, host);
            WindowsInstallationPaths.EnsureNoReparseAncestors(destinationPath, host);
            temporary = Path.Combine(host.TempDirectory, $"weave_extract_{Guid.NewGuid():N}");
            WindowsInstallationPaths.EnsureNoReparseAncestors(temporary, host);
            if (host.DirectoryExists(temporary)) throw new IOException("The temporary extraction directory already exists.");
            host.CreateDirectory(temporary);
            owned = true;
            var request = new ProcessStartInfo(sevenZip)
            {
                UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true, CreateNoWindow = true
            };
            foreach (var argument in new[] { "x", archivePath, "-o" + temporary, "-y" }) request.ArgumentList.Add(argument);
            using (var process = host.StartProcess(request) ?? throw new IOException("7-Zip could not be started."))
            {
                try
                {
                    var output = process.ReadStandardOutputAsync();
                    var error = process.ReadStandardErrorAsync();
                    await process.WaitForExitAsync(deadline.Token).WaitAsync(deadline.Token).ConfigureAwait(false);
                    await Task.WhenAll(output, error).WaitAsync(deadline.Token).ConfigureAwait(false);
                    if (process.ExitCode != 0)
                    {
                        static string Tail(string text) => text.Length > 4096 ? text[^4096..] : text;
                        throw new IOException($"7-Zip extraction failed (exit {process.ExitCode}): {Tail(error.Result)} {Tail(output.Result)}");
                    }
                }
                catch
                {
                    if (!process.HasExited) process.Terminate();
                    throw;
                }
            }
            var payload = Path.Combine(temporary, "dist_staging");
            if (!host.DirectoryExists(payload))
            {
                Log("[WARN] Archive has no dist_staging wrapper; checking its top-level payload.");
                payload = temporary;
            }
            CopyDirectory(payload, destinationPath, logger, host, deadline.Token);
            Log("[OK] MayaFlux extracted successfully");
            return true;
        }
        catch (OperationCanceledException)
        {
            Log($"[ERROR] Extraction timed out after {host.Timeout.TotalMinutes:g} minutes. Select Retry to extract the package again.");
            return false;
        }
        catch (Exception error)
        {
            Log($"[ERROR] Extraction failed: {error.Message}. Correct the reported problem and select Retry.");
            return false;
        }
        finally
        {
            if (owned && temporary != null)
            {
                try
                {
                    WindowsInstallationPaths.EnsureNoReparseAncestors(temporary, host);
                    host.DeleteDirectory(temporary);
                }
                catch (Exception error) { Log($"[WARN] Temporary extraction files remain at {temporary}: {error.Message}"); }
            }
        }
    }

    public static bool VerifyInstallation(string mayaFluxRoot, Logger logger)
        => VerifyInstallation(mayaFluxRoot, logger, null);

    internal static bool VerifyInstallation(string mayaFluxRoot, Logger logger, Action<string>? report)
    {
        void Log(string message) { if (report != null) report(message); else logger.Log(message); }
        try
        {
            var required = new[] { @"lib\MayaFluxLib.lib", @"bin\MayaFluxLib.dll", @"include\MayaFlux\MayaFlux.hpp", @"lib\cmake\MayaFlux\MayaFluxConfig.cmake", @"lib\cmake\MayaFlux\MayaFluxConfigVersion.cmake" };
            var valid = true;
            foreach (var relative in required)
            {
                var path = Path.Combine(mayaFluxRoot, relative);
                if (!File.Exists(path) || new FileInfo(path).Length == 0)
                {
                    Log($"[ERROR] Required MayaFlux file is missing or empty: {path}. Retry the download to reinstall the complete package.");
                    valid = false;
                }
            }
            if (valid) Log("[OK] MayaFlux core runtime, public header and CMake package files verified");
            return valid;
        }
        catch (Exception ex)
        {
            Log($"[ERROR] Verification failed: {ex.Message}");
            return false;
        }
    }

    private static string? Find7zPath(ArchiveExtractionHost host)
    {
        var possiblePaths = new[]
        {
            "C:\\Program Files\\7-Zip\\7z.exe",
            "C:\\Program Files (x86)\\7-Zip\\7z.exe",
            Path.Combine(host.GetEnvironmentVariable("ProgramFiles", EnvironmentVariableTarget.Process) ?? "", "7-Zip", "7z.exe"),
            Path.Combine(host.GetEnvironmentVariable("ProgramFiles(x86)", EnvironmentVariableTarget.Process) ?? "", "7-Zip", "7z.exe")
        };

        return possiblePaths.FirstOrDefault(path => host.FileExists(path));
    }

    private static void CopyDirectory(string source, string destination, Logger logger, ArchiveExtractionHost host, CancellationToken cancellation)
    {
        cancellation.ThrowIfCancellationRequested();
        WindowsInstallationPaths.EnsureNoReparseAncestors(source, host);
        WindowsInstallationPaths.EnsureNoReparseAncestors(destination, host);
        host.CreateDirectory(destination);
        foreach (var entry in host.GetFileSystemEntries(source))
        {
            cancellation.ThrowIfCancellationRequested();
            if (host.IsReparsePoint(entry)) throw new IOException($"Extracted package contains a linked path: {entry}");
            var path = Path.Combine(destination, Path.GetFileName(entry));
            if (host.DirectoryExists(entry)) CopyDirectory(entry, path, logger, host, cancellation);
            else host.CopyFile(entry, path);
        }
    }

    private static string FormatBytes(long bytes)
    {
        string[] sizes = { "B", "KB", "MB", "GB" };
        double len = bytes;
        int order = 0;
        while (len >= 1024 && order < sizes.Length - 1)
        {
            order++;
            len = len / 1024;
        }
        return $"{len:0.##} {sizes[order]}";
    }
}
