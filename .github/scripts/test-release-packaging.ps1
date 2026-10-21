$ErrorActionPreference = 'Stop'
$testRoot = Join-Path ([IO.Path]::GetTempPath()) "ggscanner-release-test-$([guid]::NewGuid())"
New-Item -ItemType Directory -Path $testRoot | Out-Null
try {
    $source = Join-Path $testRoot 'source'
    New-Item -ItemType Directory -Path $source | Out-Null
    Set-Content (Join-Path $source 'GGScanner.exe') 'capture edition'
    Set-Content (Join-Path $source 'GGScannerOCR.exe') 'OCR edition'
    foreach ($date in @('2026-10-21', '2026-11-20', '2026-11-21', '2026-12-01')) {
        $output = Join-Path $testRoot $date
        & "$PSScriptRoot/prepare-release.ps1" -Tag 'v2026.10.21' -Revision 123 -OutputDirectory $output -SourceDirectory $source -PublicationDate $date
        $expected = @('GGScanner.exe', 'GGScannerOCR.exe', 'update.json')
        if ([datetime]$date -lt [datetime]'2026-11-21') {
            $expected += @('GOODCapture.exe', 'GOODScanner.exe')
            foreach ($pair in @(@('GGScanner.exe', 'GOODCapture.exe'), @('GGScannerOCR.exe', 'GOODScanner.exe'))) {
                if ((Get-FileHash (Join-Path $output $pair[0])).Hash -ne (Get-FileHash (Join-Path $output $pair[1])).Hash) {
                    throw 'Compatibility copy changed editions'
                }
            }
        }
        $actual = @(Get-ChildItem -LiteralPath $output -File | Select-Object -ExpandProperty Name)
        if (Compare-Object ($expected | Sort-Object) ($actual | Sort-Object)) { throw "Wrong release inventory on $date" }
        $manifest = Get-Content -Raw (Join-Path $output 'update.json') | ConvertFrom-Json
        if ($manifest.assets.capture -ne 'GGScanner.exe' -or $manifest.assets.scanner -ne 'GGScannerOCR.exe' -or $manifest.revision -ne 123) {
            throw 'Wrong updater manifest'
        }
    }
    Write-Output 'Release inventories, edition hashes, canonical manifests and cutoff checks passed.'
} finally {
    $resolved = [IO.Path]::GetFullPath($testRoot)
    $temp = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    if (-not $resolved.StartsWith($temp) -or [IO.Path]::GetFileName($resolved) -notlike 'ggscanner-release-test-*') {
        throw 'Unexpected test cleanup path'
    }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
