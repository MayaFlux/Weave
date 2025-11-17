param(
    [string]$OutputDir = "$env:TEMP\Weave-Install",
    [switch]$Download
)

$ErrorActionPreference = "Stop"

try {
    Write-Host "Fetching latest release from GitHub API..."
    $releases = Invoke-RestMethod -Uri 'https://api.github.com/repos/MayaFlux/MayaFlux/releases'
    $release = $releases[0]
    
    $tag = $release.tag_name
    Write-Host "Tag: $tag"
    
    $asset = $release.assets | Where-Object { $_.name -match 'windows-x64\.7z$' } | Select-Object -First 1
    
    if (-not $asset) {
        throw "No Windows .7z asset found. Available: $($release.assets.name -join ', ')"
    }
    
    Write-Host "Found: $($asset.name)"
    
    $tag | Out-File "$OutputDir\tag.txt" -Encoding UTF8 -NoNewline
    $asset.browser_download_url | Out-File "$OutputDir\url.txt" -Encoding UTF8 -NoNewline
    
    if ($Download) {
        Write-Host "Downloading..."
        $outfile = Join-Path $OutputDir "mayaflux.7z"
        Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $outfile -TimeoutSec 600
        Write-Host "Downloaded: $outfile"
    }
    
    Write-Host "Success"
    exit 0
}
catch {
    Write-Host "ERROR: $($_.Exception.Message)"
    $_.Exception.Message | Out-File "$OutputDir\error.txt" -Encoding UTF8
    exit 1
}