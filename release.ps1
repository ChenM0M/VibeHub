# release.ps1 - VibeHub Release Script
# Usage: .\release.ps1 2.0.0-pre.1 "Add description" -IncludePaths src/components/ProjectCard.tsx -Push

param(
    [Parameter(Mandatory=$true)]
    [string]$Version,
    
    [Parameter(Mandatory=$false)]
    [string]$Message = "",

    [Parameter(Mandatory=$false)]
    [string[]]$IncludePaths = @(),

    [Parameter(Mandatory=$false)]
    [switch]$Push
)

$ErrorActionPreference = "Stop"
$tag = $Version
$managedReleasePaths = @(
    "package.json",
    "src-tauri/tauri.conf.json",
    "src-tauri/Cargo.toml",
    "src-tauri/Cargo.lock",
    "release.ps1",
    ".github/workflows/release.yml"
)

if ($Version -notmatch '^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$') {
    throw "Version '$Version' must be semver-compatible, for example 2.0.0-pre.1."
}

function Write-Utf8NoBom {
    param(
        [Parameter(Mandatory=$true)]
        [string]$Path,
        [Parameter(Mandatory=$true)]
        [string]$Content
    )
    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Content, $utf8NoBom)
}

function Normalize-GitPath {
    param(
        [Parameter(Mandatory=$true)]
        [string]$Path
    )

    return ($Path -replace '\\', '/').Trim()
}

function Ensure-GitSuccess {
    param(
        [Parameter(Mandatory=$true)]
        [string]$Operation
    )

    if ($LASTEXITCODE -ne 0) {
        throw "$Operation failed."
    }
}

Write-Host ""
Write-Host "=======================================" -ForegroundColor Cyan
Write-Host "     VibeHub Release Script            " -ForegroundColor Cyan
Write-Host "=======================================" -ForegroundColor Cyan
Write-Host "" 

# 1. Check for uncommitted changes
$status = git status --porcelain
Ensure-GitSuccess "git status"
if ($status) {
    Write-Host "[Info] Uncommitted changes detected" -ForegroundColor Yellow
} else {
    Write-Host "[Warn] No changes detected, will only create tag" -ForegroundColor Yellow
}

$releasePaths = @($managedReleasePaths + $IncludePaths | ForEach-Object { Normalize-GitPath $_ } | Select-Object -Unique)
$releasePathSet = @{}
foreach ($path in $releasePaths) {
    $releasePathSet[$path] = $true
}

$preStagedPaths = @(git diff --cached --name-only)
Ensure-GitSuccess "git diff --cached"
$unexpectedStagedPaths = @(
    $preStagedPaths |
    Where-Object {
        $normalized = Normalize-GitPath $_
        -not $releasePathSet.ContainsKey($normalized)
    }
)

if ($unexpectedStagedPaths.Count -gt 0) {
    Write-Host "[Error] Staged files outside the release scope were found:" -ForegroundColor Red
    $unexpectedStagedPaths | ForEach-Object { Write-Host "   $_" -ForegroundColor Red }
    throw "Refusing to continue with unrelated staged files. Unstage or commit them separately first."
}

Write-Host "[Info] Release-managed paths:" -ForegroundColor Cyan
$releasePaths | ForEach-Object { Write-Host "   $_" -ForegroundColor Gray }

# 2. Update version files
Write-Host "Updating version to $Version..." -ForegroundColor Cyan

# Update package.json
$packageJsonPath = "package.json"
if (Test-Path $packageJsonPath) {
    $raw = Get-Content $packageJsonPath -Raw
    $updated = $raw -replace '"version"\s*:\s*"[^"]+"', ('"version": "' + $Version + '"')
    Write-Utf8NoBom -Path $packageJsonPath -Content $updated
    Write-Host "   OK: package.json" -ForegroundColor Green
}

# Update tauri.conf.json
$tauriConfPath = "src-tauri/tauri.conf.json"
if (Test-Path $tauriConfPath) {
    $tauriConf = Get-Content $tauriConfPath -Raw | ConvertFrom-Json
    $tauriConf.version = $Version
    $json = $tauriConf | ConvertTo-Json -Depth 10
    Write-Utf8NoBom -Path $tauriConfPath -Content ($json + "`n")
    Write-Host "   OK: tauri.conf.json" -ForegroundColor Green
}

# Update Cargo.toml - ONLY the package version, not dependencies
$cargoPath = "src-tauri/Cargo.toml"
if (Test-Path $cargoPath) {
    $lines = Get-Content $cargoPath
    $inPackageSection = $false
    $newLines = @()
    foreach ($line in $lines) {
        if ($line -match '^\[package\]') {
            $inPackageSection = $true
        } elseif ($line -match '^\[') {
            $inPackageSection = $false
        }
        
        if ($inPackageSection -and $line -match '^version\s*=\s*"[^"]*"') {
            $newLines += "version = `"$Version`""
        } else {
            $newLines += $line
        }
    }
    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllLines($cargoPath, $newLines, $utf8NoBom)
    Write-Host "   OK: Cargo.toml" -ForegroundColor Green
}

# 3. Build commit message
if ($Message) {
    $commitMsg = $tag + ": " + $Message
} else {
    $commitMsg = $tag + ": Release"
}

# 4. Git operations
Write-Host ""
Write-Host "Git operations..." -ForegroundColor Cyan

git add -- $releasePaths
Ensure-GitSuccess "git add"
Write-Host "   OK: staged release-scoped files only" -ForegroundColor Green

$stagedReleasePaths = @(git diff --cached --name-only)
Ensure-GitSuccess "git diff --cached"
if ($stagedReleasePaths.Count -gt 0) {
    git commit -m $commitMsg
    Ensure-GitSuccess "git commit"
    Write-Host "   OK: git commit: $commitMsg" -ForegroundColor Green
} else {
    Write-Host "   Warn: no staged release file changes to commit" -ForegroundColor Yellow
}

git tag $tag
Ensure-GitSuccess "git tag"
Write-Host "   OK: git tag $tag" -ForegroundColor Green

# 5. Push to remote
if ($Push) {
    Write-Host ""
    Write-Host "Pushing to remote..." -ForegroundColor Cyan

    $branch = git rev-parse --abbrev-ref HEAD
    Ensure-GitSuccess "git rev-parse"
    git push origin $branch --tags
    Ensure-GitSuccess "git push"
    Write-Host "   OK: git push origin $branch --tags" -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host "Push skipped. Use -Push when you are ready to publish the release tag." -ForegroundColor Yellow
}

# 6. Done
Write-Host ""
Write-Host "---------------------------------------" -ForegroundColor Green
Write-Host "Success! $tag" -ForegroundColor Green
Write-Host "---------------------------------------" -ForegroundColor Green
Write-Host ""
Write-Host "Git tag prepared. GitHub Actions will build after you push the tag." -ForegroundColor Gray
Write-Host "Progress: https://github.com/ChenM0M/VibeHub/actions" -ForegroundColor Gray
Write-Host ""
