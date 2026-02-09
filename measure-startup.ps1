param([int]$Runs = 5)

Write-Host "`nMeasuring Startup Time (Backend Spawn)" -ForegroundColor Cyan
$times = @()
$exePath = "src-tauri/target/release/wavemux.exe"

for ($i = 1; $i -le $Runs; $i++) {
    Write-Host "Run $i/$Runs... " -NoNewline

    # Clean
    Get-Process wavemux* -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500

    # Measure
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $proc = Start-Process -FilePath $exePath -PassThru

    # Wait for backend
    $timeout = 10
    $elapsed = 0
    $started = $false

    while ($elapsed -lt $timeout) {
        Start-Sleep -Milliseconds 100
        $elapsed += 0.1

        $backend = Get-Process wavemuxsrv -ErrorAction SilentlyContinue
        if ($backend) {
            $started = $true
            break
        }
    }

    $sw.Stop()
    $time = $sw.Elapsed.TotalMilliseconds

    if ($started) {
        $times += $time
        Write-Host "$([math]::Round($time, 2))ms" -ForegroundColor Green
    } else {
        Write-Host "FAILED" -ForegroundColor Red
    }

    # Cleanup
    Get-Process wavemux* -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500
}

if ($times.Count -gt 0) {
    Write-Host "`nResults:" -ForegroundColor Cyan
    Write-Host "  Average: $([math]::Round(($times | Measure-Object -Average).Average, 2))ms"
    Write-Host "  Median:  $([math]::Round(($times | Sort-Object)[[math]::Floor($times.Count / 2)], 2))ms"
    Write-Host "  Min:     $([math]::Round(($times | Measure-Object -Minimum).Minimum, 2))ms"
    Write-Host "  Max:     $([math]::Round(($times | Measure-Object -Maximum).Maximum, 2))ms"
}
