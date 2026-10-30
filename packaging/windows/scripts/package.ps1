param([string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (!$IsWindows -or ![Environment]::Is64BitOperatingSystem) { throw 'Packaging requires x64 Windows and PowerShell 7.' }
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
$buildDirectory = Join-Path $repository 'target/packaging/windows'
if (!$OutputDirectory) { $OutputDirectory = Join-Path $buildDirectory 'output' }
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
$manifest = Get-Content -Raw -LiteralPath (Join-Path $repository 'Cargo.toml')
if ($manifest -notmatch '(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"') { throw 'Cannot read the package version from Cargo.toml.' }
$version = $Matches[1]
$target = 'x86_64-pc-windows-msvc'
$flagName = 'CARGO_ENCODED_RUSTFLAGS'
$previousFlags = [Environment]::GetEnvironmentVariable($flagName, 'Process')
try {
    # An explicit target keeps static-CRT flags off host proc macros. Windows
    # frameworks and graphics drivers remain supplied by the operating system.
    # The encoded flags have Cargo's highest precedence, including when a
    # developer already has RUSTFLAGS or target-specific flags configured.
    $flags = @('-C', 'target-feature=+crt-static', '-C', 'link-arg=/OSVERSION:10.0', '-C', 'link-arg=/SUBSYSTEM:CONSOLE,10.0') -join [char]31
    [Environment]::SetEnvironmentVariable($flagName, $flags, 'Process')
    Push-Location $repository
    try {
        & cargo build --release --locked --target $target
        if ($LASTEXITCODE -ne 0) { throw 'The Windows package build failed.' }
    } finally { Pop-Location }
} finally {
    if ($null -eq $previousFlags) { Remove-Item -LiteralPath "Env:$flagName" -ErrorAction SilentlyContinue }
    else { [Environment]::SetEnvironmentVariable($flagName, $previousFlags, 'Process') }
}
$targetDirectory = if ($env:CARGO_TARGET_DIR) { [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR, $repository) } else { Join-Path $repository 'target' }
$executable = Join-Path $targetDirectory "$target/release/weave.exe"
. (Join-Path $PSScriptRoot 'pe.ps1')
Assert-WeavePe $executable
New-Item -ItemType Directory -Path $buildDirectory, $OutputDirectory -Force | Out-Null
$stage = Join-Path $buildDirectory "stage-$([Guid]::NewGuid().ToString('N'))"
$archive = Join-Path $OutputDirectory "Weave-$version-Windows-x64.zip"
try {
    New-Item -ItemType Directory -Path $stage | Out-Null
    Copy-Item -LiteralPath $executable -Destination (Join-Path $stage 'Weave.exe')
    Copy-Item -LiteralPath (Join-Path $repository 'LICENSE.txt') -Destination $stage
    @"
Weave $version for Windows x64

Requires Windows 10 22H2 (build 19045) or later and the host graphics driver.
Extract this archive and open Weave.exe. No installer, Rust, .NET, or separate
Weave runtime is needed. Templates and scripts are embedded in the executable.
Do not launch as administrator for project or community operations. Choosing
Install MayaFlux requests UAC consent when host installation needs it.

The same executable supports the CLI: .\Weave.exe --help
Git, CMake, compilers, MayaFlux and other build tools remain separate host tools.
"@ | Set-Content -LiteralPath (Join-Path $stage 'README.txt') -Encoding utf8
    @{ version = $version; architecture = 'x64'; minimumWindows = '10.0.19045'; runtime = 'static-msvc'; resources = 'embedded' } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $stage 'package.json') -Encoding utf8
    Compress-Archive -LiteralPath (Join-Path $stage 'Weave.exe'), (Join-Path $stage 'LICENSE.txt'), (Join-Path $stage 'README.txt'), (Join-Path $stage 'package.json') -DestinationPath $archive -Force
} finally {
    $resolvedStage = [IO.Path]::GetFullPath($stage)
    if ([IO.Path]::GetDirectoryName($resolvedStage) -ne $buildDirectory) { throw 'Invalid package staging path.' }
    if (Test-Path -LiteralPath $resolvedStage) { Remove-Item -LiteralPath $resolvedStage -Recurse -Force }
}
Write-Host "Created $archive"
