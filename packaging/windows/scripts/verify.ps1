param([Parameter(Mandatory)][string]$Package, [switch]$SkipGui, [switch]$AllowMissingGpu)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (!$IsWindows) { throw 'Package validation requires Windows and PowerShell 7.' }
. (Join-Path $PSScriptRoot 'pe.ps1')
. (Join-Path $PSScriptRoot 'gui.ps1')
if ($SkipGui -and $AllowMissingGpu) { throw 'Choose either SkipGui or AllowMissingGpu.' }
$Package = (Resolve-Path -LiteralPath $Package).Path
$work = Join-Path ([IO.Path]::GetTempPath()) "weave-package-check-$([Guid]::NewGuid().ToString('N'))"
$unpacked = Join-Path $work 'standalone executable'
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
    if ($env:WEAVE_TEST_MAYAFLUX_ROOT) { $start.Environment['MAYAFLUX_ROOT'] = $env:WEAVE_TEST_MAYAFLUX_ROOT }
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
    # Copy only the downloadable executable into an otherwise empty directory.
    # No source resources or sidecar metadata are available to the child.
    $executable = Join-Path $unpacked 'Weave.exe'
    Copy-Item -LiteralPath $Package -Destination $executable
    Assert-WeavePe $executable
    $metadata = Invoke-PackagedCli @('--package-info') | ConvertFrom-Json
    if ($metadata.architecture -ne 'x64' -or $metadata.minimumWindows -ne '10.0.19045' -or $metadata.runtime -ne 'static-msvc' -or $metadata.resources -ne 'embedded') { throw 'The executable support contract is incorrect.' }
    $license = Invoke-PackagedCli @('--license')
    if (!$license.Contains('GNU GENERAL PUBLIC LICENSE') -or !$license.Contains('Version 3, 29 June 2007')) { throw 'The embedded license is missing or incorrect.' }
    $help = Invoke-PackagedCli @('--help')
    foreach ($operation in @('weave new','weave manage','weave community','weave install')) {
        if (!$help.Contains($operation)) { throw "Packaged CLI help is missing $operation." }
    }
    if ($help.Contains('weave update')) { throw 'Packaged CLI still advertises the retired update command.' }
    $projects = Join-Path $work 'generated projects'
    New-Item -ItemType Directory -Path $projects | Out-Null
    Invoke-PackagedCli @('new','PackageSmoke',$projects) | Write-Host
    $project = Join-Path $projects 'PackageSmoke'
    Invoke-PackagedCli @('manage',$project,'--with-lila') | Write-Host
    $enabledCmake = Get-Content -Raw -LiteralPath (Join-Path $project 'cmake/mayaflux.cmake')
    if (!$enabledCmake.Contains('MayaFlux::MayaFluxHost') -or !$enabledCmake.Contains('MayaFluxHost.dll')) { throw 'Packaged Manage did not enable Lila integration.' }
    Invoke-PackagedCli @('manage',$project,'--with-lila') | Out-Null
    if ((Get-Content -Raw -LiteralPath (Join-Path $project 'cmake/mayaflux.cmake')) -cne $enabledCmake) { throw 'Repeated packaged Manage changed an already enabled project.' }
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

    # Junction ancestors must be rejected before any operation writes through them.
    $junctionTarget = Join-Path $work 'junction target'
    $junctionParent = Join-Path $work 'junction parent'
    New-Item -ItemType Directory -Path (Join-Path $junctionTarget 'project output') -Force | Out-Null
    New-Item -ItemType Junction -Path $junctionParent -Target $junctionTarget | Out-Null

    $linkedProjectOutput = Invoke-PackagedCli @('new','LinkedAncestorRejected',(Join-Path $junctionParent 'project output')) $false
    if ($linkedProjectOutput -notmatch 'reparse point') { throw "Project creation did not report the linked ancestor: $linkedProjectOutput" }
    if (Test-Path -LiteralPath (Join-Path $junctionTarget 'project output/LinkedAncestorRejected')) { throw 'Project creation wrote through a junction ancestor.' }

    $linkedCommunityDestination = Join-Path $junctionParent 'community output'
    $linkedCommunityOutput = Invoke-PackagedCli @('community','linked_module',$linkedCommunityDestination) $false
    if ($linkedCommunityOutput -notmatch 'reparse point') { throw "Community creation did not report the linked ancestor: $linkedCommunityOutput" }
    if (Test-Path -LiteralPath (Join-Path $junctionTarget 'community output')) { throw 'Community creation created its destination through a junction ancestor.' }

    $mayaFluxTarget = Join-Path $junctionTarget 'mayaflux'
    $shaderRoot = Join-Path $mayaFluxTarget 'share/MayaFlux/src/shaders'
    New-Item -ItemType Directory -Path $shaderRoot -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $shaderRoot 'test.frag') -Value 'shader fixture' -NoNewline
    $env:WEAVE_TEST_MAYAFLUX_ROOT = Join-Path $junctionParent 'mayaflux'
    try {
        $linkedShaderOutput = Invoke-PackagedCli @('new','ShaderAncestorRejected',$projects,'--copy-shader-sources') $false
    } finally {
        Remove-Item Env:WEAVE_TEST_MAYAFLUX_ROOT -ErrorAction SilentlyContinue
    }
    if ($linkedShaderOutput -notmatch 'reparse point') { throw "Shader copying did not report the linked ancestor: $linkedShaderOutput" }
    if (Test-Path -LiteralPath (Join-Path $projects 'ShaderAncestorRejected')) { throw 'Project creation copied shaders through a junction ancestor.' }

    $plainMayaFluxRoot = Join-Path $work 'plain mayaflux root'
    $nestedShaderRoot = Join-Path $plainMayaFluxRoot 'share/MayaFlux/src/shaders'
    $externalShaders = Join-Path $work 'external shaders'
    New-Item -ItemType Directory -Path $nestedShaderRoot, $externalShaders -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $externalShaders 'linked.frag') -Value 'linked shader fixture' -NoNewline
    New-Item -ItemType Junction -Path (Join-Path $nestedShaderRoot 'linked') -Target $externalShaders | Out-Null
    $env:WEAVE_TEST_MAYAFLUX_ROOT = $plainMayaFluxRoot
    try {
        $linkedShaderDescendantOutput = Invoke-PackagedCli @('new','ShaderDescendantRejected',$projects,'--copy-shader-sources') $false
    } finally {
        Remove-Item Env:WEAVE_TEST_MAYAFLUX_ROOT -ErrorAction SilentlyContinue
    }
    if ($linkedShaderDescendantOutput -notmatch 'reparse point') { throw "Shader copying did not report the linked descendant: $linkedShaderDescendantOutput" }
    if (Test-Path -LiteralPath (Join-Path $projects 'ShaderDescendantRejected')) { throw 'Project creation copied shaders through a junction descendant.' }

    $projectAlias = Join-Path $work 'project alias'
    New-Item -ItemType Junction -Path $projectAlias -Target $projects | Out-Null
    $linkedManageOutput = Invoke-PackagedCli @('manage',(Join-Path $projectAlias 'PackageSmoke'),'package_smoke') $false
    if ($linkedManageOutput -notmatch 'reparse point') { throw "Project management did not report the linked ancestor: $linkedManageOutput" }

    $before = (Get-FileHash -LiteralPath (Join-Path $project 'CMakeLists.txt')).Hash
    Invoke-PackagedCli @('new','PackageSmoke',$projects) $false | Out-Null
    Invoke-PackagedCli @('new','../escape',$projects) $false | Out-Null
    Invoke-PackagedCli @('manage',$project,'../unsafe') $false | Out-Null
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
            if ($gui.HasExited) {
                $outcome = Get-WeaveGuiExitOutcome -ExitCode $gui.ExitCode -ErrorText ($guiErrors.GetAwaiter().GetResult()) -AllowMissingGpu:$AllowMissingGpu
                if ($outcome -eq 'Unavailable') {
                    $notice = 'Windows package CLI, resources, architecture and runtime checks passed. GUI startup coverage is unavailable because the runner has no GPU adapter. Validate GUI startup on a Windows host with a supported GPU.'
                    Write-Warning $notice
                    if ($env:GITHUB_ACTIONS -eq 'true') {
                        Write-Host "::warning::$notice"
                        if ($env:GITHUB_STEP_SUMMARY) { Add-Content -LiteralPath $env:GITHUB_STEP_SUMMARY -Value $notice }
                    }
                    return
                }
            }
            if ([WeavePackageWindow]::Exists($gui.Id)) { $opened = $true; break }
        }
        if (!$opened) { throw 'The packaged GUI did not create its main window.' }
        Start-Sleep -Seconds 2
        if ($gui.HasExited) { throw "The packaged GUI failed after opening: $($guiErrors.GetAwaiter().GetResult())" }
        $gui.Kill($true)
        $gui.WaitForExit()
        Write-Host 'Packaged GUI launched with its normal GPU backend.'
    }
    if ($SkipGui) { Write-Host "Windows executable validation passed with GUI explicitly skipped: $Package" }
    else { Write-Host "Windows executable validation passed: $Package" }
} finally {
    if ($null -ne $gui) { if (!$gui.HasExited) { $gui.Kill($true); $gui.WaitForExit() }; $gui.Dispose() }
    $resolvedWork = [IO.Path]::GetFullPath($work)
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
    if ([IO.Path]::GetDirectoryName($resolvedWork) -ne $tempRoot -or ![IO.Path]::GetFileName($resolvedWork).StartsWith('weave-package-check-')) { throw 'Invalid package validation cleanup path.' }
    if (Test-Path -LiteralPath $resolvedWork) { Remove-Item -LiteralPath $resolvedWork -Recurse -Force }
}
