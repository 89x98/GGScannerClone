param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("Genshin", "StarRail")]
    [string]$Game,
    [switch]$Characters,
    [switch]$Weapons,
    [switch]$Artifacts,
    [switch]$Achievements,
    [int]$TimeoutMinutes = 90
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$scannerDirectory = Join-Path $repoRoot "target\release"
$dataDirectory = Join-Path $scannerDirectory "data"
$baseUrl = "http://127.0.0.1:8765"

if (-not ($Characters -or $Weapons -or $Artifacts -or $Achievements)) {
    $Characters = $true
    $Weapons = $true
    $Artifacts = $true
    if ($Game -eq "Genshin") {
        $Achievements = $true
    }
}

function Invoke-Api([string]$Method, [string]$Path, $Body = $null) {
    $parameters = @{
        Method = $Method
        Uri = "$baseUrl$Path"
        TimeoutSec = 15
        ErrorAction = "Stop"
    }
    if ($null -ne $Body) {
        $parameters.ContentType = "application/json"
        $parameters.Body = ConvertTo-Json -InputObject $Body -Depth 100 -Compress
    }
    Invoke-RestMethod @parameters
}

function Get-JobId($Response) {
    if ($Response.jobId) {
        return [string]$Response.jobId
    }
    throw "The manager did not return a jobId: $(ConvertTo-Json $Response -Depth 20 -Compress)"
}

$healthDeadline = (Get-Date).AddMinutes(5)
do {
    try {
        $health = Invoke-Api GET "/health"
        break
    }
    catch {
        if ((Get-Date) -ge $healthDeadline) {
            throw "Manager health endpoint did not become available: $($_.Exception.Message)"
        }
        Start-Sleep -Seconds 2
    }
} while ((Get-Date) -lt $healthDeadline)
if ($Game -eq "Genshin") {
    if (-not $health.enabled) {
        throw "Genshin manager is disabled."
    }
    if (-not $health.gameAlive) {
        throw "Genshin Impact was not detected. Open the game and retry."
    }
} else {
    if ($health.game -ne "star-rail") {
        throw "Port 8765 is serving '$($health.game)', not Star Rail. Restart with -Game StarRail."
    }
    if (-not $health.enabled) {
        throw "Star Rail manager is disabled."
    }
}
if ($health.busy) {
    throw "The selected manager already has a running task."
}

if ($Game -eq "Genshin") {
    $body = @{
        characters = [bool]$Characters
        weapons = [bool]$Weapons
        artifacts = [bool]$Artifacts
        achievements = [bool]$Achievements
    }
} else {
    $body = @{
        characters = [bool]$Characters
        lightCones = [bool]$Weapons
        relics = [bool]$Artifacts
    }
}

$accepted = Invoke-Api POST "/scan" $body
$jobId = Get-JobId $accepted
$deadline = (Get-Date).AddMinutes($TimeoutMinutes)
$lastProgress = ""
do {
    Start-Sleep -Seconds 2
    $status = Invoke-Api GET "/status"
    if ($status.jobId -ne $jobId) {
        throw "Manager job changed while scanning (expected $jobId, found $($status.jobId))."
    }

    if ($Game -eq "Genshin") {
        $progress = $status.scanProgress | ConvertTo-Json -Depth 8 -Compress
        $phase = [string]$status.state
    } else {
        $progress = $status.progress | ConvertTo-Json -Depth 8 -Compress
        $phase = [string]$status.phase
    }
    if ($progress -ne $lastProgress) {
        Write-Output "[$phase] $progress"
        $lastProgress = $progress
    }
    if ($phase -in @("failed", "completed", "idle")) {
        break
    }
} while ((Get-Date) -lt $deadline)

if ((Get-Date) -ge $deadline) {
    throw "Scan $jobId exceeded the $TimeoutMinutes minute timeout."
}

$result = Invoke-Api GET "/result?jobId=$([uri]::EscapeDataString($jobId))"
if ($Game -eq "Genshin" -and $phase -eq "completed") {
    $requested = @()
    if ($Characters) { $requested += "characters" }
    if ($Weapons) { $requested += "weapons" }
    if ($Artifacts) { $requested += "artifacts" }
    if ($Achievements) { $requested += "achievements" }
    $data = [ordered]@{ job = $result }
    foreach ($category in $requested) {
        $data[$category] = Invoke-Api GET "/$category`?jobId=$([uri]::EscapeDataString($jobId))"
    }
    $result = $data
}

$gameName = if ($Game -eq "Genshin") { "genshin" } else { "star-rail" }
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$resultPath = Join-Path $dataDirectory "agent-scan-$gameName-$timestamp.json"
New-Item -ItemType Directory -Path $dataDirectory -Force | Out-Null
$resultJson = ConvertTo-Json -InputObject $result -Depth 100
[IO.File]::WriteAllText($resultPath, $resultJson + "`n", [Text.UTF8Encoding]::new($false))
Write-Output "Result saved: $resultPath"
Write-Output (ConvertTo-Json -InputObject $result -Depth 20 -Compress)

if ($phase -eq "failed") {
    exit 1
}
