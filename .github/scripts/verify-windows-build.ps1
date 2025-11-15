param(
    [string]$Version
)

$outputExe = "build\windows\Weave-${Version}.exe"

if (Test-Path $outputExe) {
    $fileSize = (Get-Item $outputExe).Length / 1MB
    Write-Host "[OK] Installer created: $outputExe" -ForegroundColor Green
    Write-Host "     Size: $([Math]::Round($fileSize, 2)) MB" -ForegroundColor Green
} else {
    Write-Host "[ERROR] Installer not found at: $outputExe" -ForegroundColor Red
    exit 1
}
