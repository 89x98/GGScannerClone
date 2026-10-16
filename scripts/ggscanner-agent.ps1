param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("Stop", "Start", "Restart", "BuildAndRestart")]
    [string]$Action,
    [ValidateSet("Genshin", "StarRail")]
    [string]$Game
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$scannerDirectory = Join-Path $repoRoot "target\release"
$dataDirectory = Join-Path $scannerDirectory "data"
$scannerPath = Join-Path $scannerDirectory "GGScanner.exe"
$appConfigPath = Join-Path $scannerDirectory "data\good_app_config.json"
$processName = "GGScanner"
$stopTask = "Codex GGScanner Stop"
$startTask = "Codex GGScanner Start"

function Test-ScannerRunning {
    return [bool](Get-Process -Name $processName -ErrorAction SilentlyContinue)
}

function Wait-ScannerState([bool]$ShouldRun, [int]$TimeoutSeconds = 30) {
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    do {
        if ((Test-ScannerRunning) -eq $ShouldRun) {
            return
        }
        Start-Sleep -Milliseconds 500
    } while ((Get-Date) -lt $deadline)

    $expected = if ($ShouldRun) { "running" } else { "stopped" }
    throw "Timed out waiting for GGScanner to be $expected."
}

function Invoke-ScannerTask([string]$TaskName) {
    $task = Get-ScheduledTask -TaskName $TaskName -ErrorAction Stop
    if ($task.State -eq "Disabled") {
        throw "Scheduled task '$TaskName' is disabled."
    }
    Start-ScheduledTask -TaskName $TaskName
}

function Set-ManagerGame {
    if (-not $Game) {
        return
    }
    if (-not (Test-Path -LiteralPath $appConfigPath)) {
        throw "GGScanner settings not found: $appConfigPath"
    }
    $config = Get-Content -LiteralPath $appConfigPath -Raw | ConvertFrom-Json
    if (-not $config.navigation) {
        throw "GGScanner settings have no navigation object: $appConfigPath"
    }
    $config.navigation.activeGame = if ($Game -eq "StarRail") { "starRail" } else { "genshin" }
    $config.navigation.selectedTab = "manager"
    $json = ConvertTo-Json -InputObject $config -Depth 100
    [IO.File]::WriteAllText($appConfigPath, $json + "`n", [Text.UTF8Encoding]::new($false))
}

function Start-ScannerAgent {
    New-Item -ItemType Directory -Path $dataDirectory -Force | Out-Null
    [IO.File]::WriteAllText(
        (Join-Path $dataDirectory "ggscanner-agent-mode.once"),
        "agent-mode-v1`n",
        [Text.UTF8Encoding]::new($false)
    )
    Invoke-ScannerTask $startTask
    Wait-ScannerState $true
    Wait-ManagerHealth
}

function Wait-ManagerHealth([int]$TimeoutSeconds = 300) {
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    do {
        try {
            $health = Invoke-RestMethod -Uri "http://127.0.0.1:8765/health" -TimeoutSec 5
            $expectedGame = if ($Game -eq "StarRail") { "star-rail" } else { $null }
            if ($health.status -eq "ok" -and $health.enabled -and
                (-not $expectedGame -or $health.game -eq $expectedGame)) {
                return
            }
        }
        catch {
            # Startup can spend time checking or downloading the OCR runtime.
        }
        Start-Sleep -Seconds 2
    } while ((Get-Date) -lt $deadline)

    $expected = if ($Game -eq "StarRail") { "Star Rail" } else { "Genshin" }
    throw "$expected Manager did not become healthy within $TimeoutSeconds seconds."
}

switch ($Action) {
    "Stop" {
        if (Test-ScannerRunning) {
            Invoke-ScannerTask $stopTask
            Wait-ScannerState $false
        }
        Write-Output "GGScanner stopped."
    }
    "Start" {
        if (-not (Test-Path -LiteralPath $scannerPath)) {
            throw "GGScanner executable not found: $scannerPath"
        }
        if (-not (Test-ScannerRunning)) {
            Set-ManagerGame
            Start-ScannerAgent
        } else {
            Wait-ManagerHealth
        }
        Write-Output "GGScanner and its selected game's Manager server are running."
    }
    "Restart" {
        if (Test-ScannerRunning) {
            Invoke-ScannerTask $stopTask
            Wait-ScannerState $false
        }
        Set-ManagerGame
        Start-ScannerAgent
        Write-Output "GGScanner and its selected game's Manager server are running."
    }
    "BuildAndRestart" {
        if (-not (Test-Path -LiteralPath $scannerPath)) {
            throw "GGScanner executable not found: $scannerPath"
        }
        if (Test-ScannerRunning) {
            Invoke-ScannerTask $stopTask
            Wait-ScannerState $false
        }
        Set-ManagerGame

        Push-Location $repoRoot
        try {
            & cargo build --locked --release -p good_tools_app --features capture --bin GGScanner
            if ($LASTEXITCODE -ne 0) {
                throw "cargo build failed with exit code $LASTEXITCODE. GGScanner remains stopped."
            }
        }
        finally {
            Pop-Location
        }

        Start-ScannerAgent
        Write-Output "GGScanner rebuilt and restarted with its selected game's Manager server running."
    }
}
