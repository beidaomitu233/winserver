$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $root "dist\WinServerManagerNetFx\WinServerManager.exe"
if (-not (Test-Path $exe)) {
  & (Join-Path $root "build-wpf-netfx.ps1")
}
Start-Process -FilePath $exe -WorkingDirectory (Split-Path -Parent $exe)
