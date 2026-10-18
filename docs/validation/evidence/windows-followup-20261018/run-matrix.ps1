$ErrorActionPreference='Stop'
$evidence=Join-Path $PWD 'validation-artifacts/windows-followup-20261018'
$env:WDIO_APP_BINARY=Join-Path $PWD 'dist-portable/XArchive-0.2.1-dev-dc731cc-windows-x64-full/xarchive-desktop.exe'
$env:WDIO_DIRECT_DRIVER='1'
$env:WDIO_DIRECT_DRIVER_PORT='4447'
$env:WDIO_LOG_DIR=Join-Path $evidence 'matrix-final'
$env:TW2TG_GUI_MATRIX_ROOT=Join-Path $env:TEMP 'tw2tg-followup-20261018/matrix-canonical'
$env:TW2TG_MATRIX_DOWNLOADS='E:\Shiraishi\Downloads'
$env:XARCHIVE_PORTABLE_ROOT=Join-Path $env:TW2TG_GUI_MATRIX_ROOT 'profile'
$driver=Start-Process -FilePath (Join-Path $PWD 'validation-artifacts/windows-f3-20261018/driver-155/msedgedriver.exe') -ArgumentList '--port=4447','--allowed-ips=127.0.0.1' -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $evidence 'matrix-driver-final.log') -RedirectStandardError (Join-Path $evidence 'matrix-driver-final-err.log')
try {
 npm.cmd run test:e2e:windows --workspace desktop -- --spec ./e2e/specs/record-matrix.e2e.mjs *> (Join-Path $evidence 'matrix-final.log')
 $result=$LASTEXITCODE
} finally { if(Get-Process -Id $driver.Id -ErrorAction SilentlyContinue){Stop-Process -Id $driver.Id} }
Get-Content (Join-Path $evidence 'matrix-final.log') -Tail 26
exit $result
