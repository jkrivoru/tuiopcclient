# Release script for opcua-client (PowerShell)
# Usage: .\scripts\release.ps1 <version>
# Example: .\scripts\release.ps1 v0.1.1

param(
    [Parameter(Mandatory=$true)]
    [string]$Version
)

$ErrorActionPreference = "Stop"

Write-Host "🚀 Starting release process for $Version" -ForegroundColor Green

# Validate version format
if ($Version -notmatch "^v\d+\.\d+\.\d+$") {
    Write-Host "❌ Invalid version format. Use semantic versioning like v1.0.0" -ForegroundColor Red
    exit 1
}

# Check current branch
$currentBranch = git rev-parse --abbrev-ref HEAD
if ($currentBranch -ne "main") {
    Write-Host "⚠️  Warning: You're not on the main branch. Current branch: $currentBranch" -ForegroundColor Yellow
    $continue = Read-Host "Continue anyway? (y/N)"
    if ($continue -notmatch "^[Yy]$") {
        exit 1
    }
}

# Check for uncommitted changes
$status = git status --porcelain
if ($status) {
    Write-Host "❌ You have uncommitted changes. Please commit or stash them first." -ForegroundColor Red
    exit 1
}

# Extract version number without 'v' prefix
$versionNumber = $Version.Substring(1)

Write-Host "📝 Updating Cargo.toml version to $versionNumber" -ForegroundColor Blue
$cargoContent = Get-Content "Cargo.toml" -Raw
$cargoContent = $cargoContent -replace 'version = ".*"', "version = `"$versionNumber`""
Set-Content "Cargo.toml" -Value $cargoContent

Write-Host "📝 Updating CHANGELOG.md" -ForegroundColor Blue
$today = Get-Date -Format "yyyy-MM-dd"
$changelogContent = Get-Content "CHANGELOG.md" -Raw
$changelogContent = $changelogContent -replace "## \[Unreleased\]", "## [Unreleased]`n`n## [$versionNumber] - $today"
Set-Content "CHANGELOG.md" -Value $changelogContent

Write-Host "🔨 Building to verify everything works" -ForegroundColor Blue
cargo build --release

Write-Host "🧪 Running tests" -ForegroundColor Blue
cargo test

Write-Host "📋 Preparing commit" -ForegroundColor Blue
git add Cargo.toml CHANGELOG.md Cargo.lock
git commit -m "chore: bump version to $Version"

Write-Host "🏷️  Creating git tag" -ForegroundColor Blue
git tag -a "$Version" -m "Release $Version"

Write-Host "📤 Pushing changes and tag" -ForegroundColor Blue
git push origin $currentBranch
git push origin $Version

Write-Host "✅ Release $Version has been created!" -ForegroundColor Green
Write-Host "🔗 GitHub Actions will automatically build and create the release." -ForegroundColor Cyan
Write-Host "🔗 Monitor the progress at: https://github.com/yourusername/jk-opc-client/actions" -ForegroundColor Cyan
Write-Host ""
Write-Host "📋 Next steps:" -ForegroundColor Yellow
Write-Host "1. Wait for the GitHub Actions release workflow to complete"
Write-Host "2. Edit the release notes on GitHub if needed"
Write-Host "3. Announce the release"
