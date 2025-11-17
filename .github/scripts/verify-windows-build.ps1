#!/usr/bin/env powershell
# Verify Windows build artifacts

param(
    [Parameter(Mandatory = $true)]
    [string]$Version
)

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Verifying Windows Build" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$buildDir = "build\windows"
$installerPath = "$buildDir\Weave-$Version.exe"

# Check if installer exists
Write-Host "Checking installer..." -ForegroundColor Yellow
if (Test-Path $installerPath) {
    $fileSize = (Get-Item $installerPath).Length
    $fileSizeMB = [Math]::Round($fileSize / 1MB, 2)
    Write-Host "[OK] Installer found" -ForegroundColor Green
    Write-Host "     Path: $installerPath" -ForegroundColor Green
    Write-Host "     Size: $fileSizeMB MB" -ForegroundColor Green
} else {
    Write-Host "[ERROR] Installer not found at: $installerPath" -ForegroundColor Red
    exit 1
}

# Check that scripts were included in the build output
Write-Host ""
Write-Host "Checking bundled resources..." -ForegroundColor Yellow

$scriptFiles = @(
    "install_package.ps1",
    "packages.psd1"
)

$allScriptsPresent = $true
foreach ($script in $scriptFiles) {
    $scriptPath = Join-Path $buildDir $script
    if (Test-Path $scriptPath) {
        $fileSize = (Get-Item $scriptPath).Length / 1KB
        Write-Host "[OK] $script ($('{0:F1}' -f $fileSize) KB)" -ForegroundColor Green
    } else {
        Write-Host "[ERROR] $script not found in build output" -ForegroundColor Red
        $allScriptsPresent = $false
    }
}

if (-not $allScriptsPresent) {
    Write-Host ""
    Write-Host "[ERROR] Some required scripts are missing from build output" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Build Verification Complete" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "✅ Installer ready for distribution:" -ForegroundColor Green
Write-Host "   $installerPath" -ForegroundColor Green
Write-Host ""