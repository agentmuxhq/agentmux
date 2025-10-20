#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Bump version across all WaveTerm fork configs and docs
.DESCRIPTION
    Updates version in package.json, package-lock.json, VERSION_HISTORY.md, and commits changes.
    Creates a git tag for the new version.
.PARAMETER Type
    Version bump type: patch, minor, major, or specific version (e.g., 0.12.5)
.PARAMETER Agent
    Agent name (default: current branch agent prefix or 'agentx')
.PARAMETER Message
    Commit message describing changes (default: generic bump message)
.PARAMETER NoCommit
    Skip git commit and tag creation
.PARAMETER NoTag
    Skip git tag creation (still commits)
.EXAMPLE
    ./bump-version.ps1 patch
    Bumps patch version (0.12.3 -> 0.12.4)
.EXAMPLE
    ./bump-version.ps1 minor -Agent agent2 -Message "Add new terminal feature"
    Bumps minor version with custom message
.EXAMPLE
    ./bump-version.ps1 0.12.10
    Sets specific version
#>

param(
    [Parameter(Mandatory=$true)]
    [string]$Type,

    [Parameter(Mandatory=$false)]
    [string]$Agent = "",

    [Parameter(Mandatory=$false)]
    [string]$Message = "",

    [Parameter(Mandatory=$false)]
    [switch]$NoCommit,

    [Parameter(Mandatory=$false)]
    [switch]$NoTag
)

# Colors for output
function Write-Success { param($msg) Write-Host "✓ $msg" -ForegroundColor Green }
function Write-Info { param($msg) Write-Host "→ $msg" -ForegroundColor Cyan }
function Write-Error { param($msg) Write-Host "✗ $msg" -ForegroundColor Red }

# Get current version
$packageJson = Get-Content -Path "package.json" -Raw | ConvertFrom-Json
$currentVersion = $packageJson.version
Write-Info "Current version: $currentVersion"

# Determine new version
$newVersion = ""
if ($Type -match '^\d+\.\d+\.\d+$') {
    # Specific version provided
    $newVersion = $Type
    Write-Info "Setting specific version: $newVersion"
} else {
    # Use npm version to calculate new version
    Write-Info "Bumping $Type version..."
    try {
        # Run npm version with --no-git-tag-version to prevent automatic git operations
        $npmOutput = npm version $Type --no-git-tag-version 2>&1
        if ($LASTEXITCODE -ne 0) {
            Write-Error "npm version failed: $npmOutput"
            exit 1
        }
        $newVersion = ($npmOutput -replace '^v', '')
    } catch {
        Write-Error "Failed to bump version: $_"
        exit 1
    }
}

Write-Success "New version: $newVersion"

# Determine agent name
if ($Agent -eq "") {
    # Try to get from current branch
    $branch = git rev-parse --abbrev-ref HEAD 2>$null
    if ($branch -match '^(agent\w+)/') {
        $Agent = $matches[1]
    } else {
        $Agent = "agentx"
    }
}
Write-Info "Agent: $Agent"

# Update VERSION_HISTORY.md
Write-Info "Updating VERSION_HISTORY.md..."
$today = Get-Date -Format "yyyy-MM-dd"
$versionHistoryPath = "VERSION_HISTORY.md"

if (Test-Path $versionHistoryPath) {
    $content = Get-Content -Path $versionHistoryPath -Raw

    # Update current version at top
    $content = $content -replace 'Current Version: [\d.]+(-fork)?', "Current Version: $newVersion-fork"

    # Find the table and add new entry after header
    $tablePattern = '(\| Fork Version \| Upstream Base \| Date \| Agent \| Changes \|\r?\n\|[-\|]+\|\r?\n)'
    $changeMsg = if ($Message) { $Message } else { "Version bump" }
    $newEntry = "| $newVersion-fork | v0.12.0 | $today | $Agent | $changeMsg |`n"
    $content = $content -replace $tablePattern, "`$1$newEntry"

    Set-Content -Path $versionHistoryPath -Value $content -NoNewline
    Write-Success "Updated VERSION_HISTORY.md"
} else {
    Write-Error "VERSION_HISTORY.md not found!"
}

# Commit changes if requested
if (-not $NoCommit) {
    Write-Info "Committing version bump..."

    git add package.json package-lock.json VERSION_HISTORY.md

    $commitMsg = if ($Message) {
        "chore: bump version to $newVersion`n`n$Message"
    } else {
        "chore: bump version to $newVersion"
    }

    git commit -m $commitMsg

    if ($LASTEXITCODE -eq 0) {
        Write-Success "Committed version bump"
    } else {
        Write-Error "Failed to commit changes"
        exit 1
    }

    # Create git tag if requested
    if (-not $NoTag) {
        Write-Info "Creating git tag v$newVersion-fork..."
        git tag -a "v$newVersion-fork" -m "Release $newVersion-fork"

        if ($LASTEXITCODE -eq 0) {
            Write-Success "Created tag v$newVersion-fork"
            Write-Info "Push with: git push origin $(git rev-parse --abbrev-ref HEAD) --tags"
        } else {
            Write-Error "Failed to create tag"
        }
    }
}

Write-Host ""
Write-Success "Version bump complete: $currentVersion -> $newVersion"
Write-Host ""
Write-Info "Next steps:"
Write-Host "  1. Review changes: git show HEAD"
Write-Host "  2. Push to remote: git push origin $(git rev-parse --abbrev-ref HEAD 2>$null)"
if (-not $NoTag) {
    Write-Host "  3. Push tags: git push origin --tags"
}
