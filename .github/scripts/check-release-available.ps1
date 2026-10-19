param(
    [Parameter(Mandatory)][string]$Tag,
    [Parameter(Mandatory)][string]$Repository
)

$ErrorActionPreference = 'Stop'
# Only a confirmed 404 means absence. Authentication/network failures must stop
# publication rather than being mistaken for permission to replace a release.
foreach ($endpoint in @("releases/tags/$Tag", "git/ref/tags/$Tag")) {
    $response = & gh api "repos/$Repository/$endpoint" --silent 2>&1
    if ($LASTEXITCODE -eq 0) {
        throw "Release or tag $Tag already exists. Same-day publication is blocked. To replace it, manually remove the release and its tag first; the workflow never deletes or overwrites them."
    }
    if (($response | Out-String) -notmatch '\(HTTP 404\)') {
        throw "Could not verify availability of $Tag`: $response"
    }
}
Write-Output "Release and tag $Tag are available."
# The confirmed 404 leaves gh's LASTEXITCODE at 1. GitHub's PowerShell wrapper
# propagates it unless this successful availability check exits explicitly.
exit 0
