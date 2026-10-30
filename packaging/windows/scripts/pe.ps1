function Get-WeavePeInfo([string]$Path) {
    $stream = [IO.File]::OpenRead($Path)
    $reader = [System.Reflection.PortableExecutable.PEReader]::new($stream)
    try {
        $header = $reader.PEHeaders.PEHeader
        if ($null -eq $header) { throw 'The package executable is not a PE image.' }
        $imports = [Collections.Generic.List[string]]::new()
        $importLocations = [Collections.Generic.List[object]]::new()
        foreach ($table in @(
            @{ Directory = $header.ImportTableDirectory; Size = 20; NameOffset = 12; Delay = $false },
            @{ Directory = $header.DelayImportTableDirectory; Size = 32; NameOffset = 4; Delay = $true }
        )) {
            if ($table.Directory.Size -eq 0) { continue }
            [byte[]]$bytes = $reader.GetSectionData($table.Directory.RelativeVirtualAddress).GetContent()
            for ($offset = 0; $offset + $table.Size -le [Math]::Min($bytes.Length, $table.Directory.Size); $offset += $table.Size) {
                [int]$nameRva = [BitConverter]::ToUInt32($bytes, $offset + $table.NameOffset)
                if ($nameRva -eq 0) { break }
                if ($table.Delay -and ([BitConverter]::ToUInt32($bytes, $offset) -band 1) -eq 0) {
                    throw 'Unsupported delay-import address format.'
                }
                [byte[]]$nameBytes = $reader.GetSectionData($nameRva).GetContent()
                $end = [Array]::IndexOf($nameBytes, [byte]0)
                if ($end -lt 1) { throw 'Invalid DLL import name.' }
                $name = [Text.Encoding]::ASCII.GetString($nameBytes, 0, $end)
                $imports.Add($name)
                $section = $reader.PEHeaders.SectionHeaders[$reader.PEHeaders.GetContainingSectionIndex($nameRva)]
                $importLocations.Add([pscustomobject]@{ Name = $name; FileOffset = $section.PointerToRawData + $nameRva - $section.VirtualAddress })
            }
        }
        [pscustomobject]@{
            Machine = $reader.PEHeaders.CoffHeader.Machine.ToString()
            Magic = $header.Magic.ToString()
            MinimumOS = "$($header.MajorOperatingSystemVersion).$($header.MinorOperatingSystemVersion)"
            MinimumSubsystem = "$($header.MajorSubsystemVersion).$($header.MinorSubsystemVersion)"
            Managed = $header.CorHeaderTableDirectory.Size -ne 0
            Imports = @($imports | Sort-Object -Unique)
            ImportLocations = $importLocations.ToArray()
        }
    } finally { $reader.Dispose(); $stream.Dispose() }
}

function Assert-WeavePe([string]$Path) {
    $info = Get-WeavePeInfo $Path
    if ($info.Machine -ne 'Amd64' -or $info.Magic -ne 'PE32Plus' -or $info.Managed) {
        throw 'The package must contain a native x64 PE32+ executable.'
    }
    if ($info.MinimumOS -ne '10.0' -or $info.MinimumSubsystem -ne '10.0') {
        throw "The package must encode Windows 10.0 in its PE headers (found $($info.MinimumOS)/$($info.MinimumSubsystem))."
    }
    # These are Windows APIs, not development/runtime redistributables. Do not
    # accept a DLL merely because a developer installed it in System32.
    $systemDlls = @('advapi32.dll','avrt.dll','bcrypt.dll','bcryptprimitives.dll','cfgmgr32.dll',
        'combase.dll','comctl32.dll','comdlg32.dll','crypt32.dll','d2d1.dll','d3d11.dll','d3d12.dll',
        'd3dcompiler_47.dll','dbghelp.dll','dcomp.dll','dinput8.dll','dwmapi.dll','dwrite.dll','dxgi.dll',
        'gdi32.dll','hid.dll','imm32.dll','iphlpapi.dll','kernel32.dll','msimg32.dll','msvcrt.dll',
        'ncrypt.dll','netapi32.dll','normaliz.dll','ntdll.dll','ole32.dll','oleaut32.dll','opengl32.dll',
        'powrprof.dll','propsys.dll','psapi.dll','rpcrt4.dll','secur32.dll','setupapi.dll','shell32.dll',
        'shcore.dll','shlwapi.dll','user32.dll','userenv.dll','usp10.dll','uxtheme.dll','version.dll',
        'winhttp.dll','wininet.dll','winmm.dll','windowscodecs.dll','winspool.drv','ws2_32.dll','wtsapi32.dll')
    foreach ($dll in $info.Imports) {
        if ($dll -notin $systemDlls -and $dll -notmatch '^(api|ext)-ms-win-.*\.dll$') {
            throw "The package depends on a non-OS DLL: $dll. Build with the static MSVC runtime or bundle the dependency."
        }
    }
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne 'NotSigned' -and $signature.Status -ne 'Valid') {
        throw "Executable signature verification failed: $($signature.Status)."
    }
    Write-Host "Verified native x64 image, Windows 10 PE baseline, OS-only imports; signature: $($signature.Status)."
}
