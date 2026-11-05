
function __CliSuperpowerInvokeNative {
    param([string]$Program, [string[]]$Values, [string]$Directory)
    if ($Program.IndexOf([char]0) -ge 0) { throw 'Il programma contiene NUL' }
    if ($Program.Contains('\') -or $Program.Contains('/')) {
        $__cspPath = (Microsoft.PowerShell.Management\Resolve-Path -LiteralPath $Program -ErrorAction Stop).ProviderPath
    } else {
        $__cspPattern = [System.Management.Automation.WildcardPattern]::Escape($Program)
        $__cspCommands = @(Microsoft.PowerShell.Core\Get-Command -Name $__cspPattern -CommandType Application -ErrorAction Stop)
        $__cspPath = $__cspCommands[0].Source
    }
    $__cspStart = [System.Diagnostics.ProcessStartInfo]::new()
    $__cspStart.FileName = $__cspPath
    $__cspStart.UseShellExecute = $false
    if ($Directory) {
        $__cspStart.WorkingDirectory = (Microsoft.PowerShell.Management\Resolve-Path -LiteralPath $Directory -ErrorAction Stop).ProviderPath
    } else {
        $__cspStart.WorkingDirectory = (Microsoft.PowerShell.Management\Get-Location -PSProvider FileSystem).ProviderPath
    }
    foreach ($__cspValue in $Values) {
        if ($__cspValue.IndexOf([char]0) -ge 0) { throw 'Un argomento contiene NUL' }
    }
    if ([IO.Path]::GetExtension($__cspPath) -in @('.cmd', '.bat')) {
        # Batch files can expand the supplied arguments again. Accept only data
        # that can cross this shell boundary with unambiguous quoting.
        $__cspBatch = foreach ($__cspValue in (@($__cspPath) + $Values)) {
            if ($__cspValue -match '["%!^&|<>\r\n]' -or $__cspValue.EndsWith('\')) {
                throw 'Il file batch non può ricevere questo argomento senza reinterpretarlo'
            }
            '"' + $__cspValue + '"'
        }
        $__cspStart.FileName = [IO.Path]::Combine([Environment]::SystemDirectory, 'cmd.exe')
        $__cspStart.Arguments = '/d /v:off /s /c "' + ($__cspBatch -join ' ') + '"'
    } else {
        # The native CRT rules also preserve empty values and trailing backslashes.
        # ProcessStartInfo.Arguments is available in Windows PowerShell 5.1.
        $__cspQuoted = foreach ($__cspValue in $Values) {
            $__cspValue = [regex]::Replace($__cspValue, '(\\*)"', '$1$1\"')
            $__cspValue = [regex]::Replace($__cspValue, '(\\+)\z', '$1$1')
            '"' + $__cspValue + '"'
        }
        $__cspStart.Arguments = $__cspQuoted -join ' '
    }
    $__cspProcess = [System.Diagnostics.Process]::Start($__cspStart)
    try {
        $__cspProcess.WaitForExit()
        $global:LASTEXITCODE = $__cspProcess.ExitCode
    } finally {
        $__cspProcess.Dispose()
    }
    if ($global:LASTEXITCODE -ne 0) {
        Microsoft.PowerShell.Utility\Write-Error "Il programma è terminato con codice $global:LASTEXITCODE"
    }
}

