param(
    [Parameter(Mandatory)][string]$Tag,
    [Parameter(Mandatory)][int]$Revision,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [string]$SourceDirectory = 'target/release',
    [datetime]$PublicationDate = [datetime]::UtcNow.Date
)

$ErrorActionPreference = 'Stop'
$policy = Get-Content -Raw (Join-Path $PSScriptRoot '../release-policy.json') | ConvertFrom-Json
if (Test-Path -LiteralPath $OutputDirectory) {
    throw "Release directory must be new: $OutputDirectory"
}
New-Item -ItemType Directory -Path $OutputDirectory | Out-Null
$editions = @(
    @{ Canonical = 'GGScannerOCR.exe'; Legacy = 'GOODScanner.exe' },
    @{ Canonical = 'GGScanner.exe'; Legacy = 'GOODCapture.exe' }
)
foreach ($edition in $editions) {
    $source = Join-Path $SourceDirectory $edition.Canonical
    Copy-Item -LiteralPath $source -Destination (Join-Path $OutputDirectory $edition.Canonical)
    if ($PublicationDate.Date -lt [datetime]$policy.legacyDownloadsUntil) {
        Copy-Item -LiteralPath $source -Destination (Join-Path $OutputDirectory $edition.Legacy)
    }
}
@{
    tag = $Tag
    revision = $Revision
    assets = @{ scanner = 'GGScannerOCR.exe'; capture = 'GGScanner.exe' }
} | ConvertTo-Json | Set-Content -Encoding utf8 (Join-Path $OutputDirectory 'update.json')
