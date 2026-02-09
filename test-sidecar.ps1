$env:WAVETERM_AUTH_KEY = "test-key-123"
$env:WAVETERM_DEV = ""
$env:WAVETERM_DATA_HOME = "$env:LOCALAPPDATA\wavemux-test"
$env:WAVETERM_CONFIG_HOME = "$env:LOCALAPPDATA\wavemux-test"

New-Item -ItemType Directory -Force -Path $env:WAVETERM_DATA_HOME | Out-Null

Write-Host "Testing sidecar binary..." -ForegroundColor Cyan
$proc = Start-Process -FilePath "src-tauri/binaries/wavemuxsrv-x86_64-pc-windows-msvc.exe" -ArgumentList "--wavedata", $env:WAVETERM_DATA_HOME -PassThru -NoNewWindow

Start-Sleep -Seconds 3

if (Get-Process -Id $proc.Id -ErrorAction SilentlyContinue) {
    Write-Host "✓ Sidecar started successfully!" -ForegroundColor Green
    Write-Host "  PID: $($proc.Id)"
    $mem = [math]::Round((Get-Process -Id $proc.Id).WorkingSet64/1MB, 2)
    Write-Host "  Memory: $mem MB"

    # Kill it
    Stop-Process -Id $proc.Id -Force
    Write-Host "✓ Stopped" -ForegroundColor Yellow
} else {
    Write-Host "✗ Sidecar failed to start or crashed" -ForegroundColor Red
}
