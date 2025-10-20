# Build wavesrv for Windows x64
$ErrorActionPreference = "Stop"

Set-Location -Path "D:\Code\agent-workspaces\agentx\waveterm"

# Refresh environment to get MinGW in PATH
Import-Module "$env:ChocolateyInstall\helpers\chocolateyProfile.psm1" -ErrorAction SilentlyContinue
refreshenv -ErrorAction SilentlyContinue

# Try to find Go installation
$goPaths = @(
    "C:\Program Files\Go\bin",
    "C:\Go\bin",
    "$env:USERPROFILE\go\bin",
    "$env:USERPROFILE\scoop\apps\go\current\bin"
)

$goExe = $null
foreach ($path in $goPaths) {
    $testPath = Join-Path $path "go.exe"
    if (Test-Path $testPath) {
        $goExe = $testPath
        Write-Host "Found Go at: $goExe"
        break
    }
}

if (-not $goExe) {
    # Try to find go.exe in PATH
    $goExe = (Get-Command go.exe -ErrorAction SilentlyContinue).Path
}

if (-not $goExe) {
    Write-Host "ERROR: Go not found. Please install Go from https://go.dev/dl/" -ForegroundColor Red
    exit 1
}

$env:CGO_ENABLED = "1"
$env:GOARCH = "amd64"
$zigPath = "D:\Code\agent-workspaces\agentx\waveterm\_temp\zig-windows-x86_64-0.13.0\zig.exe"
$env:CC = "$zigPath cc -target x86_64-windows-gnu"
$buildTime = Get-Date -Format "yyyyMMddHHmm"
$version = "0.12.0"

Write-Host "Building wavesrv.x64.exe with Zig..."
Write-Host "BuildTime: $buildTime"
Write-Host "Version: $version"
Write-Host "CC: $env:CC"

& $goExe build `
    -tags "osusergo,sqlite_omit_load_extension" `
    -ldflags "-X main.BuildTime=$buildTime -X main.WaveVersion=$version" `
    -o dist/bin/wavesrv.x64.exe `
    cmd/server/main-server.go

if ($LASTEXITCODE -eq 0) {
    Write-Host "Build successful!" -ForegroundColor Green
    Get-Item dist/bin/wavesrv.x64.exe | Select-Object Name, Length, LastWriteTime
} else {
    Write-Host "Build failed with exit code $LASTEXITCODE" -ForegroundColor Red
    exit 1
}
