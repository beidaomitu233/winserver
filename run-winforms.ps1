$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $root "bin\WinServerManager.exe"

if (-not (Test-Path $exe)) {
  & (Join-Path $root "build-winforms.ps1")
}

Start-Process -FilePath $exe -WorkingDirectory $root
