param([string]$MayaFluxRoot, [string]$ProjectRoot)

function Get-ValidationEnvironment($Name, $Target = 'Process') {
    [Environment]::GetEnvironmentVariable($Name, [EnvironmentVariableTarget]$Target)
}

function Expand-ValidationPath($Value) {
    if ([string]::IsNullOrWhiteSpace($Value)) { return '' }
    $expanded = [regex]::Replace($Value.Trim().Trim('"'), '%([^%]+)%', {
        param($match)
        $value = Get-ValidationEnvironment $match.Groups[1].Value
        if ($null -eq $value) { return $match.Value }
        return $value
    })
    if (-not [IO.Path]::IsPathRooted($expanded) -or $expanded -match '^[A-Za-z]:[^\\/]') { return '' }
    [IO.Path]::GetFullPath($expanded).TrimEnd('\', '/')
}

function Find-ValidationTool($Name, $Candidates = @()) {
    foreach ($candidate in $Candidates) {
        if ($candidate -and (Test-Path -LiteralPath $candidate -PathType Leaf)) { return $candidate }
    }
    $command = Get-Command "$Name.exe" -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($command) { return $command.Source }
    return $null
}

function Invoke-ValidationTool($Path, $Arguments, $TimeoutSeconds = 15) {
    $process = $null
    try {
        $request = New-Object Diagnostics.ProcessStartInfo
        $request.FileName = $Path
        $request.Arguments = (($Arguments | ForEach-Object { '"' + $_.Replace('"', '\"') + '"' }) -join ' ')
        $request.UseShellExecute = $false
        $request.CreateNoWindow = $true
        $request.RedirectStandardOutput = $true
        $request.RedirectStandardError = $true
        $process = [Diagnostics.Process]::Start($request)
        if (-not $process) { throw 'The tool could not be started.' }
        $output = $process.StandardOutput.ReadToEndAsync()
        $errorOutput = $process.StandardError.ReadToEndAsync()
        if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
            $process.Kill()
            return @{ Code = -1; Text = "The tool did not finish within $TimeoutSeconds seconds." }
        }
        if (-not $output.Wait(2000) -or -not $errorOutput.Wait(2000)) { throw 'The tool did not finish returning its output.' }
        return @{ Code = $process.ExitCode; Text = ($output.Result + "`n" + $errorOutput.Result).Trim() }
    }
    catch { return @{ Code = -1; Text = $_.Exception.Message } }
    finally { if ($process) { $process.Dispose() } }
}

function Get-ValidationMicrosoftState {
    $compiler = $null
    $vswhere = Find-ValidationTool 'vswhere' @(
        'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe',
        'C:\Program Files\Microsoft Visual Studio\Installer\vswhere.exe'
    )
    if ($vswhere) {
        $result = Invoke-ValidationTool $vswhere @('-products', '*', '-requires', 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', '-find', 'VC\Tools\MSVC\**\bin\Hostx64\x64\cl.exe')
        if ($result.Code -eq 0) {
            $compiler = $result.Text -split '\r?\n' | ForEach-Object { $_.Trim() } | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
        }
    }
    $registry = [Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::LocalMachine, [Microsoft.Win32.RegistryView]::Registry64)
    $kits = $null
    $runtime = $null
    try {
        $kits = $registry.OpenSubKey('SOFTWARE\Microsoft\Windows Kits\Installed Roots')
        $runtime = $registry.OpenSubKey('SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64')
        return @{
            Compiler = $compiler
            SdkRoot = $(if ($kits) { $kits.GetValue('KitsRoot10') })
            RuntimeInstalled = ($runtime -and $runtime.GetValue('Installed') -eq 1)
            RuntimeVersion = $(if ($runtime) { $runtime.GetValue('Version') })
        }
    }
    finally {
        if ($kits) { $kits.Dispose() }
        if ($runtime) { $runtime.Dispose() }
        $registry.Dispose()
    }
}

function Write-ValidationResult($Status, $Message, $Detail) {
    if ($Status -eq 'FIX') { $script:validationIssues++ }
    if ($Status -eq 'WARN') { $script:validationWarnings++ }
    Write-Host "[$Status] $Message"
    if ($Detail) { Write-Host "      $Detail" }
}

function Invoke-ValidationSection($Name, [scriptblock]$Check) {
    Write-Host "`n$Name"
    try { & $Check }
    catch { Write-ValidationResult 'FIX' "$Name could not be checked." ($_.Exception.Message + ' Include this message in your report.') }
}

function Test-ValidationEnvironment($Name, $Expected, [switch]$List, [switch]$OptionalFolder) {
    try {
        $current = Get-ValidationEnvironment $Name
        $saved = Get-ValidationEnvironment $Name 'User'
        if (-not $saved) { $saved = Get-ValidationEnvironment $Name 'Machine' }
        if (-not $current) {
            $next = if ($saved) { 'A saved value exists. Close this terminal completely, open a new one, and run this check again.' } else { 'Run environment setup in Weave, then open a new terminal.' }
            Write-ValidationResult 'FIX' "$Name is not available in this terminal." $next
            return
        }
        $entries = if ($List) { $current -split ';' } else { @($current) }
        $found = @($entries | Where-Object { (Expand-ValidationPath $_) -eq (Expand-ValidationPath $Expected) }).Count -gt 0
        if (-not $found) {
            Write-ValidationResult 'FIX' "$Name does not point to the expected location." "Expected: $Expected. Run environment setup in Weave, then open a new terminal."
            if (-not $List) { Write-Host "      This terminal: $current" }
            return
        }
        if (-not (Test-Path -LiteralPath $Expected -PathType Container)) {
            $status = if ($OptionalFolder) { 'WARN' } else { 'FIX' }
            Write-ValidationResult $status "$Name points to a folder that is missing." $Expected
        }
        else { Write-ValidationResult 'OK' "$Name" $Expected }
        if (-not $List -and $saved -and (Expand-ValidationPath $saved) -ne (Expand-ValidationPath $current)) {
            Write-ValidationResult 'WARN' "$Name has a different saved value for new terminals." "Saved: $saved. Reopen your terminal and check again."
        }
    }
    catch { Write-ValidationResult 'FIX' "$Name could not be checked." ($_.Exception.Message + ' Check access to this location and include this message in your report.') }
}

function Test-ValidationTool($Label, $Name, $Pattern, $Candidates = @(), $Arguments = @('--version')) {
    try {
        $path = Find-ValidationTool $Name $Candidates
        if (-not $path) {
            Write-ValidationResult 'FIX' "$Label was not found." 'Run dependency setup in Weave, then open a new terminal.'
            return
        }
        $result = Invoke-ValidationTool $path $Arguments
        $match = [regex]::Match($result.Text, $Pattern, 'IgnoreCase, Multiline')
        if ($result.Code -ne 0 -or -not $match.Success) {
            Write-ValidationResult 'FIX' "$Label could not be verified." "Location: $path. Exit code: $($result.Code). $($result.Text)"
            return
        }
        if ($Name -eq 'cmake' -and [version]$match.Groups[1].Value -lt [version]'3.31') {
            Write-ValidationResult 'FIX' 'CMake needs an update.' "Found $($match.Groups[1].Value); MayaFlux projects need CMake 3.31 or newer. Run dependency setup in Weave."
            return
        }
        Write-ValidationResult 'OK' $Label (($result.Text -split '\r?\n')[0] + ". Location: $path")
    }
    catch { Write-ValidationResult 'FIX' "$Label could not be checked." ($_.Exception.Message + ' Check access to the tool and include this message in your report.') }
}

function Test-ValidationProject([string]$Directory) {
    try {
        $Directory = [IO.Path]::GetFullPath($Directory)
        if (-not (Test-Path -LiteralPath $Directory -PathType Container)) { throw 'The project folder does not exist.' }
    }
    catch {
        Write-ValidationResult 'FIX' 'The project location could not be checked.' ($_.Exception.Message + ' Check -ProjectRoot and try again.')
        return
    }
    Invoke-ValidationSection 'Project files' {
        Write-Host "      Location: $Directory"
        foreach ($relative in @('CMakeLists.txt', 'CMakePresets.json', 'cmake\mayaflux.cmake', 'cmake\shaders.cmake', 'cmake\build_community.cmake')) {
            $path = Join-Path $Directory $relative
            if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
                Write-ValidationResult 'FIX' "A project build file is missing: $relative" 'Restore the file from your project backup or the installed Weave templates; preserve your custom changes.'
                continue
            }
            $content = Get-Content -LiteralPath $path -Raw -ErrorAction Stop
            if ([string]::IsNullOrWhiteSpace($content)) {
                Write-ValidationResult 'FIX' "A project build file is empty: $relative" 'Restore the build file before configuring the project.'
            }
            elseif ($content -match '@[A-Z][A-Z0-9_]*@') {
                Write-ValidationResult 'FIX' "A template substitution is unfinished in $relative." "Unresolved value: $($Matches[0]). Correct the generated project configuration."
            }
            else { Write-ValidationResult 'OK' $relative }
        }
        $sourceDirectory = Join-Path $Directory 'src'
        $sources = @(if (Test-Path -LiteralPath $sourceDirectory -PathType Container) { Get-ChildItem -LiteralPath $sourceDirectory -Filter '*.cpp' -Recurse -File -ErrorAction Stop })
        if ($sources.Count) { Write-ValidationResult 'OK' 'C++ source files' "$($sources.Count) file(s) under src, including nested folders." }
        else { Write-ValidationResult 'WARN' 'No .cpp files were found under src.' 'The generated CMake template uses src. If you moved your sources, confirm that CMakeLists.txt lists their new locations.' }
    }

    Invoke-ValidationSection 'Project presets' {
        $path = Join-Path $Directory 'CMakePresets.json'
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { return }
        $presets = Get-Content -LiteralPath $path -Raw -ErrorAction Stop | ConvertFrom-Json -ErrorAction Stop
        if (-not $presets.version -or -not $presets.configurePresets) {
            Write-ValidationResult 'FIX' 'Project presets are missing a version or configure presets.' 'Restore or correct CMakePresets.json.'
            return
        }
        foreach ($name in @('release', 'debug')) {
            $configure = @($presets.configurePresets | Where-Object { $_.name -ceq $name })
            $build = @($presets.buildPresets | Where-Object { $_.name -ceq $name })
            if ($configure.Count -ne 1 -or $build.Count -ne 1) {
                Write-ValidationResult 'WARN' "The standard $name configure/build preset pair is unavailable." 'If you renamed the presets, update your VS Code tasks and build commands to match.'
            }
            elseif ($build[0].configurePreset -cne $name) {
                Write-ValidationResult 'WARN' "The $name build preset references another configure preset." "It references '$($build[0].configurePreset)'. Confirm this is intentional."
            }
            else { Write-ValidationResult 'OK' "$name preset pair" }
        }
        $cmake = Find-ValidationTool 'cmake'
        if ($cmake) {
            $result = Invoke-ValidationTool $cmake @('-S', $Directory, '--list-presets=all')
            if ($result.Code -ne 0) { Write-ValidationResult 'FIX' 'CMake could not read the project presets.' $result.Text }
            else { Write-ValidationResult 'OK' 'CMake preset syntax, inheritance and references' }
        }
    }

    Invoke-ValidationSection 'Project configuration' {
        $path = Join-Path $Directory 'mayaflux.json'
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            Write-ValidationResult 'OK' 'No mayaflux.json in the project.' 'Engine defaults apply.'
            return
        }
        $config = Get-Content -LiteralPath $path -Raw -ErrorAction Stop | ConvertFrom-Json -ErrorAction Stop
        $known = @('stream', 'graphics', 'input', 'network', 'journal')
        $unknown = @($config.PSObject.Properties.Name | Where-Object { $known -notcontains $_ })
        if ($unknown.Count) { Write-ValidationResult 'FIX' 'mayaflux.json has unknown top-level sections.' ($unknown -join ', ') }
        else { Write-ValidationResult 'OK' 'mayaflux.json is valid JSON with known sections.' }
    }

    Invoke-ValidationSection 'Project community modules' {
        $list = Join-Path $Directory 'community.cmake'
        if (-not (Test-Path -LiteralPath $list -PathType Leaf)) {
            Write-ValidationResult 'WARN' 'community.cmake is missing.' 'No modules will be discovered through the standard project module list.'
            return
        }
        $names = @(Get-Content -LiteralPath $list -ErrorAction Stop | ForEach-Object { $_.Trim() } | Where-Object { $_ -and -not $_.StartsWith('#') })
        foreach ($name in $names) {
            if ($name -notmatch '^[A-Za-z_][A-Za-z0-9_-]*$') {
                Write-ValidationResult 'FIX' "Unsafe or unsupported community module entry: $name" 'Use one module directory name per line, without paths or CMake commands.'
                continue
            }
            $module = Join-Path $Directory "community\$name\$name.cmake"
            if (-not (Test-Path -LiteralPath $module -PathType Leaf)) {
                Write-ValidationResult 'FIX' "Community module '$name' is listed but its CMake file is missing." $module
            }
            else { Write-ValidationResult 'OK' "Community module: $name" }
        }
        if (-not $names.Count) { Write-ValidationResult 'OK' 'No community modules are requested.' }
    }

    Invoke-ValidationSection 'Project shader sources' {
        $shaderDirectory = Join-Path $Directory 'data\shaders'
        if (-not (Test-Path -LiteralPath $shaderDirectory -PathType Container)) {
            Write-ValidationResult 'WARN' 'data/shaders is missing.' 'Create it if this project uses local shaders. Installed runtime shaders remain available.'
            return
        }
        $stages = @('.vert', '.frag', '.comp', '.geom', '.tesc', '.tese', '.mesh', '.task', '.rgen', '.rchit', '.rahit', '.rmiss', '.rint', '.rcall')
        $shaders = @(Get-ChildItem -LiteralPath $shaderDirectory -Recurse -File -ErrorAction Stop | Where-Object { $stages -contains $_.Extension })
        Write-ValidationResult 'OK' 'Local shader source folder' "$($shaders.Count) stage source(s). GLSL include files remain under data/shaders."
        $pending = @($shaders | Where-Object {
            $compiled = $_.FullName + '.spv'
            -not (Test-Path -LiteralPath $compiled -PathType Leaf) -or (Get-Item -LiteralPath $compiled).LastWriteTimeUtc -lt $_.LastWriteTimeUtc
        })
        if ($pending.Count) {
            Write-ValidationResult 'WARN' 'Some local shader outputs are missing or older than their sources.' "$($pending.Count) source(s). Build the project to compile them; this diagnostic does not compile shaders."
        }
    }

    Invoke-ValidationSection 'Project VS Code configuration' {
        $editorDirectory = Join-Path $Directory '.vscode'
        if (-not (Test-Path -LiteralPath $editorDirectory -PathType Container)) {
            Write-ValidationResult 'OK' 'VS Code configuration is optional and is not present.'
            return
        }
        foreach ($name in @('settings.json', 'tasks.json', 'launch.json')) {
            $path = Join-Path $editorDirectory $name
            if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
                Write-ValidationResult 'WARN' "VS Code $name is missing." 'Configure the editor manually or restore the optional project editor files.'
                continue
            }
            $content = Get-Content -LiteralPath $path -Raw -ErrorAction Stop
            if ($content -match '@[A-Z][A-Z0-9_]*@') {
                Write-ValidationResult 'FIX' "An unfinished template value remains in .vscode/$name." $Matches[0]
            }
            else { Write-ValidationResult 'OK' ".vscode/$name is present" }
        }
    }
}

function Invoke-InstallValidation([string]$Root = $MayaFluxRoot, [string]$ProjectDirectory = $ProjectRoot) {
    $ErrorActionPreference = 'Stop'
    $script:validationIssues = 0
    $script:validationWarnings = 0
    if ($ProjectDirectory) { Write-Host 'MayaFlux project diagnostics' }
    else { Write-Host 'MayaFlux installation check' }
    Write-Host 'This diagnostic is run manually. It checks files, tools and terminal settings without building or changing anything. Nothing was changed.'
    Write-Host "Windows: $([Environment]::OSVersion.Version) | PowerShell: $($PSVersionTable.PSVersion) | 64-bit terminal: $([Environment]::Is64BitProcess)"
    if (-not [Environment]::Is64BitProcess) { Write-ValidationResult 'FIX' 'Open the 64-bit version of PowerShell and try again.' }
    if (-not $Root) { $Root = Get-ValidationEnvironment 'MAYAFLUX_ROOT' }
    if (-not $Root) { $Root = Get-ValidationEnvironment 'MAYAFLUX_ROOT' 'User' }
    if (-not $Root) { $Root = Get-ValidationEnvironment 'MAYAFLUX_ROOT' 'Machine' }
    try { $Root = Expand-ValidationPath $Root } catch { $Root = '' }
    if (-not $Root) { Write-ValidationResult 'FIX' 'The MayaFlux installation location is not set.' 'Run environment setup in Weave, or run this script with -MayaFluxRoot followed by your installation folder.' }
    $roots = @{}
    foreach ($name in @('LLVM_ROOT', 'VULKAN_SDK', 'FFMPEG_ROOT')) {
        try { $roots[$name] = Expand-ValidationPath (Get-ValidationEnvironment $name) }
        catch { $roots[$name] = ''; Write-ValidationResult 'FIX' "$name contains an invalid path." 'Run environment setup in Weave, then open a new terminal.' }
    }
    $llvm = $roots.LLVM_ROOT
    $vulkan = $roots.VULKAN_SDK
    $ffmpeg = $roots.FFMPEG_ROOT

    Invoke-ValidationSection 'MayaFlux installation' {
        if (-not $Root) { return }
        Write-Host "      Location: $Root"
        try {
            $metadata = Get-Content -LiteralPath (Join-Path $Root 'share\MayaFlux\.version') -Raw -ErrorAction Stop | ConvertFrom-Json -ErrorAction Stop
            if ($metadata.version -notmatch '^v?\d+\.\d+\.\d+(?:-dev)?$') { throw 'The installed version information is not valid.' }
            Write-ValidationResult 'OK' 'MayaFlux version' $metadata.version
            if ($metadata.build_time) { Write-Host "      Built: $($metadata.build_time)" }
        }
        catch { Write-ValidationResult 'FIX' 'MayaFlux version information is missing or unreadable.' 'Run MayaFlux installation in Weave again. Your report should include this installation location.' }
        foreach ($relative in @('bin\MayaFluxLib.dll', 'lib\MayaFluxLib.lib', 'lib\cmake\MayaFlux\MayaFluxConfig.cmake')) {
            if (-not (Test-Path -LiteralPath (Join-Path $Root $relative) -PathType Leaf)) { Write-ValidationResult 'FIX' "A required MayaFlux file is missing: $relative" 'Run MayaFlux installation in Weave again.' }
        }
        if (-not (Test-Path -LiteralPath (Join-Path $Root 'include') -PathType Container)) { Write-ValidationResult 'FIX' 'MayaFlux headers are missing.' 'Run MayaFlux installation in Weave again.' }
    }

    Invoke-ValidationSection 'Installed shader resources' {
        if (-not $Root) { return }
        foreach ($entry in @(@('GLSL sources', 'share\MayaFlux\src\shaders'), @('Runtime SPIR-V', 'share\MayaFlux\runtime\shaders'))) {
            $path = Join-Path $Root $entry[1]
            if (-not (Test-Path -LiteralPath $path -PathType Container)) {
                Write-ValidationResult 'WARN' "$($entry[0]) folder is missing." "Expected: $path. Older MayaFlux packages may use a different layout. Update MayaFlux if you need these resources."
                continue
            }
            $pattern = if ($entry[0] -eq 'GLSL sources') { '\.(glsl|vert|frag|comp|geom|tesc|tese|mesh|task|rgen|rchit|rahit|rmiss|rint|rcall)$' } else { '\.spv$' }
            $resources = @(Get-ChildItem -LiteralPath $path -Recurse -File -ErrorAction Stop | Where-Object { $_.Name -match $pattern })
            if ($resources.Count) { Write-ValidationResult 'OK' $entry[0] "$($resources.Count) file(s) at $path" }
            else { Write-ValidationResult 'WARN' "$($entry[0]) folder contains no shader files." "Location: $path. Update MayaFlux if you need these resources." }
        }
    }

    Invoke-ValidationSection 'Bug report configuration' {
        if (-not $Root) { return }
        $path = Join-Path $Root 'share\MayaFlux\report\mayaflux.report.json'
        if (Test-Path -LiteralPath $path -PathType Leaf) { Write-ValidationResult 'OK' 'Report configuration' $path }
        else { Write-ValidationResult 'WARN' 'The report configuration is missing.' "Expected: $path. It ships with MayaFlux 0.5 and later. Update MayaFlux to follow the bug report instructions." }
    }

    Invoke-ValidationSection 'Graphics' {
        foreach ($adapter in @(Get-CimInstance -ClassName Win32_VideoController -ErrorAction Stop)) {
            Write-Host "      $($adapter.Name) | driver $($adapter.DriverVersion)"
        }
        foreach ($name in @('MAYAFLUX_GPU', 'VK_ICD_FILENAMES', 'VK_DRIVER_FILES', 'VK_LAYER_PATH')) {
            $value = [Environment]::GetEnvironmentVariable($name)
            if ($value) { Write-Host "      $name=$value" }
        }
        $vulkaninfo = Find-ValidationTool 'vulkaninfo' $(if ($vulkan) { @((Join-Path $vulkan 'Bin\vulkaninfo.exe'), (Join-Path $vulkan 'Bin\vulkaninfoSDK.exe')) })
        if ($vulkaninfo) {
            $result = Invoke-ValidationTool $vulkaninfo @('--summary') 20
            if ($result.Code -eq 0) {
                foreach ($line in @($result.Text -split '\r?\n' | Where-Object { $_ -match 'deviceName|deviceType|driverName|driverInfo|apiVersion' } | ForEach-Object { $_.Trim() })) {
                    Write-Host "      $line"
                }
                Write-ValidationResult 'OK' 'vulkaninfo'
            }
            else { Write-ValidationResult 'WARN' 'vulkaninfo could not list devices.' $result.Text }
        }
        else { Write-ValidationResult 'WARN' 'vulkaninfo was not found.' 'It is part of the Vulkan SDK. Include its output when reporting graphics problems.' }
        if ($vulkan -and @(Get-ChildItem -LiteralPath (Join-Path $vulkan 'Bin') -Filter 'VkLayer_khronos_validation*' -File -ErrorAction SilentlyContinue).Count) {
            Write-ValidationResult 'OK' 'Vulkan validation layer'
        }
        else { Write-ValidationResult 'WARN' 'The Vulkan validation layer was not found.' 'The Vulkan SDK provides it. The bug report configuration turns validation on.' }
    }

    Invoke-ValidationSection 'Tools used by MayaFlux and Weave' {
        Test-ValidationTool 'CMake' 'cmake' '\bcmake version (\d+\.\d+)'
        Test-ValidationTool 'Git' 'git' '\bgit version \d+\.\d+'
        Test-ValidationTool 'Ninja' 'ninja' '^\d+\.\d+(?:\.\d+)?[^\s]*\s*$'
        Test-ValidationTool 'LLVM' 'clang' '\bclang version \d+\.\d+' $(if ($llvm) { @(Join-Path $llvm 'bin\clang.exe') })
        Test-ValidationTool 'Vulkan SDK' 'glslc' '\bshaderc\s+v?\d+' $(if ($vulkan) { @(Join-Path $vulkan 'Bin\glslc.exe') })
        Test-ValidationTool 'FFmpeg Shared' 'ffmpeg' '\bffmpeg version (?=\S*\d)\S+' $(if ($ffmpeg) { @(Join-Path $ffmpeg 'bin\ffmpeg.exe') }) @('-version')
        Test-ValidationTool '7-Zip' '7z' '7-Zip\s+\d+\.\d+' @('C:\Program Files\7-Zip\7z.exe', 'C:\Program Files (x86)\7-Zip\7z.exe') @('i')
        if (-not (Get-Command winget.exe -CommandType Application -ErrorAction SilentlyContinue)) {
            Write-ValidationResult 'WARN' 'WinGet is not available in this terminal.' 'Installed tools may still work. Weave needs WinGet to install or update dependencies.'
        }
    }

    Invoke-ValidationSection 'FFmpeg shared libraries' {
        if (-not $ffmpeg) { Write-ValidationResult 'FIX' 'FFMPEG_ROOT is not set.' 'Run environment setup in Weave, then open a new terminal.'; return }
        foreach ($component in @('avcodec', 'avformat', 'avutil', 'swresample', 'swscale', 'avdevice')) {
            $library = Join-Path $ffmpeg "lib\$component.lib"
            $header = Join-Path $ffmpeg "include\lib$component\$component.h"
            $dll = @(Get-ChildItem -LiteralPath (Join-Path $ffmpeg 'bin') -Filter "$component-*.dll" -File -ErrorAction SilentlyContinue)
            if (-not (Test-Path -LiteralPath $library -PathType Leaf) -or -not (Test-Path -LiteralPath $header -PathType Leaf) -or $dll.Count -eq 0) {
                Write-ValidationResult 'FIX' "FFmpeg Shared is missing files for $component." "Location: $ffmpeg. Run dependency setup in Weave; the shared package needs libraries and headers as well as ffmpeg.exe."
            }
            else { Write-ValidationResult 'OK' "FFmpeg $component" }
        }
    }

    Invoke-ValidationSection 'Microsoft build support' {
        $state = Get-ValidationMicrosoftState
        if ($state.Compiler -and (Test-Path -LiteralPath $state.Compiler -PathType Leaf)) {
            Write-ValidationResult 'OK' 'Microsoft C++ build tools' $state.Compiler
        }
        else { Write-ValidationResult 'FIX' 'Microsoft C++ build tools were not found.' 'Run prerequisite setup in Weave. LLVM still needs Microsoft C++ libraries and SDK files on Windows.' }
        if ($state.RuntimeInstalled) { Write-ValidationResult 'OK' 'Microsoft x64 C++ runtime' $state.RuntimeVersion }
        else { Write-ValidationResult 'FIX' 'The Microsoft x64 C++ runtime was not found.' 'Run prerequisite setup in Weave.' }
        $usableSdk = $null
        if ($state.SdkRoot) {
            foreach ($version in @(Get-ChildItem -LiteralPath (Join-Path $state.SdkRoot 'Include') -Directory -ErrorAction SilentlyContinue | Sort-Object { try { [version]$_.Name } catch { [version]'0.0' } } -Descending)) {
                $missingHeaders = @(@('um\Windows.h', 'um\audioclient.h', 'um\mmdeviceapi.h', 'um\commdlg.h', 'ucrt\stdio.h') | Where-Object {
                    -not (Test-Path -LiteralPath (Join-Path $version.FullName $_) -PathType Leaf)
                })
                if ($missingHeaders.Count -eq 0 -and
                    (Test-Path -LiteralPath (Join-Path $state.SdkRoot "Lib\$($version.Name)\um\x64\winmm.lib") -PathType Leaf) -and
                    (Test-Path -LiteralPath (Join-Path $state.SdkRoot "Lib\$($version.Name)\ucrt\x64\ucrt.lib") -PathType Leaf)) { $usableSdk = $version.Name; break }
            }
        }
        if ($usableSdk) { Write-ValidationResult 'OK' 'Windows SDK headers and x64 libraries' "$usableSdk at $($state.SdkRoot)" }
        else { Write-ValidationResult 'FIX' 'Windows SDK headers or x64 libraries are missing.' 'Run prerequisite setup in Weave.' }
        Write-Host '      To build projects, use an x64 Visual Studio developer shell. Its INCLUDE and LIB settings belong to that shell.'
    }

    Invoke-ValidationSection 'Environment settings in this terminal' {
        if ($Root) {
            Test-ValidationEnvironment 'MAYAFLUX_ROOT' $Root
            Test-ValidationEnvironment 'PATH' (Join-Path $Root 'bin') -List
            Test-ValidationEnvironment 'CPATH' (Join-Path $Root 'include') -List
            Test-ValidationEnvironment 'LIBRARY_PATH' (Join-Path $Root 'lib') -List
        }
        foreach ($entry in @(@('LLVM_ROOT', $llvm), @('VULKAN_SDK', $vulkan), @('VK_SDK_PATH', $vulkan), @('FFMPEG_ROOT', $ffmpeg))) {
            Test-ValidationEnvironment $entry[0] $entry[1]
            if ($entry[1] -and $entry[0] -ne 'VK_SDK_PATH') { Test-ValidationEnvironment 'PATH' (Join-Path $entry[1] 'bin') -List }
        }
        if ($llvm) {
            Test-ValidationEnvironment 'LLVM_DIR' (Join-Path $llvm 'lib\cmake\llvm') -OptionalFolder
            Test-ValidationEnvironment 'Clang_DIR' (Join-Path $llvm 'lib\cmake\clang') -OptionalFolder
        }
        if ($vulkan) { Test-ValidationEnvironment 'CPATH' (Join-Path $vulkan 'Include') -List }
    }

    if ($ProjectDirectory) { Test-ValidationProject $ProjectDirectory }

    Write-Host "`nSummary"
    if ($script:validationIssues -eq 0) { Write-Host 'No problems found in the required diagnostic checks.' }
    else { Write-Host "$script:validationIssues item(s) need attention. Follow the [FIX] messages above, then run this check again." }
    if ($script:validationWarnings) { Write-Host "$script:validationWarnings warning(s) are shown above for context." }
    Write-Host 'Copy this report, from the heading through this summary, when reporting a problem.'
    if ($ProjectDirectory) { Write-Host 'Project checks do not prove successful compilation or runtime shader lookup. No configure, build or application launch was performed.' }
    else { Write-Host 'To check a project as well, pass -ProjectRoot followed by its folder, or run the copy of this script in that project.' }
    if ($script:validationIssues -gt 0) { return 1 }
    return 0
}

if ($MyInvocation.InvocationName -ne '.') {
    $diagnosticProject = $ProjectRoot
    if (-not $diagnosticProject) {
        foreach ($candidate in @($PSScriptRoot, (Get-Location).Path)) {
            if ($candidate -match '[\\/]share[\\/]weave[\\/]templates[\\/]?$') { continue }
            if ((Test-Path -LiteralPath (Join-Path $candidate 'main.cpp') -PathType Leaf) -and
                (Test-Path -LiteralPath (Join-Path $candidate 'user_project.hpp') -PathType Leaf)) { continue }
            if ((Test-Path -LiteralPath (Join-Path $candidate 'CMakeLists.txt') -PathType Leaf) -or
                ((Test-Path -LiteralPath (Join-Path $candidate 'src') -PathType Container) -and
                 (Test-Path -LiteralPath (Join-Path $candidate 'cmake') -PathType Container))) {
                $diagnosticProject = $candidate
                break
            }
        }
    }
    exit (Invoke-InstallValidation -ProjectDirectory $diagnosticProject)
}
