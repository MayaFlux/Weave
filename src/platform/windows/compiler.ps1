# Native Visual Studio and Windows SDK discovery only. No environment writes.
$vswhere = @(${env:ProgramFiles(x86)}, $env:ProgramFiles) |
    ForEach-Object { Join-Path $_ 'Microsoft Visual Studio\Installer\vswhere.exe' } |
    Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
if (!$vswhere) { throw 'Visual Studio discovery tool is missing. Retry prerequisite setup.' }
$installation = (& $vswhere -latest -prerelease -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or !$installation -or !(Test-Path -LiteralPath $installation -PathType Container)) { throw 'Microsoft x64 C++ tools were not found. Retry prerequisite setup.' }
$version = (Get-Content -Raw -LiteralPath (Join-Path $installation 'VC\Auxiliary\Build\Microsoft.VCToolsVersion.default.txt')).Trim()
$parsed = $null
if (![Version]::TryParse($version, [ref]$parsed)) { throw 'Microsoft C++ default toolset version is invalid.' }
$tools = Join-Path $installation "VC\Tools\MSVC\$version"
foreach ($relative in @('lib\x64\oldnames.lib', 'lib\x64\msvcrtd.lib')) {
    if (!(Test-Path -LiteralPath (Join-Path $tools $relative) -PathType Leaf)) { throw "Microsoft runtime library is missing: $relative. Retry prerequisite setup." }
}
$registry = [Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::LocalMachine, [Microsoft.Win32.RegistryView]::Registry64)
try {
    $kits = $registry.OpenSubKey('SOFTWARE\Microsoft\Windows Kits\Installed Roots')
    try { $sdk = $kits.GetValue('KitsRoot10') } finally { if ($kits) { $kits.Dispose() } }
} finally { $registry.Dispose() }
if (!$sdk) { throw 'Windows SDK root is missing. Retry prerequisite setup.' }
$sdkVersion = Get-ChildItem -LiteralPath (Join-Path $sdk 'Lib') -Directory |
    Where-Object { $v=$null; [Version]::TryParse($_.Name, [ref]$v) } |
    Sort-Object { [Version]$_.Name } -Descending |
    Where-Object {
        $candidate = $_.Name
        @('Lib\{0}\um\x64\kernel32.lib', 'Lib\{0}\ucrt\x64\ucrt.lib', 'Include\{0}\um\Windows.h', 'Include\{0}\ucrt\stdio.h' | Where-Object {
            !(Test-Path -LiteralPath (Join-Path $sdk ($_ -f $candidate)) -PathType Leaf)
        }).Count -eq 0
    } | Select-Object -First 1 -ExpandProperty Name
if (!$sdkVersion) { throw 'A complete Windows x64 SDK was not found. Retry prerequisite setup.' }
$includes = @((Join-Path $tools 'include')) + @('ucrt', 'shared', 'um', 'winrt', 'cppwinrt' | ForEach-Object {
    Join-Path $sdk "Include\$sdkVersion\$_"
} | Where-Object { Test-Path -LiteralPath $_ -PathType Container })
$libraries = @((Join-Path $tools 'lib\x64'), (Join-Path $sdk "Lib\$sdkVersion\ucrt\x64"), (Join-Path $sdk "Lib\$sdkVersion\um\x64"))
@{ includes=$includes; libraries=$libraries } | ConvertTo-Json -Depth 3 -Compress
