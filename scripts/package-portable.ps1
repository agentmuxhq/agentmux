Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Set-Location "C:\Users\asafe\.claw\agentx-workspace\agentmux"
$VERSION = (Get-Content package.json | ConvertFrom-Json).version
$DESKTOP = [Environment]::GetFolderPath("Desktop")
$PORTABLE = Join-Path $DESKTOP "agentmux-cef-$VERSION-x64-portable-new"
$ZIPPATH = Join-Path $DESKTOP "agentmux-cef-$VERSION-x64-portable-new.zip"

Write-Host "Packaging AgentMux CEF v$VERSION"

if (Test-Path $PORTABLE) { Remove-Item $PORTABLE -Recurse -Force }
if (Test-Path $ZIPPATH) { Remove-Item $ZIPPATH -Force }

New-Item -ItemType Directory -Force -Path "$PORTABLE\runtime\locales","$PORTABLE\runtime\frontend" | Out-Null

Copy-Item "target\release\agentmux-launcher.exe" "$PORTABLE\agentmux.exe"
Copy-Item "target\release\agentmux-cef.exe" "$PORTABLE\runtime\"
Copy-Item "dist\bin\agentmuxsrv-rs.x64.exe" "$PORTABLE\runtime\"

$WSH = "dist\bin\wsh-$VERSION-windows.x64.exe"
if (Test-Path $WSH) { Copy-Item $WSH "$PORTABLE\runtime\wsh.exe" }

Copy-Item "dist\frontend\*" "$PORTABLE\runtime\frontend" -Recurse
Copy-Item "dist\cef\libcef.dll" "$PORTABLE\runtime\"
Get-ChildItem "dist\cef\" -Filter "*.dll" | Where-Object { $_.Name -ne "libcef.dll" } | ForEach-Object { Copy-Item $_.FullName "$PORTABLE\runtime\" }
Get-ChildItem "dist\cef\" -Filter "*.dat" | ForEach-Object { Copy-Item $_.FullName "$PORTABLE\runtime\" }
Get-ChildItem "dist\cef\" -Filter "*.bin" | ForEach-Object { Copy-Item $_.FullName "$PORTABLE\runtime\" }
Get-ChildItem "dist\cef\" -Filter "*.pak" | ForEach-Object { Copy-Item $_.FullName "$PORTABLE\runtime\" }
if (Test-Path "dist\cef\locales\en-US.pak") { Copy-Item "dist\cef\locales\en-US.pak" "$PORTABLE\runtime\locales\" }

Set-Content "$PORTABLE\README.txt" "AgentMux v$VERSION Portable - Run agentmux.exe"

$size = [math]::Round((Get-ChildItem $PORTABLE -Recurse | Measure-Object -Property Length -Sum).Sum / 1MB, 1)
Write-Host "Directory: $size MB"

Compress-Archive -Path "$PORTABLE\*" -DestinationPath $ZIPPATH -Force
$zipSize = [math]::Round((Get-Item $ZIPPATH).Length / 1MB, 1)
Write-Host "[SUCCESS] v$VERSION - Dir: $PORTABLE - ZIP: $zipSize MB"
