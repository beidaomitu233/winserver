$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root

$node = Get-Command node -ErrorAction SilentlyContinue
if (-not $node) {
  throw "未找到 Node.js。请先安装 Node.js 18+，或把 node.exe 加入 PATH。"
}

Write-Host "XP.CN 小皮 Web 管理台启动中..."
Write-Host "地址：http://127.0.0.1:18113"
& $node.Source server.js
