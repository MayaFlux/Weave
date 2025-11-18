#!/usr/bin/env powershell

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Verifying Windows Build" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$buildDir = "build\windows"
$zipPath = "$buildDir\Weave.zip"

# Check if ZIP exists
Write-Host "Checking installer..." -ForegroundColor Yellow
if (Test-Path $zipPath) {
    $fileSize = (Get-Item $zipPath).Length
    $fileSizeMB = [Math]::Round($fileSize / 1MB, 2)
    Write-Host "OK: Installer found" -ForegroundColor Green
    Write-Host "    Path: $zipPath" -ForegroundColor Green
    Write-Host "    Size: $fileSizeMB MB" -ForegroundColor Green
} else {
    Write-Host "Error: Installer not found at: $zipPath" -ForegroundColor Red
    exit 1
}

# Check that scripts were included in the ZIP
Write-Host ""
Write-Host "Checking bundled resources..." -ForegroundColor Yellow

$requiredFiles = @(
    "install_package.ps1",
    "packages.psd1",
    "Weave.exe"
)

$stagingDir = "$buildDir\staging"
$allPresent = $true

foreach ($file in $requiredFiles) {
    $filePath = "$stagingDir\$file"
    if (Test-Path $filePath) {
        $fileSize = (Get-Item $filePath).Length / 1KB
        Write-Host "OK: $file ($('{0:F1}' -f $fileSize) KB)" -ForegroundColor Green
    } else {
        Write-Host "Error: $file not found in staging directory" -ForegroundColor Red
        $allPresent = $false
    }
}

if (-not $allPresent) {
    Write-Host ""
    Write-Host "Error: Some required files are missing" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Build Verification Complete" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "OK: Installer ready for distribution:" -ForegroundColor Green
Write-Host "   $zipPath" -ForegroundColor Green
Write-Host ""