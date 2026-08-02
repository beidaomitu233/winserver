param(
    [Parameter(Mandatory = $true)]
    [string]$Repository,

    [Parameter(Mandatory = $true)]
    [string]$Tag,

    [switch]$Publish
)

$ErrorActionPreference = "Stop"

$workDir = Join-Path $env:RUNNER_TEMP "winserver-updater-manifest-$Tag"
New-Item -ItemType Directory -Path $workDir -Force | Out-Null
$manifestPath = Join-Path $workDir "latest.json"

& gh release download $Tag --repo $Repository --pattern latest.json --dir $workDir --clobber
if ($LASTEXITCODE -ne 0) {
    throw "Unable to download latest.json for $Tag"
}

$release = (& gh api "repos/$Repository/releases/tags/$Tag") | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) {
    throw "Unable to inspect release $Tag"
}

$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
$expectedVersion = $Tag -replace '^v', ''
if ([string]$manifest.version -ne $expectedVersion) {
    throw "Updater manifest version $($manifest.version) does not match tag $Tag"
}

# Tauri signs the NSIS updater archive, not the human-facing setup executable.
# latest.json must therefore point to *.nsis.zip and carry the matching
# *.nsis.zip.sig contents. Pointing it at *-setup.exe produces a guaranteed
# signature mismatch during downloadAndInstall().
$updaterBundle = $release.assets |
    Where-Object { $_.name -like "*_x64-setup.nsis.zip" -or $_.name -like "*-x64-setup.nsis.zip" } |
    Select-Object -First 1
if (-not $updaterBundle) {
    $updaterBundle = $release.assets |
        Where-Object { $_.name -like "*.nsis.zip" } |
        Select-Object -First 1
}
if (-not $updaterBundle) {
    throw "Release $Tag does not contain an NSIS updater bundle (*.nsis.zip)"
}

$signatureName = "$($updaterBundle.name).sig"
$signatureAsset = $release.assets |
    Where-Object { $_.name -eq $signatureName } |
    Select-Object -First 1
if (-not $signatureAsset) {
    throw "Release $Tag does not contain the matching updater signature $signatureName"
}

& gh release download $Tag --repo $Repository --pattern $signatureName --dir $workDir --clobber
if ($LASTEXITCODE -ne 0) {
    throw "Unable to download updater signature $signatureName"
}

$signaturePath = Join-Path $workDir $signatureName
$signature = (Get-Content -LiteralPath $signaturePath -Raw -Encoding UTF8).Trim()
if ([string]::IsNullOrWhiteSpace($signature)) {
    throw "Updater signature $signatureName is empty"
}

$windowsPlatforms = @(
    $manifest.platforms.PSObject.Properties |
        Where-Object { $_.Name -like "windows-*" }
)
if ($windowsPlatforms.Count -eq 0) {
    throw "latest.json does not contain a Windows platform entry"
}

foreach ($platform in $windowsPlatforms) {
    $platform.Value.url = [string]$updaterBundle.browser_download_url
    $platform.Value.signature = $signature
}

$json = $manifest | ConvertTo-Json -Depth 20
[IO.File]::WriteAllText($manifestPath, $json, [Text.UTF8Encoding]::new($false))

& gh release upload $Tag $manifestPath --repo $Repository --clobber
if ($LASTEXITCODE -ne 0) {
    throw "Unable to upload normalized latest.json for $Tag"
}

# Probe the actual updater archive through GitHub's public download URL. A range
# request avoids downloading the full package while still detecting API JSON or
# an HTML error page.
$response = Invoke-WebRequest `
    -UseBasicParsing `
    -Uri $updaterBundle.browser_download_url `
    -Method Get `
    -Headers @{ Range = "bytes=0-1023" }
$contentType = [string]$response.Headers["Content-Type"]
if ($contentType -match "application/json|text/html") {
    throw "Updater URL resolves to metadata or HTML instead of the NSIS updater bundle: $contentType"
}
if ($response.RawContentLength -le 0) {
    throw "Updater bundle probe returned no data"
}

if ($Publish) {
    & gh release edit $Tag --repo $Repository --draft=false --prerelease=false --latest
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to publish release $Tag after updater validation"
    }
}

Write-Host "Updater manifest validated for $Tag -> $($updaterBundle.browser_download_url)"
