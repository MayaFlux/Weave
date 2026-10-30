param(
    [string]$ProjectRoot = $PSScriptRoot,
    [string]$Executable,
    [string]$MayaFluxRoot,
    [string]$ProjectName = '@PROJECT_NAME@',
    [string]$Configuration
)

function Resolve-ReportPath([string]$Value, [string]$BaseDirectory) {
    $value = [Environment]::ExpandEnvironmentVariables($Value.Trim().Trim('"'))
    if ($value -match '^[A-Za-z]:[^\\/]' -or $value -match '^[\\/](?![\\/])') {
        throw 'A drive-relative path is not supported. Use a full path or a path relative to the project folder.'
    }
    if (-not [IO.Path]::IsPathRooted($value)) { $value = Join-Path $BaseDirectory $value }
    return [IO.Path]::GetFullPath($value)
}

function Save-PreviousReport([string]$Path, [string]$Suffix) {
    if ([IO.File]::Exists($Path)) {
        $previous = "$Path.previous-$Suffix"
        [IO.File]::Move($Path, $previous)
        Write-Host "Previous log saved: $previous"
    }
}

function Write-ReportLine([string]$Line, [IO.StreamWriter]$Writer) {
    $Writer.WriteLine($Line)
    $Writer.Flush()
    Write-Host $Line
}

$process = $null
$writer = $null
$consoleLog = $null
$applicationLog = $null
$code = 1
$unresolvedProjectName = '@PROJECT_' + 'NAME@'
try {
    $project = Resolve-ReportPath $ProjectRoot (Get-Location).Path
    if (-not [IO.Directory]::Exists($project)) { throw 'The project folder does not exist. Check -ProjectRoot.' }
    if ($Configuration -and $Configuration -notin @('Debug', 'Release', 'RelWithDebInfo', 'MinSizeRel')) {
        throw 'Choose Debug, Release, RelWithDebInfo or MinSizeRel with -Configuration, or use -Executable.'
    }
    if ($Executable) {
        $executablePath = Resolve-ReportPath $Executable $project
        if (-not [IO.File]::Exists($executablePath) -or [IO.Path]::GetExtension($executablePath) -ine '.exe') {
            throw 'The specified executable is missing or is not an .exe file. Check -Executable.'
        }
    }
    else {
        if (-not $ProjectName -or $ProjectName -eq $unresolvedProjectName) {
            $cache = Join-Path $project 'build\CMakeCache.txt'
            if ([IO.File]::Exists($cache)) {
                $names = @([IO.File]::ReadAllLines($cache) | Where-Object { $_ -match '^CMAKE_PROJECT_NAME:[^=]+=' })
                if ($names.Count -eq 1) { $ProjectName = ($names[0] -split '=', 2)[1].Trim() }
            }
        }
        if (-not $ProjectName -or $ProjectName -eq $unresolvedProjectName) {
            throw 'Build the project first, or supply -ProjectName or -Executable for a custom build layout.'
        }
        if ($ProjectName -notmatch '^[A-Za-z_][A-Za-z0-9_]*$' -or $ProjectName -match '^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$') {
            throw 'The project name is unsafe or unsupported. Use -Executable for a custom target name.'
        }
        $layouts = @('build')
        if ($Configuration) { $layouts += "build\$Configuration" }
        else { $layouts += @('build\Debug', 'build\Release', 'build\RelWithDebInfo', 'build\MinSizeRel') }
        $candidates = @($layouts | ForEach-Object { Join-Path (Join-Path $project $_) "$ProjectName.exe" } | Where-Object { [IO.File]::Exists($_) })
        if ($candidates.Count -eq 0) { throw 'No project executable was found. Build the project first, or provide -Executable.' }
        if ($candidates.Count -gt 1) { throw 'Found multiple project executables. Select -Configuration or provide -Executable.' }
        $executablePath = $candidates[0]
    }
    if (-not $MayaFluxRoot) { $MayaFluxRoot = [Environment]::GetEnvironmentVariable('MAYAFLUX_ROOT', 'Process') }
    if (-not $MayaFluxRoot) { $MayaFluxRoot = [Environment]::GetEnvironmentVariable('MAYAFLUX_ROOT', 'User') }
    if (-not $MayaFluxRoot) { $MayaFluxRoot = [Environment]::GetEnvironmentVariable('MAYAFLUX_ROOT', 'Machine') }
    if (-not $MayaFluxRoot) { $MayaFluxRoot = 'C:\MayaFlux' }
    $install = Resolve-ReportPath $MayaFluxRoot $project
    $config = Join-Path $install 'share\MayaFlux\report\mayaflux.report.json'
    if (-not [IO.File]::Exists($config)) {
        throw "The MayaFlux report configuration is missing: $config. Update MayaFlux or specify -MayaFluxRoot."
    }
    try {
        $configurationData = [IO.File]::ReadAllText($config) | ConvertFrom-Json -ErrorAction Stop
        if ($null -eq $configurationData -or $configurationData -isnot [PSCustomObject]) { throw 'Expected a JSON object.' }
    }
    catch { throw "The MayaFlux report configuration is not a readable JSON object: $($_.Exception.Message)" }

    $consoleLog = Join-Path $project 'mayaflux_run.txt'
    $applicationLog = Join-Path $project 'data\log\mayaflux_report.log'
    $suffix = [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss-fff') + '-' + [Guid]::NewGuid().ToString('N')
    Save-PreviousReport $applicationLog $suffix
    Save-PreviousReport $consoleLog $suffix
    $writer = New-Object IO.StreamWriter($consoleLog, $false, (New-Object Text.UTF8Encoding($false)))
    $writer.AutoFlush = $true
    Write-ReportLine "Executable: $executablePath" $writer
    Write-ReportLine "Project folder: $project" $writer
    Write-ReportLine "Report configuration: $config" $writer
    Write-Host 'Reproduce the problem, then close the application to finish collecting logs.'

    $request = New-Object Diagnostics.ProcessStartInfo
    $request.FileName = $executablePath
    $request.WorkingDirectory = $project
    $request.Arguments = '--config "' + $config + '"'
    $request.UseShellExecute = $false
    $request.RedirectStandardOutput = $true
    $request.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($request)
    if (-not $process) { throw 'The project executable could not be started.' }
    $stdout = $process.StandardOutput.ReadLineAsync()
    $stderr = $process.StandardError.ReadLineAsync()
    while ($null -ne $stdout -or $null -ne $stderr) {
        $received = $false
        if ($null -ne $stdout -and $stdout.IsCompleted) {
            $line = $stdout.GetAwaiter().GetResult()
            $stdout = $null
            if ($null -ne $line) {
                Write-ReportLine $line $writer
                $stdout = $process.StandardOutput.ReadLineAsync()
            }
            $received = $true
        }
        if ($null -ne $stderr -and $stderr.IsCompleted) {
            $line = $stderr.GetAwaiter().GetResult()
            $stderr = $null
            if ($null -ne $line) {
                Write-ReportLine $line $writer
                $stderr = $process.StandardError.ReadLineAsync()
            }
            $received = $true
        }
        if (-not $received) { Start-Sleep -Milliseconds 20 }
    }
    $process.WaitForExit()
    $code = $process.ExitCode
    Write-ReportLine "Application exit code: $code" $writer
}
catch {
    $code = 1
    $message = '[ERROR] ' + $_.Exception.Message
    Write-Host $message
    if ($writer) { $writer.WriteLine($message) }
}
finally {
    if ($process) {
        try { if (-not $process.HasExited) { $process.Kill() } }
        finally { $process.Dispose() }
    }
    if ($writer) { $writer.Dispose() }
}
if ($consoleLog -and [IO.File]::Exists($consoleLog)) { Write-Host "Console log: $consoleLog" }
if ($applicationLog) {
    if ([IO.File]::Exists($applicationLog)) { Write-Host "Application report: $applicationLog" }
    else { Write-Host "[WARN] Application report was not created: $applicationLog. Attach mayaflux_run.txt; the application may have stopped before logging started." }
}
exit $code
