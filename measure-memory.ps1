Write-Host "`nMeasuring Idle Memory" -ForegroundColor Cyan

# Launch
$proc = Start-Process -FilePath "src-tauri/target/release/wavemux.exe" -PassThru
Start-Sleep -Seconds 5

$ui = Get-Process wavemux -ErrorAction SilentlyContinue
$backend = Get-Process wavemuxsrv -ErrorAction SilentlyContinue

if ($ui -and $backend) {
    Write-Host "`nIdle State (5s after launch):" -ForegroundColor Green
    $memUI = [math]::Round($ui.WorkingSet64/1MB, 2)
    $memBackend = [math]::Round($backend.WorkingSet64/1MB, 2)
    $pmUI = [math]::Round($ui.PrivateMemorySize64/1MB, 2)
    $pmBackend = [math]::Round($backend.PrivateMemorySize64/1MB, 2)

    Write-Host "  UI (wavemux):       $memUI MB (WS), $pmUI MB (PM)"
    Write-Host "  Backend (wavemuxsrv): $memBackend MB (WS), $pmBackend MB (PM)"
    Write-Host "  Total:              $($memUI + $memBackend) MB (WS), $($pmUI + $pmBackend) MB (PM)" -ForegroundColor Cyan

    # Wait more for stabilization
    Start-Sleep -Seconds 10

    $ui = Get-Process wavemux -ErrorAction SilentlyContinue
    $backend = Get-Process wavemuxsrv -ErrorAction SilentlyContinue

    if ($ui -and $backend) {
        Write-Host "`nStable State (15s after launch):" -ForegroundColor Green
        $memUI = [math]::Round($ui.WorkingSet64/1MB, 2)
        $memBackend = [math]::Round($backend.WorkingSet64/1MB, 2)
        $pmUI = [math]::Round($ui.PrivateMemorySize64/1MB, 2)
        $pmBackend = [math]::Round($backend.PrivateMemorySize64/1MB, 2)

        Write-Host "  UI:     $memUI MB (WS), $pmUI MB (PM)"
        Write-Host "  Backend: $memBackend MB (WS), $pmBackend MB (PM)"
        Write-Host "  Total:  $($memUI + $memBackend) MB (WS), $($pmUI + $pmBackend) MB (PM)" -ForegroundColor Cyan
    }
}

# Cleanup
Get-Process wavemux* -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
