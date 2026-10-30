# Only this complete backend diagnostic identifies unavailable GPU coverage.
# Missing DLLs, device failures, and other startup errors must still fail CI.
function Get-WeaveGuiExitOutcome([int]$ExitCode, [string]$ErrorText, [switch]$AllowMissingGpu) {
    if ($AllowMissingGpu -and $ExitCode -ne 0 -and
        $ErrorText -match '\AWeave could not start: Winit backend failed to find a suitable renderer:\s*WGPU: No GPU adapters found\s*\z') {
        return 'Unavailable'
    }
    throw "Packaged GUI exited during startup (exit $ExitCode): $ErrorText"
}
