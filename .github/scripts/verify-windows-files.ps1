#!/usr/bin/env powershell
# Verify Windows source files before build

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "Verifying Windows Source Files"
Write-Host ""

$allPresent = $true

Write-Host "Checking project files..."
if (Test-Path "windows\Weave.sln") {
    Write-Host "[OK] Weave Solution"
} else {
    Write-Host "[ERROR] Weave.sln missing"
    $allPresent = $false
}

if (Test-Path "windows\Weave\Weave.csproj") {
    Write-Host "[OK] Weave Project"
} else {
    Write-Host "[ERROR] Weave.csproj missing"
    $allPresent = $false
}

if (Test-Path "windows\Shared\Shared.csproj") {
    Write-Host "[OK] Shared Library"
} else {
    Write-Host "[ERROR] Shared.csproj missing"
    $allPresent = $false
}

Write-Host ""
Write-Host "Checking templates..."
$templates = @(
    "templates\CMakeLists.txt",
    "templates\shaders.cmake",
    "templates\main.cpp",
    "templates\user_project.hpp",
    "templates\vscode\settings.json",
    "templates\vscode\tasks.json",
    "templates\vscode\launch.json"
)
foreach ($t in $templates) {
    if (Test-Path $t) {
        Write-Host "[OK] $t"
    } else {
        Write-Host "[ERROR] $t missing"
        $allPresent = $false
    }
}

Write-Host ""
Write-Host "Checking scripts..."
$scripts = @(
    "windows\scripts\install_package.ps1",
    "windows\scripts\packages.psd1"
)
foreach ($s in $scripts) {
    if (Test-Path $s) {
        Write-Host "[OK] $s"
    } else {
        Write-Host "[ERROR] $s missing"
        $allPresent = $false
    }
}

Write-Host ""
if ($allPresent) {
    Write-Host "All files present - ready for build"
    exit 0
} else {
    Write-Host "Missing files - build cannot proceed"
    exit 1
}
