param([Parameter(Mandatory)][string]$Package, [switch]$SkipGui)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (!$IsWindows) { throw 'Package validation requires Windows and PowerShell 7.' }
. (Join-Path $PSScriptRoot 'pe.ps1')
$Package = (Resolve-Path -LiteralPath $Package).Path
$work = Join-Path ([IO.Path]::GetTempPath()) "weave-package-check-$([Guid]::NewGuid().ToString('N'))"
$unpacked = Join-Path $work 'portable package'
$gui = $null

function New-PackagedProcess([string[]]$Arguments) {
    $start = [Diagnostics.ProcessStartInfo]::new($executable)
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.WorkingDirectory = $work
    foreach ($argument in $Arguments) { $start.ArgumentList.Add($argument) }
    foreach ($name in @($start.Environment.Keys | Where-Object { $_ -match '^(CARGO|RUST|SLINT|QT_|MAYAFLUX)' })) { $start.Environment.Remove($name) | Out-Null }
    # Run outside the checkout without Git, Cargo, compilers or installed tools
    # on PATH. Do not edit the machine/user environment or touch the registry.
    $start.Environment['PATH'] = "$env:SystemRoot\System32;$env:SystemRoot"
    $start.Environment['USERPROFILE'] = Join-Path $work 'profile'
    $start.Environment['HOME'] = Join-Path $work 'profile'
    $start.Environment['LOCALAPPDATA'] = Join-Path $work 'profile/Local'
    $start.Environment['APPDATA'] = Join-Path $work 'profile/Roaming'
    [Diagnostics.Process]::Start($start)
}

function Invoke-PackagedCli([string[]]$Arguments, [bool]$Success = $true) {
    $process = New-PackagedProcess $Arguments
    try {
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        if (!$process.WaitForExit(30000)) { $process.Kill($true); throw "Packaged CLI timed out: $Arguments" }
        $text = $stdout.GetAwaiter().GetResult() + $stderr.GetAwaiter().GetResult()
        if (($process.ExitCode -eq 0) -ne $Success) { throw "Packaged CLI returned $($process.ExitCode): $Arguments`n$text" }
        return $text
    } finally { $process.Dispose() }
}

try {
    New-Item -ItemType Directory -Path $unpacked, (Join-Path $work 'profile/Local'), (Join-Path $work 'profile/Roaming') -Force | Out-Null
    $zip = [IO.Compression.ZipFile]::OpenRead($Package)
    try {
        $expected = @('LICENSE.txt','README.txt','Weave.exe','package.json') | Sort-Object
        $actual = @($zip.Entries.FullName | Sort-Object)
        if (($actual -join '|') -cne ($expected -join '|')) { throw 'Unexpected or missing archive entries; refusing to extract.' }
        if (($zip.Entries | Measure-Object Length -Sum).Sum -gt 512MB) { throw 'The archive exceeds the package validation size limit.' }
    } finally { $zip.Dispose() }
    [IO.Compression.ZipFile]::ExtractToDirectory($Package, $unpacked)
    $executable = Join-Path $unpacked 'Weave.exe'
    Assert-WeavePe $executable
    $metadata = Get-Content -Raw -LiteralPath (Join-Path $unpacked 'package.json') | ConvertFrom-Json
    if ($metadata.architecture -ne 'x64' -or $metadata.minimumWindows -ne '10.0.19045' -or $metadata.runtime -ne 'static-msvc' -or $metadata.resources -ne 'embedded') { throw 'The package support contract is incorrect.' }
    if ((Get-Item -LiteralPath (Join-Path $unpacked 'LICENSE.txt')).Length -eq 0) { throw 'Packaged license is empty.' }

    $help = Invoke-PackagedCli @('--help')
    foreach ($operation in @('weave new','weave community','weave update','weave install')) {
        if (!$help.Contains($operation)) { throw "Packaged CLI help is missing $operation." }
    }
    $projects = Join-Path $work 'generated projects'
    New-Item -ItemType Directory -Path $projects | Out-Null
    Invoke-PackagedCli @('new','PackageSmoke',$projects,'--with-lila') | Write-Host
    $project = Join-Path $projects 'PackageSmoke'
    foreach ($resource in @('CMakeLists.txt','CMakePresets.json','src/main.cpp','src/user_project.hpp',
        'cmake/mayaflux.cmake','cmake/shaders.cmake','cmake/build_community.cmake',
        '.vscode/settings.json','.vscode/tasks.json','.vscode/launch.json',
        'validate.ps1','run-report.ps1','validate.bat','run-report.bat')) {
        if (!(Test-Path -LiteralPath (Join-Path $project $resource) -PathType Leaf)) { throw "Embedded project resource is missing: $resource" }
    }
    $presets = Get-Content -Raw -LiteralPath (Join-Path $project 'CMakePresets.json') | ConvertFrom-Json
    if (!$presets.configurePresets) { throw 'Generated presets are invalid.' }
    $modules = Join-Path $work 'generated modules'
    New-Item -ItemType Directory -Path $modules | Out-Null
    Invoke-PackagedCli @('community','package_smoke',$modules,'--description','Packaged resource check') | Write-Host
    foreach ($resource in @('community.json','CMakeLists.txt','package_smoke.cmake','CMakePresets.json','test/CMakeLists.txt')) {
        if (!(Test-Path -LiteralPath (Join-Path $modules "package_smoke/$resource") -PathType Leaf)) { throw "Embedded community resource is missing: $resource" }
    }
    Get-Content -Raw -LiteralPath (Join-Path $modules 'package_smoke/community.json') | ConvertFrom-Json | Out-Null
    $before = (Get-FileHash -LiteralPath (Join-Path $project 'CMakeLists.txt')).Hash
    Invoke-PackagedCli @('new','PackageSmoke',$projects) $false | Out-Null
    Invoke-PackagedCli @('new','../escape',$projects) $false | Out-Null
    Invoke-PackagedCli @('update',$project,'../unsafe') $false | Out-Null
    Invoke-PackagedCli @('install','nightly') $false | Out-Null
    if ((Test-Path -LiteralPath (Join-Path $work 'escape')) -or (Get-FileHash -LiteralPath (Join-Path $project 'CMakeLists.txt')).Hash -ne $before) { throw 'Invalid operations modified the package fixture.' }

    if (!$SkipGui) {
        if (!('WeavePackageWindow' -as [type])) {
            Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class WeavePackageWindow {
    private delegate bool Visitor(IntPtr window, IntPtr data);
    [StructLayout(LayoutKind.Sequential)] private struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] private static extern bool EnumWindows(Visitor visitor, IntPtr data);
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr window, out uint pid);
    [DllImport("user32.dll")] private static extern bool GetWindowRect(IntPtr window, out Rect rect);
    public static bool Exists(int process) {
        bool found = false;
        EnumWindows((window, data) => {
            uint pid; Rect rect; GetWindowThreadProcessId(window, out pid);
            if (pid == process && GetWindowRect(window, out rect) && rect.Right - rect.Left >= 500 && rect.Bottom - rect.Top >= 400) found = true;
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
'@
        }
        $gui = New-PackagedProcess @()
        $guiOutput = $gui.StandardOutput.ReadToEndAsync()
        $guiErrors = $gui.StandardError.ReadToEndAsync()
        $opened = $false
        for ($attempt = 0; $attempt -lt 40; $attempt++) {
            Start-Sleep -Milliseconds 200
            if ($gui.HasExited) { throw "Packaged GUI exited during startup: $($guiErrors.GetAwaiter().GetResult())" }
            if ([WeavePackageWindow]::Exists($gui.Id)) { $opened = $true; break }
        }
        if (!$opened) { throw 'The packaged GUI did not create its main window.' }
        Start-Sleep -Seconds 2
        if ($gui.HasExited) { throw "The packaged GUI failed after opening: $($guiErrors.GetAwaiter().GetResult())" }
        $gui.Kill($true)
        $gui.WaitForExit()
        Write-Host 'Packaged GUI launched with its normal GPU backend.'
    }
    Write-Host "Windows portable package validation passed: $Package"
} finally {
    if ($null -ne $gui) { if (!$gui.HasExited) { $gui.Kill($true); $gui.WaitForExit() }; $gui.Dispose() }
    $resolvedWork = [IO.Path]::GetFullPath($work)
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
    if ([IO.Path]::GetDirectoryName($resolvedWork) -ne $tempRoot -or ![IO.Path]::GetFileName($resolvedWork).StartsWith('weave-package-check-')) { throw 'Invalid package validation cleanup path.' }
    if (Test-Path -LiteralPath $resolvedWork) { Remove-Item -LiteralPath $resolvedWork -Recurse -Force }
}
