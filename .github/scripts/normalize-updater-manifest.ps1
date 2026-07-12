param(
    [Parameter(Mandatory = $true)]
    [string]$Repository,

    [Parameter(Mandatory = $true)]
    [string]$Tag
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

$installer = $release.assets |
    Where-Object { $_.name -like "*-setup.exe" } |
    Select-Object -First 1
if (-not $installer) {
    throw "Release $Tag does not contain an NSIS setup executable"
}

$encodedName = [Uri]::EscapeDataString($installer.name)
$downloadUrl = "https://github.com/$Repository/releases/download/$Tag/$encodedName"
$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json

foreach ($platform in $manifest.platforms.PSObject.Properties) {
    if ([string]::IsNullOrWhiteSpace($platform.Value.signature)) {
        throw "Updater signature is missing for platform $($platform.Name)"
    }
    $platform.Value.url = $downloadUrl
}

$json = $manifest | ConvertTo-Json -Depth 20
[IO.File]::WriteAllText($manifestPath, $json, [Text.UTF8Encoding]::new($false))

& gh release upload $Tag $manifestPath --repo $Repository --clobber
if ($LASTEXITCODE -ne 0) {
    throw "Unable to upload normalized latest.json for $Tag"
}

$response = Invoke-WebRequest -UseBasicParsing -Uri $downloadUrl -Method Head
if ($response.Headers["Content-Type"] -match "application/json") {
    throw "Updater URL still resolves to GitHub API metadata instead of the installer"
}

Write-Host "Updater manifest normalized for $Tag -> $downloadUrl"
