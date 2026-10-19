$ErrorActionPreference = "Stop"

$currentIdentity = [Security.Principal.WindowsIdentity]::GetCurrent()
$currentPrincipal = [Security.Principal.WindowsPrincipal]::new($currentIdentity)
$isElevated = $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isElevated) {
    $quotedScript = '"' + $PSCommandPath + '"'
    $arguments = "-NoProfile -ExecutionPolicy Bypass -File $quotedScript"
    try {
        Start-Process -FilePath "powershell.exe" -Verb RunAs -ArgumentList $arguments -Wait
    }
    catch {
        throw "Administrator approval was cancelled or failed. No tasks were registered. $($_.Exception.Message)"
    }
    exit
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$scannerDirectory = Join-Path $repoRoot "target\release"
$scannerPath = Join-Path $scannerDirectory "GGScanner.exe"
if (-not (Test-Path -LiteralPath $scannerPath)) {
    throw "Build GGScanner.exe first; expected: $scannerPath"
}

$stopTaskName = "Codex GGScanner Stop"
$startTaskName = "Codex GGScanner Start"
$userSid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$principal = New-ScheduledTaskPrincipal -UserId $userSid -LogonType Interactive -RunLevel Highest
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -MultipleInstances IgnoreNew

# Keep elevated work narrowly scoped: the stop task only terminates this exact
# GGScanner.exe path, and the start task only launches that executable.
$stopCode = @"
`$target = [IO.Path]::GetFullPath('$scannerPath')
Get-CimInstance Win32_Process -Filter "Name='GGScanner.exe'" |
    Where-Object { `$_.ExecutablePath -and [IO.Path]::GetFullPath(`$_.ExecutablePath) -ieq `$target } |
    ForEach-Object { Stop-Process -Id `$_.ProcessId -Force }
"@
$stopEncoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($stopCode))
$stopAction = New-ScheduledTaskAction -Execute "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -Argument "-NoLogo -NoProfile -NonInteractive -WindowStyle Hidden -EncodedCommand $stopEncoded"
$startCode = "Start-Process -FilePath '$scannerPath' -ArgumentList '--agent-mode' -WorkingDirectory '$scannerDirectory'"
$startEncoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($startCode))
$startAction = New-ScheduledTaskAction -Execute "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -Argument "-NoLogo -NoProfile -NonInteractive -WindowStyle Hidden -EncodedCommand $startEncoded"

Register-ScheduledTask -TaskName $stopTaskName -Action $stopAction -Principal $principal -Settings $settings -Force | Out-Null
Register-ScheduledTask -TaskName $startTaskName -Action $startAction -Principal $principal -Settings $settings -Force | Out-Null

Write-Output "Registered '$stopTaskName' and '$startTaskName' for $userSid."
Write-Output "The tasks only stop or start: $scannerPath"
Write-Output "Use scripts\ggscanner-agent.ps1 -Action BuildAndRestart for later iterations."
