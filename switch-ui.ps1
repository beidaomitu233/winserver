param(
  [Parameter(Mandatory = $true)]
  [ValidateSet("web", "winforms", "wpf", "winui")]
  [string]$Ui,

  [switch]$Create
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root

$branches = @{
  web      = "codex/simple-web-manager"
  winforms = "codex/winforms-ui"
  wpf      = "codex/wpf-ui"
  winui    = "codex/winui-ui"
}

$target = $branches[$Ui]
$dirty = git status --porcelain
if ($dirty) {
  Write-Host "Working tree has uncommitted changes. Commit or stash before switching UI branches." -ForegroundColor Yellow
  git status --short
  exit 1
}

$exists = git show-ref --verify --quiet "refs/heads/$target"
if ($LASTEXITCODE -eq 0) {
  git switch $target
  exit 0
}

if ($Create) {
  git switch -c $target
  exit 0
}

Write-Host "Branch $target does not exist. After an initial commit, create it with:" -ForegroundColor Yellow
Write-Host ".\switch-ui.ps1 $Ui -Create"
exit 1
