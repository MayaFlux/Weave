#!/usr/bin/env powershell
# Verify Windows source files before build

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Verifying Windows Source Files" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$allPresent = $true

$requiredFiles = @(
    @{ Path = "windows\Weave.sln"; Name = "Weave Solution" },
    @{ Path = "windows\Weave\Weave.csproj"; Name = "Weave Project" },
    @{ Path = "windows\Shared\Shared.csproj"; Name = "Shared Project" },
    @{ Path = "windows\scripts\install_package.ps1"; Name = "Install Package Script" },
    @{ Path = "windows\scripts\packages.psd1"; Name = "Packages Definition" },
    @{ Path = "templates"; Name = "Project Templates (embedded in exe)" }
)

Write-Host "Checking required files..." -ForegroundColor Yellow
echo ""

foreach ($file in $requiredFiles) {
    $path = $file.Path
    $name = $file.Name
    
    if (Test-Path $path) {
        if ((Get-Item $path) -is [System.IO.DirectoryInfo]) {
            $itemCount = @(Get-ChildItem $path -Recurse -ErrorAction SilentlyContinue).Count
            Write-Host "[OK] $name (directory, $itemCount items)" -ForegroundColor Green
        } else {
            $fileSize = (Get-Item $path).Length
            $fileSizeKB = [Math]::Round($fileSize / 1KB, 1)
            Write-Host "[OK] $name ($fileSizeKB KB)" -ForegroundColor Green
        }
    } else {
        Write-Host "[ERROR] Missing: $name at $path" -ForegroundColor Red
        $allPresent = $false
    }
}

echo ""

if ($allPresent) {
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "  All Required Files Present" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    echo ""
} else {
    Write-Host "[ERROR] Some required files are missing" -ForegroundColor Red
    exit 1
}