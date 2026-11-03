# Weave graphics driver advisory v1. Reads installed driver records only.
function Get-WeaveGraphicsDriverRules {
    @'
# @WEAVE_DRIVER_RULES@
'@ | ConvertFrom-Json -ErrorAction Stop
}

function Get-WeaveGraphicsDriverAdvisories {
    param([object[]]$Drivers)
    if (-not $PSBoundParameters.ContainsKey('Drivers')) {
        $Drivers = @(Get-CimInstance Win32_PnPSignedDriver -Filter "DeviceClass = 'DISPLAY'" -OperationTimeoutSec 10 -ErrorAction Stop)
    }
    $rules = Get-WeaveGraphicsDriverRules
    $seen = @{}
    foreach ($driver in $Drivers) {
        $provider = ([string]$driver.DriverProviderName).Trim()
        $rule = $rules.vendors | Where-Object { $provider -match $_.providerPattern } | Select-Object -First 1
        if (-not $rule) { continue }
        $raw = ([string]$driver.DriverVersion).Trim()
        $key = "$($rule.name):$raw"
        if ($seen.ContainsKey($key)) { continue }
        $seen[$key] = $true
        $version = $null
        $kind = 'unknown'
        if ($raw -notmatch '^\d+\.\d+\.\d+\.\d+$' -or -not [version]::TryParse($raw, [ref]$version)) {
            $message = "The installed $($rule.name) graphics driver version could not be read. Check for the latest available driver."
        }
        elseif (-not $rule.minimumWindowsVersion) {
            $message = "$($rule.name) graphics driver $raw is installed. An automatic age comparison is not available for this driver provider. Check for the latest available driver."
        }
        elseif ($version -lt [version]$rule.minimumWindowsVersion) {
            $kind = 'outdated'
            $message = "Your $($rule.name) graphics driver is older than the MayaFlux driver baseline. Installed: $raw. Baseline: $($rule.minimumWindowsVersion) ($($rule.release)). Install the latest driver available for your system."
        }
        else { continue }
        [pscustomobject]@{ Vendor = $rule.name; Kind = $kind; Message = $message; Url = $rule.downloadUrl }
    }
}

function Write-GraphicsDriverAdvisory {
    Write-Host "`nGraphics driver update advice"
    try {
        $notices = @(Get-WeaveGraphicsDriverAdvisories)
        foreach ($notice in $notices) {
            $status = if ($notice.Kind -eq 'outdated') { 'WARN' } else { 'INFO' }
            Write-ValidationResult $status $notice.Message $notice.Url
        }
        if ($notices.Count -eq 0) { Write-Host 'No outdated graphics drivers were identified by the installed-driver version check.' }
    }
    catch { Write-ValidationResult 'WARN' 'Installed graphics driver versions could not be checked.' 'Check your graphics driver provider for updates, then run validation again.' }
}
