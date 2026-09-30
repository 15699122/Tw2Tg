param(
  [Parameter(Mandatory = $true)]
  [string]$Executable,

  [Parameter(Mandatory = $true)]
  [string]$OutputDirectory
)

$ErrorActionPreference = "Continue"
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null

function Write-CapturedFile {
  param(
    [string]$Path,
    [scriptblock]$Command
  )
  try {
    & $Command *>&1 | Out-File -FilePath $Path -Encoding utf8
  } catch {
    $_ | Out-File -FilePath $Path -Encoding utf8
  }
}

$metadata = [ordered]@{
  timestamp_utc = [DateTime]::UtcNow.ToString("o")
  executable = [System.IO.Path]::GetFullPath($Executable)
  executable_exists = Test-Path -LiteralPath $Executable
  windows_product_name = $null
  windows_display_version = $null
  node = $null
  npm = $null
  rustc = $null
  cargo = $null
  tauri_driver = $null
  msedgedriver = $null
  msedgedriver_banner_accepted = $null
  msedgedriver_banner_reason = $null
  tauri_driver_path = $null
  msedgedriver_path = $null
  tauri_driver_sha256 = $null
  msedgedriver_sha256 = $null
  edgedriver_version_expected = $env:EDGEDRIVER_VERSION
  tauri_driver_version_expected = $env:TAURI_DRIVER_VERSION
  webview2_registry = @()
}

try {
  $os = Get-ComputerInfo -Property WindowsProductName, WindowsDisplayVersion
  $metadata.windows_product_name = $os.WindowsProductName
  $metadata.windows_display_version = $os.WindowsDisplayVersion
} catch { }

foreach ($tool in @("node", "npm", "rustc", "cargo", "tauri-driver", "msedgedriver")) {
  try {
    $command = Get-Command $tool -ErrorAction Stop
    $version = & $command.Source --version 2>&1 | Select-Object -First 1
    $metadata[$tool.Replace('-', '_')] = [string]$version
  } catch {
    $metadata[$tool.Replace('-', '_')] = "NOT_FOUND"
  }
}

foreach ($toolProperty in @(
  @{ name = "tauri-driver"; property = "tauri_driver_path"; hash = "tauri_driver_sha256" },
  @{ name = "msedgedriver"; property = "msedgedriver_path"; hash = "msedgedriver_sha256" }
)) {
  try {
    $command = Get-Command $toolProperty.name -ErrorAction Stop
    $metadata[$toolProperty.property] = $command.Source
    $metadata[$toolProperty.hash] = (Get-FileHash $command.Source -Algorithm SHA256).Hash
  } catch { }
}

try {
  $edgeCommand = Get-Command msedgedriver.exe -ErrorAction Stop
  $edgeBanner = (& $edgeCommand.Source --version 2>&1 | Out-String).Trim()
  $expectedVersion = $env:EDGEDRIVER_VERSION
  $versionMatch = if ([string]::IsNullOrWhiteSpace($expectedVersion)) {
    $edgeBanner -match '(MSEdgeDriver|Microsoft Edge WebDriver)\s+([\d.]+)'
  } else {
    $edgeBanner -match [regex]::Escape($expectedVersion)
  }
  $bannerMatch = $edgeBanner -match '(MSEdgeDriver|Microsoft Edge WebDriver)\s+([\d.]+)'
  $metadata.msedgedriver_banner_accepted = [bool]($versionMatch -and $bannerMatch)
  if ($metadata.msedgedriver_banner_accepted) {
    $metadata.msedgedriver_banner_reason = "banner recognized: $edgeBanner"
  } else {
    $metadata.msedgedriver_banner_reason = "banner did not match @wdio/tauri-service 1.4.0 discovery: $edgeBanner"
  }
} catch {
  $metadata.msedgedriver_banner_accepted = $false
  $metadata.msedgedriver_banner_reason = "msedgedriver --version unavailable: $_"
}

try {
  $metadata.webview2_registry = @(
    Get-ItemProperty "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\*" -ErrorAction SilentlyContinue |
      Where-Object { $_.name -match "WebView2|Edge" } |
      Select-Object PSPath, name, pv
  )
} catch { }

$metadata | ConvertTo-Json -Depth 6 | Out-File (Join-Path $OutputDirectory "environment.json") -Encoding utf8
Write-CapturedFile (Join-Path $OutputDirectory "processes-before.txt") { Get-Process xarchive-desktop, msedgewebview2, tauri-driver, msedgedriver -ErrorAction SilentlyContinue | Select-Object Id, ProcessName, Path }
Write-CapturedFile (Join-Path $OutputDirectory "ports-before.txt") { Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue | Where-Object { $_.LocalPort -in 1420, 4444, 4445, 9223 } | Select-Object LocalAddress, LocalPort, OwningProcess }

if (-not $metadata.executable_exists) {
  "executable missing: $Executable" | Out-File (Join-Path $OutputDirectory "preflight-error.txt") -Encoding utf8
  exit 2
}

$process = $null
try {
  $process = Start-Process -FilePath $Executable -WorkingDirectory (Split-Path -Parent $Executable) -PassThru
  Start-Sleep -Seconds 10
  $process.Refresh()
  if ($process.HasExited) {
    "application exited during direct startup; exit_code=$($process.ExitCode)" | Out-File (Join-Path $OutputDirectory "preflight-error.txt") -Encoding utf8
    exit 3
  }
  "application remained alive for 10 seconds; pid=$($process.Id)" | Out-File (Join-Path $OutputDirectory "direct-startup.txt") -Encoding utf8
} catch {
  $_ | Out-File (Join-Path $OutputDirectory "preflight-error.txt") -Encoding utf8
  exit 4
} finally {
  if ($null -ne $process -and -not $process.HasExited) {
    & taskkill.exe /PID $process.Id /T /F 2>&1 | Out-File (Join-Path $OutputDirectory "direct-startup-cleanup.txt") -Encoding utf8
  }
}

Write-CapturedFile (Join-Path $OutputDirectory "processes-after.txt") { Get-Process xarchive-desktop, msedgewebview2, tauri-driver, msedgedriver -ErrorAction SilentlyContinue | Select-Object Id, ProcessName, Path }
Write-CapturedFile (Join-Path $OutputDirectory "ports-after.txt") { Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue | Where-Object { $_.LocalPort -in 1420, 4444, 4445, 9223 } | Select-Object LocalAddress, LocalPort, OwningProcess }
exit 0