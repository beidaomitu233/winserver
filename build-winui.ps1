$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$dotnet = Join-Path $root ".tools\dotnet\dotnet.exe"
$project = Join-Path $root "src\WinUIManager\WinUIManager.csproj"
$out = Join-Path $root "dist\WinUIManager"

& $dotnet publish $project -c Release -r win-x64 --self-contained true -o $out
Write-Host "已发布：$out\WinServerManager.exe"
