$ErrorActionPreference='Stop'
$evidence=Join-Path $PWD 'validation-artifacts/windows-f3-20261018'
$env:WDIO_APP_BINARY=Join-Path $PWD 'dist-portable/XArchive-0.2.1-dev-dc731cc-windows-x64-full/xarchive-desktop.exe'
$env:WDIO_DIRECT_DRIVER='1'
$env:WDIO_DIRECT_DRIVER_PORT='4447'
$env:WDIO_F3_ARCHIVE='1'
$env:WDIO_LOG_DIR=Join-Path $evidence 'wdio-final'
$env:WDIO_AUTO_DOWNLOAD_EDGE_DRIVER='0'
$env:EDGEDRIVER_PATH=Join-Path $evidence 'driver-155/msedgedriver.exe'
$env:XARCHIVE_PORTABLE_ROOT=Join-Path $env:TEMP 'tw2tg-windows-f3-20261018/wdio-final-profile'
$driver=Start-Process -FilePath $env:EDGEDRIVER_PATH -ArgumentList '--port=4447','--allowed-ips=127.0.0.1' -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $evidence 'full-driver-stdout.log') -RedirectStandardError (Join-Path $evidence 'full-driver-stderr.log')
for ($attempt=0; $attempt -lt 30; $attempt++) {
 try { Invoke-RestMethod 'http://127.0.0.1:4447/status' -TimeoutSec 1 | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $evidence 'full-driver-ready.json'); break } catch { Start-Sleep -Milliseconds 200 }
}
try {
 npm.cmd run test:e2e:windows --workspace desktop *> (Join-Path $evidence 'wdio-final.log')
 $result=$LASTEXITCODE
} finally {
 if (Get-Process -Id $driver.Id -ErrorAction SilentlyContinue) { Stop-Process -Id $driver.Id -Force }
}
Get-Content (Join-Path $evidence 'wdio-final.log') -Tail 32
exit $result
