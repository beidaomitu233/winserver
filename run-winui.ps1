$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $root "dist\WinUIManager\WinServerManager.exe"
if (-not (Test-Path $exe)) {
  & (Join-Path $root "build-winui.ps1")
}
Start-Process -FilePath $exe -WorkingDirectory (Split-Path -Parent $exe)
