param()

$requiredFiles = @(
    "windows\Weave.nsi",
    "windows\scripts\install_package.ps1", 
    "windows\scripts\packages.psd1",
    "windows\resources\welcome.html",
    "windows\resources\conclusion.html",
    "windows\gui\WeaveGUI.cs",
    "templates"
)

$allPresent = $true
foreach ($file in $requiredFiles) {
    if (Test-Path $file) {
        Write-Host "[OK] Found: $file" -ForegroundColor Green
    } else {
        Write-Host "[ERROR] Missing: $file" -ForegroundColor Red
        $allPresent = $false
    }
}

if (-not $allPresent) {
    exit 1
}
