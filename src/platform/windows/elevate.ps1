$reader = $null
$server = $null
try {
    # The caller owns the server; an elevated worker can report progress even
    # when UAC credentials select a different administrator's profile.
    $server = [IO.Pipes.NamedPipeServerStream]::new($env:WEAVE_INSTALL_PIPE, [IO.Pipes.PipeDirection]::In, 1, [IO.Pipes.PipeTransmissionMode]::Byte, [IO.Pipes.PipeOptions]::Asynchronous)
    $connection = $server.WaitForConnectionAsync()
    $child = Start-Process -FilePath $env:WEAVE_INSTALL_EXECUTABLE -ArgumentList $env:WEAVE_INSTALL_ARGUMENTS -Verb RunAs -PassThru -WindowStyle Hidden
    while (!$connection.Wait(100)) {
        if ($child.HasExited) { exit $child.ExitCode }
    }
    $reader = [IO.StreamReader]::new($server, [Text.Encoding]::UTF8)
    $next = $reader.ReadLineAsync()
    while ($true) {
        if ($next.Wait(100)) {
            if ($null -eq $next.Result) { break }
            [Console]::WriteLine($next.Result)
            $next = $reader.ReadLineAsync()
        }
    }
    [void]$child.WaitForExit()
    exit $child.ExitCode
} catch {
    $cause = $_.Exception
    while ($cause.InnerException) { $cause = $cause.InnerException }
    if ($cause -is [ComponentModel.Win32Exception] -and $cause.NativeErrorCode -eq 1223) {
        [Console]::Error.WriteLine('Administrator consent was cancelled. No installation was started. Retry or continue with project operations.')
        exit 1223
    }
    [Console]::Error.WriteLine($_.Exception.Message)
    exit 1
} finally {
    if ($reader) { $reader.Dispose() }
    if ($server) { $server.Dispose() }
}
