#!/usr/bin/env powershell
# Verify Windows build output

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "Verifying Windows Build"
Write-Host ""

$zipPath = "build\windows\Weave.zip"
if (-not (Test-Path $zipPath)) {
    Write-Host "ERROR: ZIP not found at $zipPath"
    exit 1
}

Add-Type -AssemblyName System.IO.Compression
$zip = [System.IO.Compression.ZipFile]::OpenRead($zipPath)

$weaveEntry = $zip.Entries | Where-Object { $_.Name -eq "Weave.exe" }
if ($null -eq $weaveEntry) {
    Write-Host "ERROR: Weave.exe not found in ZIP"
    $zip.Dispose()
    exit 1
}

$sizeMB = [Math]::Round($weaveEntry.Length / 1MB, 1)
Write-Host "INFO: Weave.exe size is $sizeMB MB"

if ($sizeMB -lt 100) {
    Write-Host "ERROR: Executable too small - not self-contained"
    $zip.Dispose()
    exit 1
}

$zip.Dispose()
Write-Host "INFO: ZIP verified successfully"
exit 0