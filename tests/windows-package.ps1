param([Parameter(Mandatory)][string]$Package)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
. (Join-Path $root 'packaging/windows/scripts/pe.ps1')
. (Join-Path $root 'packaging/windows/scripts/gui.ps1')
$work = Join-Path ([IO.Path]::GetTempPath()) "weave-package-negative-$([Guid]::NewGuid().ToString('N'))"
$entries = @{}
$zip = [IO.Compression.ZipFile]::OpenRead((Resolve-Path -LiteralPath $Package).Path)
try {
    foreach ($entry in $zip.Entries) {
        $inputStream = $entry.Open()
        $memory = [IO.MemoryStream]::new()
        try { $inputStream.CopyTo($memory); $entries[$entry.FullName] = $memory.ToArray() }
        finally { $memory.Dispose(); $inputStream.Dispose() }
    }
} finally { $zip.Dispose() }

function Assert-Rejected([scriptblock]$Action, [string]$Reason) {
    $rejected = $false
    try { & $Action } catch {
        if ($_.Exception.Message -notmatch $Reason) { throw }
        $rejected = $true
    }
    if (!$rejected) { throw "Invalid package was accepted; expected $Reason" }
}

$missingGpu = "Weave could not start: Winit backend failed to find a suitable renderer:`nWGPU: No GPU adapters found`n"
Assert-Rejected { Get-WeaveGuiExitOutcome -ExitCode 1 -ErrorText $missingGpu } 'No GPU adapters found'
if ((Get-WeaveGuiExitOutcome -ExitCode 1 -ErrorText $missingGpu -AllowMissingGpu) -ne 'Unavailable') { throw 'The known missing GPU was not classified as unavailable.' }
if ((Get-WeaveGuiExitOutcome -ExitCode 1 -ErrorText ($missingGpu.Replace("`n", "`r`n")) -AllowMissingGpu) -ne 'Unavailable') { throw 'The Windows newline variant was not classified as unavailable.' }
foreach ($errorText in @('', 'VCRUNTIME140.dll is missing', 'WGPU: Failed to create device', 'Weave could not start: panic', "$missingGpu`nUnrelated startup failure")) {
    Assert-Rejected { Get-WeaveGuiExitOutcome -ExitCode 1 -ErrorText $errorText -AllowMissingGpu } 'Packaged GUI exited'
}
Assert-Rejected { Get-WeaveGuiExitOutcome -ExitCode 0 -ErrorText $missingGpu -AllowMissingGpu } 'Packaged GUI exited'

try {
    New-Item -ItemType Directory -Path $work | Out-Null
    $original = [byte[]]$entries['Weave.exe']
    $fixture = Join-Path $work 'wrong-architecture.exe'
    $wrongArchitecture = [byte[]]$original.Clone()
    $peOffset = [BitConverter]::ToInt32($wrongArchitecture, 0x3c)
    $wrongArchitecture[$peOffset + 4] = 0x4c
    $wrongArchitecture[$peOffset + 5] = 0x01
    [IO.File]::WriteAllBytes($fixture, $wrongArchitecture)
    Assert-Rejected { Assert-WeavePe $fixture } 'native x64'

    $fixture = Join-Path $work 'runtime-dependent.exe'
    [IO.File]::WriteAllBytes($fixture, $original)
    $info = Get-WeavePeInfo $fixture
    $longImport = $info.ImportLocations | Where-Object { $_.Name.Length -ge 'VCRUNTIME140.dll'.Length } | Select-Object -First 1
    if (!$longImport) { throw 'No import slot available for the runtime-dependency fixture.' }
    $runtimeDependent = [byte[]]$original.Clone()
    [Text.Encoding]::ASCII.GetBytes("VCRUNTIME140.dll`0").CopyTo($runtimeDependent, $longImport.FileOffset)
    [IO.File]::WriteAllBytes($fixture, $runtimeDependent)
    Assert-Rejected { Assert-WeavePe $fixture } 'non-OS DLL: VCRUNTIME140.dll'

    # Reject unsafe archive entries before extraction, even when every normal
    # package entry is also present. Nothing from this fixture is executed.
    $unsafeZip = Join-Path $work 'unsafe.zip'
    $zip = [IO.Compression.ZipFile]::Open($unsafeZip, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($name in @($entries.Keys) + @('../escape.txt')) {
            $entry = $zip.CreateEntry($name)
            $entryStream = $entry.Open()
            try {
                $bytes = if ($entries.ContainsKey($name)) { $entries[$name] } else { [Text.Encoding]::UTF8.GetBytes('must not escape') }
                $entryStream.Write($bytes, 0, $bytes.Length)
            } finally { $entryStream.Dispose() }
        }
    } finally { $zip.Dispose() }
    Assert-Rejected { & (Join-Path $root 'packaging/windows/scripts/verify.ps1') -Package $unsafeZip -SkipGui } 'Unexpected or missing archive entries'
    Write-Host 'Windows package GPU classification, architecture, runtime-dependency, and unsafe-archive rejection tests passed.'
} finally {
    $resolvedWork = [IO.Path]::GetFullPath($work)
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
    if ([IO.Path]::GetDirectoryName($resolvedWork) -ne $tempRoot -or ![IO.Path]::GetFileName($resolvedWork).StartsWith('weave-package-negative-')) { throw 'Invalid fixture cleanup path.' }
    if (Test-Path -LiteralPath $resolvedWork) { Remove-Item -LiteralPath $resolvedWork -Recurse -Force }
}
