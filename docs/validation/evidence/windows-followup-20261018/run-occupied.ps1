$ErrorActionPreference='Stop'
$evidence=Join-Path $PWD 'validation-artifacts/windows-followup-20261018'
$env:WDIO_APP_BINARY=Join-Path $PWD 'dist-portable/XArchive-0.2.1-dev-dc731cc-windows-x64-full/xarchive-desktop.exe'
$env:TAURI_DRIVER_PATH='C:\Users\Shiraishi\.cargo\bin\tauri-driver.exe'
$env:EDGEDRIVER_PATH=Join-Path $PWD 'validation-artifacts/windows-f3-20261018/driver-155/msedgedriver.exe'
$env:TAURI_DRIVER_EDGE_VERSION='155.0.4283.45'
$env:WDIO_AUTO_INSTALL_TAURI_DRIVER='0'
$env:WDIO_AUTO_DOWNLOAD_EDGE_DRIVER='0'
$env:WDIO_LOG_DIR=Join-Path $evidence 'occupied-final'
$env:XARCHIVE_PORTABLE_ROOT=Join-Path $env:TEMP 'tw2tg-followup-20261018/occupied-final-profile'
$listener=[System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::IPv6Any,4445)
$listener.Server.DualMode=$true
$listener.Start()
$fixturePid=$PID
try {
 npm.cmd run test:e2e:windows --workspace desktop *> (Join-Path $evidence 'occupied-final.log')
 $wdioExit=$LASTEXITCODE
 $owners=@(Get-NetTCPConnection -LocalPort 4445 -State Listen -ErrorAction SilentlyContinue | Select-Object -ExpandProperty OwningProcess)
 $remaining=@(Get-CimInstance Win32_Process | Where-Object Name -in 'tauri-driver.exe','msedgedriver.exe' | Select-Object ProcessId,ParentProcessId,ExecutablePath)
 $result=@{fixturePid=$fixturePid;fixtureAlive=[bool](Get-Process -Id $fixturePid -ErrorAction SilentlyContinue);fixtureOwnsListener=$owners.Contains([uint32]$fixturePid);wdioExit=$wdioExit;remainingDrivers=$remaining;source=(git rev-parse HEAD)}
 $result | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $evidence 'occupied-final.json')
 if(!$result.fixtureAlive -or !$result.fixtureOwnsListener -or $remaining.Count -ne 0){throw 'driver ownership cleanup acceptance failed'}
} finally { $listener.Stop() }
Get-Content (Join-Path $evidence 'occupied-final.json')
