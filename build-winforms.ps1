$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$outDir = Join-Path $root "bin"
$src = Join-Path $root "desktop\WinServerManager.cs"
$exe = Join-Path $outDir "WinServerManager.exe"
$csc = "$env:WINDIR\Microsoft.NET\Framework64\v4.0.30319\csc.exe"

if (-not (Test-Path $csc)) {
  throw "未找到 C# 编译器：$csc"
}

New-Item -ItemType Directory -Force $outDir | Out-Null

& $csc `
  /nologo `
  /target:winexe `
  /platform:x64 `
  /codepage:65001 `
  /define:WINFORMS_UI `
  /out:$exe `
  /r:System.dll `
  /r:System.Core.dll `
  /r:System.Drawing.dll `
  /r:System.Windows.Forms.dll `
  /r:System.Runtime.Serialization.dll `
  /r:System.IO.Compression.dll `
  /r:System.IO.Compression.FileSystem.dll `
  $src

Write-Host "已生成：$exe"
