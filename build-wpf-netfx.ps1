$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$outDir = Join-Path $root "dist\WinServerManagerNetFx"
$srcCore = Join-Path $root "desktop\WinServerManager.cs"
$srcWpf = Join-Path $root "desktop\WpfNetFxManager.cs"
$exe = Join-Path $outDir "WinServerManager.exe"
$csc = "$env:WINDIR\Microsoft.NET\Framework64\v4.0.30319\csc.exe"

$pf = "$env:WINDIR\Microsoft.NET\assembly\GAC_MSIL\PresentationFramework\v4.0_4.0.0.0__31bf3856ad364e35\PresentationFramework.dll"
$pc = "$env:WINDIR\Microsoft.NET\assembly\GAC_64\PresentationCore\v4.0_4.0.0.0__31bf3856ad364e35\PresentationCore.dll"
$wb = "$env:WINDIR\Microsoft.NET\assembly\GAC_MSIL\WindowsBase\v4.0_4.0.0.0__31bf3856ad364e35\WindowsBase.dll"
$xaml = "$env:WINDIR\Microsoft.NET\Framework64\v4.0.30319\System.Xaml.dll"

New-Item -ItemType Directory -Force $outDir | Out-Null

& $csc `
  /nologo `
  /target:winexe `
  /platform:x64 `
  /codepage:65001 `
  /out:$exe `
  /r:System.dll `
  /r:System.Core.dll `
  /r:System.Drawing.dll `
  /r:System.Windows.Forms.dll `
  /r:System.Runtime.Serialization.dll `
  /r:System.IO.Compression.dll `
  /r:System.IO.Compression.FileSystem.dll `
  /r:$pf `
  /r:$pc `
  /r:$wb `
  /r:$xaml `
  $srcCore `
  $srcWpf

Write-Host "已生成：$exe"
