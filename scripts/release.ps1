# Starts a new release. GitHub builds, signs and publishes it (.github/workflows/release.yml).
#   powershell -ExecutionPolicy Bypass -File scripts\release.ps1 -Version 0.2.0 -Notes "What changed"
# Bumps the version, runs the tests, commits, then pushes a v<version> tag with the notes.
param(
  [Parameter(Mandatory)][string]$Version,
  [string]$Notes = "",
  [switch]$SkipTests
)
$ErrorActionPreference = "Stop"
$Repo = "CukroTheOtter/pregnancy-osc"
Set-Location (Split-Path -Parent $PSScriptRoot)

function Run([string]$what, [scriptblock]$cmd) {
  Write-Host "`n== $what" -ForegroundColor Cyan
  & $cmd
  if ($LASTEXITCODE -ne 0) { throw "$what failed (exit $LASTEXITCODE)" }
}
function Replace-First([string]$path, [string]$pattern, [string]$with) {
  $text = [IO.File]::ReadAllText($path)
  $new = ([regex]$pattern).Replace($text, $with, 1)
  if ($new -eq $text -and $text -notmatch [regex]::Escape($with)) { throw "Version not found in $path" }
  [IO.File]::WriteAllText($path, $new, (New-Object Text.UTF8Encoding $false))
}

# Checks first, so nothing is half done.
if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw "Version must look like 1.2.3" }
$login = gh api user --jq .login
if ($login -ne "CukroTheOtter") { throw "gh is signed in as '$login'. Run: gh auth switch" }
if (git status --porcelain) { throw "Commit or stash your changes first." }
if (git tag -l "v$Version") { throw "v$Version already exists." }

Run "Bump version" { npm version $Version --no-git-tag-version --allow-same-version | Out-Null }
Replace-First "src-tauri\Cargo.toml" '(?m)^version = "[^"]+"' "version = `"$Version`""
Replace-First "src-tauri\tauri.conf.json" '"version": "[^"]+"' "`"version`": `"$Version`""

if (-not $SkipTests) {
  Run "Rust tests" { cargo test --workspace --quiet }
  Run "UI tests" { npm test --silent }
}

if (-not $Notes) { $Notes = "Version $Version" }
Run "Stage" { git add -A }
Run "Commit" { git commit -q -m "Release v$Version" }
Run "Tag" { git tag -a "v$Version" -m $Notes }
Run "Push" { git push -q origin HEAD "v$Version" }
Write-Host "`nPushed v$Version. GitHub is building it (about 10-15 min):" -ForegroundColor Green
Write-Host "  https://github.com/$Repo/actions"
