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

Write-Host "[OK] ZIP found: $zipPath"
$zipSizeMB = [Math]::Round((Get-Item $zipPath).Length / 1MB, 1)
Write-Host "[OK] Size: $zipSizeMB MB"

Add-Type -AssemblyName System.IO.Compression
$zip = [System.IO.Compression.ZipFile]::OpenRead($zipPath)

# Check for flat structure (Weave.exe at root, not in subdirectory)
$weaveEntry = $zip.Entries | Where-Object { $_.Name -eq "Weave.exe" -and $_.FullName -eq "Weave.exe" }
if ($null -eq $weaveEntry) {
    Write-Host "ERROR: Weave.exe not found at ZIP root"
    Write-Host "Found entries:"
    $zip.Entries | Where-Object { $_.Name -like "*Weave*" } | ForEach-Object { Write-Host "  $($_.FullName)" }
    $zip.Dispose()
    exit 1
}

Write-Host "[OK] Weave.exe at root (flat structure correct)"
$sizeMB = [Math]::Round($weaveEntry.Length / 1MB, 1)
Write-Host "[OK] Weave.exe size: $sizeMB MB"

if ($sizeMB -lt 100) {
    Write-Host "ERROR: Executable too small - not self-contained"
    $zip.Dispose()
    exit 1
}

$requiredFiles = @("install_package.ps1", "packages.psd1")
foreach ($file in $requiredFiles) {
    $entry = $zip.Entries | Where-Object { $_.Name -eq $file -and $_.FullName -eq $file }
    if ($null -eq $entry) {
        Write-Host "WARNING: $file not found at root"
    } else {
        Write-Host "[OK] $file present"
    }
}

$templatesEntries = $zip.Entries | Where-Object { $_.FullName -like "templates/*" }
if ($templatesEntries.Count -eq 0) {
    Write-Host "WARNING: templates directory not found in ZIP"
} else {
    Write-Host "[OK] templates directory found ($($templatesEntries.Count) files)"
}

$zip.Dispose()
Write-Host ""
Write-Host "[OK] ZIP verification passed - structure is correct"
exit 0