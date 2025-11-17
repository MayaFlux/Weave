#Requires -RunAsAdministrator

param(
    [Parameter(Mandatory = $false)]
    [string]$Version = "0.1.0",
    
    [Parameter(Mandatory = $false)]
    [string]$OutputDir = "build/windows"
)

$ErrorActionPreference = "Stop"

# ============================================================================
# CONFIGURATION
# ============================================================================

$SCRIPT_DIR = $PSScriptRoot
$PROJECT_ROOT = Split-Path (Split-Path $SCRIPT_DIR -Parent) -Parent
$BUILD_DIR = Join-Path $PROJECT_ROOT $OutputDir
$WINDOWS_DIR = Join-Path $PROJECT_ROOT "windows"
$TEMPLATES_DIR = Join-Path $PROJECT_ROOT "templates"

$NSI_FILE = Join-Path $WINDOWS_DIR "Weave.nsi"
$MAKENSIS = "C:\Program Files (x86)\NSIS\makensis.exe"

# ============================================================================
# UTILITIES
# ============================================================================

function Write-Header {
    param([string]$Message)
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "  $Message" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
}

function Test-CommandExists {
    param([string]$Command)
    [bool](Get-Command $Command -ErrorAction SilentlyContinue)
}

function Test-NsisInstalled {
    if (Test-Path $MAKENSIS) {
        return $true
    }
    
    # Check alternative paths
    $altPaths = @(
        "C:\Program Files\NSIS\makensis.exe",
        "C:\Program Files (x86)\NSIS\makensis.exe"
    )
    
    foreach ($path in $altPaths) {
        if (Test-Path $path) {
            $MAKENSIS = $path
            return $true
        }
    }
    
    return $false
}

# ============================================================================
# MAIN BUILD PROCESS
# ============================================================================

Write-Header "Weave Windows Installer Builder v$Version"

# Step 1: Verify prerequisites
Write-Host "Checking prerequisites..." -ForegroundColor Yellow

if (-not (Test-NsisInstalled)) {
    Write-Host "[ERROR] NSIS not found. Please install NSIS from: https://nsis.sourceforge.io/" -ForegroundColor Red
    Write-Host ""
    Write-Host "Quick install (if you have Chocolatey):" -ForegroundColor Yellow
    Write-Host "  choco install nsis" -ForegroundColor Gray
    exit 1
}

Write-Host "[OK] NSIS found: $MAKENSIS" -ForegroundColor Green

# Step 2: Verify source files
Write-Host "`nVerifying source files..." -ForegroundColor Yellow

$requiredFiles = @(
    @{ Path = $NSI_FILE; Name = "Weave.nsi" },
    @{ Path = (Join-Path $WINDOWS_DIR "scripts\install_package.ps1"); Name = "install_package.ps1" },
    @{ Path = (Join-Path $WINDOWS_DIR "scripts\packages.psd1"); Name = "packages.psd1" },
    @{ Path = $TEMPLATES_DIR; Name = "templates directory" }
)

$allFilesPresent = $true
foreach ($file in $requiredFiles) {
    if (Test-Path $file.Path) {
        Write-Host "[OK] Found: $($file.Name)" -ForegroundColor Green
    }
    else {
        Write-Host "[ERROR] Missing: $($file.Name) at $($file.Path)" -ForegroundColor Red
        $allFilesPresent = $false
    }
}

if (-not $allFilesPresent) {
    Write-Host "`n[ERROR] Missing required files" -ForegroundColor Red
    exit 1
}

# Step 3: Prepare build directory
Write-Host "`nPreparing build directory..." -ForegroundColor Yellow

if (-not (Test-Path $BUILD_DIR)) {
    New-Item -ItemType Directory -Path $BUILD_DIR -Force | Out-Null
    Write-Host "[OK] Created: $BUILD_DIR" -ForegroundColor Green
}
else {
    Write-Host "[OK] Using: $BUILD_DIR" -ForegroundColor Green
}

# Step 4: Copy installer files to build directory
Write-Host "`nCopying installer files to build directory..." -ForegroundColor Yellow

# Copy NSI
Copy-Item $NSI_FILE -Destination (Join-Path $BUILD_DIR "Weave.nsi") -Force
Write-Host "[OK] Copied Weave.nsi" -ForegroundColor Green

# Copy scripts directory
$scriptsSource = Join-Path $WINDOWS_DIR "scripts"
$scriptsDest = Join-Path $BUILD_DIR "scripts"

if (Test-Path $scriptsDest) {
    Remove-Item $scriptsDest -Recurse -Force
}

Copy-Item $scriptsSource -Destination $scriptsDest -Recurse -Force
Write-Host "[OK] Copied scripts directory" -ForegroundColor Green

# Copy templates
$templatesDest = Join-Path $BUILD_DIR "templates"

if (Test-Path $templatesDest) {
    Remove-Item $templatesDest -Recurse -Force
}

Copy-Item $TEMPLATES_DIR -Destination $templatesDest -Recurse -Force
Write-Host "[OK] Copied templates directory to: $templatesDest" -ForegroundColor Green

# Copy resources (HTML files, etc.)
$resourcesSource = Join-Path $WINDOWS_DIR "resources"
if (Test-Path $resourcesSource) {
    $resourcesDest = Join-Path $BUILD_DIR "resources"
    if (Test-Path $resourcesDest) {
        Remove-Item $resourcesDest -Recurse -Force
    }
    Copy-Item $resourcesSource -Destination $resourcesDest -Recurse -Force
    Write-Host "[OK] Copied resources directory" -ForegroundColor Green
}

# Step 5: Build the installer
Write-Host "`nBuilding installer..." -ForegroundColor Yellow

$nsiArgs = @(
    "/V4"
    "/O$BUILD_DIR"
    "/D`"VERSION=$Version`""
    (Join-Path $BUILD_DIR "Weave.nsi")
)

Write-Host "Running NSIS compiler..." -ForegroundColor Gray

& $MAKENSIS $nsiArgs
$exitCode = $LASTEXITCODE

if ($exitCode -ne 0) {
    Write-Host "`n[ERROR] NSIS compilation failed with exit code: $exitCode" -ForegroundColor Red
    exit 1
}

# Step 6: Verify output
Write-Host "`nVerifying build output..." -ForegroundColor Yellow

$outputExe = Join-Path $BUILD_DIR "Weave.exe"

if (Test-Path $outputExe) {
    $fileSize = (Get-Item $outputExe).Length / 1MB
    Write-Host "[OK] Installer created: $outputExe" -ForegroundColor Green
    Write-Host "     Size: $([Math]::Round($fileSize, 2)) MB" -ForegroundColor Green
}
else {
    Write-Host "[ERROR] Installer not created at expected path: $outputExe" -ForegroundColor Red
    exit 1
}

# Step 7: Summary
Write-Host "`n"
Write-Header "Build Complete!"

Write-Host "Installer: $outputExe" -ForegroundColor Green
Write-Host "Version: $Version" -ForegroundColor Green
Write-Host ""
Write-Host "Test installation:" -ForegroundColor Yellow
Write-Host "  $outputExe" -ForegroundColor Gray
Write-Host ""
Write-Host "Distribution options:" -ForegroundColor Yellow
Write-Host "  - Upload to GitHub Releases" -ForegroundColor Gray
Write-Host "  - Host on website" -ForegroundColor Gray
Write-Host "  - Share directly (includes all dependencies)" -ForegroundColor Gray
Write-Host ""

# Step 8: Calculate file hash (optional, useful for distribution)
Write-Host "Generating SHA256 hash for distribution..." -ForegroundColor Yellow
$hash = (Get-FileHash $outputExe -Algorithm SHA256).Hash
Write-Host "SHA256: $hash" -ForegroundColor Green
Write-Host ""

Write-Host "[OK] Ready for distribution!" -ForegroundColor Green
