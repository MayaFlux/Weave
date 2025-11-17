#!/usr/bin/env powershell
# Build WeaveGUI.cs into a standalone executable using .NET
# Modern approach using .NET SDK instead of .NET Framework

param(
    [string]$OutputDir = "build\windows\WeaveGUI",
    [string]$GuiSourceFile = "windows\gui\WeaveGUI.cs"
)

$ErrorActionPreference = "Stop"

# ============================================================================
# HELPER FUNCTIONS
# ============================================================================

function Test-Command {
    param([string]$Command)
    [bool](Get-Command $Command -ErrorAction SilentlyContinue)
}

Write-Host "=== Building Weave GUI (.NET) ===" -ForegroundColor Cyan
Write-Host ""

# ============================================================================
# CHECK PREREQUISITES
# ============================================================================

Write-Host "Checking prerequisites..." -ForegroundColor Yellow

# Check if dotnet CLI is available
if (-not (Test-Command "dotnet")) {
    Write-Host "[ERROR] .NET SDK not found" -ForegroundColor Red
    Write-Host ""
    Write-Host "CI NOTE: GitHub Actions Windows runners should have .NET SDK pre-installed." -ForegroundColor Yellow
    Write-Host "If running locally, install from: https://dotnet.microsoft.com/download" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Available dotnet versions:" -ForegroundColor Gray
    Get-ChildItem "C:\Program Files\dotnet\sdk" -ErrorAction SilentlyContinue | ForEach-Object { Write-Host "  - $_" }
    exit 1
}

Write-Host "[OK] Found dotnet CLI" -ForegroundColor Green

# Get dotnet version
try {
    $dotnetVersion = dotnet --version
    Write-Host "[OK] .NET version: $dotnetVersion" -ForegroundColor Green
}
catch {
    Write-Host "[WARNING] Could not determine .NET version, but dotnet CLI is available" -ForegroundColor Yellow
}

# Verify source file
if (-not (Test-Path $GuiSourceFile)) {
    Write-Host "[ERROR] GUI source not found: $GuiSourceFile" -ForegroundColor Red
    exit 1
}

Write-Host "[OK] Found source: $GuiSourceFile" -ForegroundColor Green

# ============================================================================
# PREPARE OUTPUT DIRECTORY
# ============================================================================

Write-Host ""
Write-Host "Preparing project structure..." -ForegroundColor Yellow

$projectDir = Join-Path $OutputDir "project"

if (Test-Path $projectDir) {
    Remove-Item $projectDir -Recurse -Force
}
New-Item -ItemType Directory -Path $projectDir -Force | Out-Null

Write-Host "[OK] Project directory: $projectDir" -ForegroundColor Green

# ============================================================================
# CREATE PROJECT FILE (.csproj)
# ============================================================================

Write-Host ""
Write-Host "Creating .NET project file..." -ForegroundColor Yellow

$csprojContent = @"
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>WinExe</OutputType>
    <TargetFramework>net8.0-windows</TargetFramework>
    <UseWindowsForms>true</UseWindowsForms>
    <RuntimeIdentifier>win-x64</RuntimeIdentifier>
    <PublishReadyToRun>true</PublishReadyToRun>
    <PublishSingleFile>true</PublishSingleFile>
    <IncludeNativeLibrariesForSelfExtract>true</IncludeNativeLibrariesForSelfExtract>
    <Nullable>disable</Nullable>
    <Version>1.0.0</Version>
    <AssemblyName>Weave</AssemblyName>
  </PropertyGroup>
</Project>
"@

$csprojPath = Join-Path $projectDir "Weave.csproj"
Set-Content -Path $csprojPath -Value $csprojContent

Write-Host "[OK] Created: Weave.csproj" -ForegroundColor Green

# ============================================================================
# COPY SOURCE FILE
# ============================================================================

Write-Host ""
Write-Host "Copying source file..." -ForegroundColor Yellow

Copy-Item $GuiSourceFile -Destination (Join-Path $projectDir "WeaveGUI.cs") -Force

Write-Host "[OK] Copied source to project" -ForegroundColor Green

# ============================================================================
# BUILD PROJECT
# ============================================================================

Write-Host ""
Write-Host "Building .NET project..." -ForegroundColor Yellow
Write-Host ""

Push-Location $projectDir

try {
    # Restore NuGet packages first
    Write-Host "Restoring NuGet packages..." -ForegroundColor Yellow
    dotnet restore

    if ($LASTEXITCODE -ne 0) {
        Write-Host ""
        Write-Host "[ERROR] Restore failed" -ForegroundColor Red
        exit 1
    }

    Write-Host ""
    Write-Host "Building .NET project..." -ForegroundColor Yellow
    Write-Host ""

    # Build for release
    dotnet build -c Release --no-restore -p:DebugType=none -p:DebugSymbols=false

    if ($LASTEXITCODE -ne 0) {
        Write-Host ""
        Write-Host "[ERROR] Build failed" -ForegroundColor Red
        exit 1
    }

    Write-Host ""
    Write-Host "Publishing as single executable..." -ForegroundColor Yellow
    Write-Host ""

    # Publish as single executable
    dotnet publish -c Release -r win-x64 -p:PublishSingleFile=true -p:IncludeNativeLibrariesForSelfExtract=true --self-contained

    if ($LASTEXITCODE -ne 0) {
        Write-Host ""
        Write-Host "[ERROR] Publish failed" -ForegroundColor Red
        exit 1
    }

    Write-Host ""
    Write-Host "[OK] Publish succeeded" -ForegroundColor Green

}
finally {
    Pop-Location
}

# ============================================================================
# COPY OUTPUT
# ============================================================================

Write-Host ""
Write-Host "Finalizing output..." -ForegroundColor Yellow

$publishDir = Join-Path $projectDir "bin\Release\net8.0-windows\win-x64\publish"
$outputExe = Join-Path $OutputDir "Weave.exe"

if (-not (Test-Path $publishDir)) {
    Write-Host "[ERROR] Publish directory not found: $publishDir" -ForegroundColor Red
    exit 1
}

$publishedExe = Join-Path $publishDir "Weave.exe"

if (-not (Test-Path $publishedExe)) {
    Write-Host "[ERROR] Executable not found: $publishedExe" -ForegroundColor Red
    Write-Host ""
    Write-Host "Publish directory contents:" -ForegroundColor Yellow
    Get-ChildItem $publishDir -Recurse
    exit 1
}

# Copy executable to output directory
Copy-Item $publishedExe -Destination $outputExe -Force

Write-Host "[OK] Executable copied: $outputExe" -ForegroundColor Green

# ============================================================================
# VERIFY OUTPUT
# ============================================================================

Write-Host ""
Write-Host "Verifying build output..." -ForegroundColor Yellow

if (-not (Test-Path $outputExe)) {
    Write-Host "[ERROR] Output executable not found at: $outputExe" -ForegroundColor Red
    exit 1
}

$fileSize = (Get-Item $outputExe).Length / 1MB
Write-Host "[OK] Executable verified: $outputExe" -ForegroundColor Green
Write-Host "     Size: $([Math]::Round($fileSize, 2)) MB" -ForegroundColor Green

# ============================================================================
# CLEANUP
# ============================================================================

Write-Host ""
Write-Host "Cleaning up build artifacts..." -ForegroundColor Yellow

# Keep only the final executable
Remove-Item (Join-Path $projectDir "bin") -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $projectDir "obj") -Recurse -Force -ErrorAction SilentlyContinue

Write-Host "[OK] Cleanup complete" -ForegroundColor Green

# ============================================================================
# SUCCESS
# ============================================================================

Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "  Weave GUI built successfully!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Output: $outputExe" -ForegroundColor Green
Write-Host "Size: $([Math]::Round($fileSize, 2)) MB" -ForegroundColor Green
Write-Host ""

exit 0