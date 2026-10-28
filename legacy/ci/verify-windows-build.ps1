param(
    [string]$ZipPath,
    [string]$ResultsDirectory,
    [switch]$NoRestore
)

$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
if (-not $ZipPath) { $ZipPath = Join-Path $repository 'build\windows\Weave.zip' }
if (-not $ResultsDirectory) { $ResultsDirectory = Join-Path $repository 'build\windows\test-results' }
if (-not (Test-Path -LiteralPath $ZipPath -PathType Leaf)) { throw "Published Windows ZIP not found: $ZipPath" }
$previousZip = $env:WEAVE_WINDOWS_ZIP
try {
    $env:WEAVE_WINDOWS_ZIP = (Resolve-Path -LiteralPath $ZipPath).Path
    $project = Join-Path $repository 'legacy\tests\windows\integration\Weave.Windows.IntegrationTests.csproj'
    $arguments = @('test', $project, '-c', 'Release', '--logger', 'trx;LogFileName=windows-integration.trx', '--results-directory', $ResultsDirectory, '--verbosity', 'quiet')
    if ($NoRestore) { $arguments += '--no-restore' }
    & dotnet @arguments
    $testExitCode = $LASTEXITCODE
} finally {
    $env:WEAVE_WINDOWS_ZIP = $previousZip
}
exit $testExitCode
