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
    {
        try
        {
            logger.Log($"Extracting archive: {archivePath}");
            
            var sevenZipPath = Find7zPath();
            if (string.IsNullOrEmpty(sevenZipPath))
            {
                logger.Log("[ERROR] 7-Zip not found at expected locations");
                return false;
            }

            Directory.CreateDirectory(destinationPath);

            var tempExtractDir = Path.Combine(Path.GetTempPath(), $"weave_extract_{Guid.NewGuid():N}");
            Directory.CreateDirectory(tempExtractDir);

            var psi = new ProcessStartInfo
            {
                FileName = sevenZipPath,
                Arguments = $"x \"{archivePath}\" -o\"{tempExtractDir}\" -y",
                UseShellExecute = false,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                CreateNoWindow = true
            };

            using (var process = Process.Start(psi))
            {
                process?.WaitForExit();
                
                if (process?.ExitCode != 0)
                {
                    logger.Log($"[ERROR] 7-Zip extraction failed with exit code: {process?.ExitCode}");
                    return false;
                }
            }

            // Move dist_staging contents to destination
            var distStaging = Path.Combine(tempExtractDir, "dist_staging");
            if (Directory.Exists(distStaging))
            {
                CopyDirectory(distStaging, destinationPath, logger);
                logger.Log("[OK] MayaFlux extracted successfully");
            }
            else
            {
                logger.Log("[WARN] dist_staging folder not found, copying all contents");
                CopyDirectory(tempExtractDir, destinationPath, logger);
            }

            // Cleanup
            try { Directory.Delete(tempExtractDir, true); } catch { }

            return true;
        }
        catch (Exception ex)
        {
            logger.Log($"[ERROR] Extraction failed: {ex.Message}");
            return false;
        }
    }

    public static bool VerifyInstallation(string mayaFluxRoot, Logger logger)
    {
        try
        {
            var libPath = Path.Combine(mayaFluxRoot, "lib", "MayaFluxLib.lib");
            if (!File.Exists(libPath))
            {
                logger.Log($"[ERROR] Verification failed - MayaFluxLib.lib not found at {libPath}");
                return false;
            }

            logger.Log("[OK] MayaFluxLib.lib verified");
            return true;
        }
        catch (Exception ex)
        {
            logger.Log($"[ERROR] Verification failed: {ex.Message}");
            return false;
        }
    }

    private static string? Find7zPath()
    {
        var possiblePaths = new[]
        {
            "C:\\Program Files\\7-Zip\\7z.exe",
            "C:\\Program Files (x86)\\7-Zip\\7z.exe",
            Path.Combine(Environment.GetEnvironmentVariable("ProgramFiles") ?? "", "7-Zip", "7z.exe"),
            Path.Combine(Environment.GetEnvironmentVariable("ProgramFiles(x86)") ?? "", "7-Zip", "7z.exe")
        };

        return possiblePaths.FirstOrDefault(path => File.Exists(path));
    }

    private static void CopyDirectory(string source, string destination, Logger logger)
    {
        var dir = new DirectoryInfo(source);
        if (!dir.Exists)
            return;

        Directory.CreateDirectory(destination);

        foreach (var file in dir.GetFiles())
        {
            file.CopyTo(Path.Combine(destination, file.Name), true);
        }

        foreach (var subdir in dir.GetDirectories())
        {
            CopyDirectory(subdir.FullName, Path.Combine(destination, subdir.Name), logger);
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
