param(
  [Parameter(Mandatory = $true)]
  [string]$Version,

  [string]$Repo = "Salutatorian/jwconverter"
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$nsisDir = Join-Path $root "src-tauri\target\release\bundle\nsis"
$setupName = "JW Converter_${Version}_x64-setup.exe"
$setupPath = Join-Path $nsisDir $setupName
$sigPath = "$setupPath.sig"

if (-not (Test-Path $setupPath)) {
  throw "Missing installer: $setupPath - run a signed tauri build first."
}
if (-not (Test-Path $sigPath)) {
  throw "Missing signature: $sigPath - set TAURI_SIGNING_PRIVATE_KEY* and rebuild."
}

$signature = (Get-Content -Raw $sigPath).Trim()
$assetName = "JW.Converter_${Version}_x64-setup.exe"
$downloadUrl = "https://github.com/$Repo/releases/download/v$Version/$assetName"

$latest = [ordered]@{
  version = $Version
  notes = "JW Converter v$Version"
  pub_date = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
  platforms = @{
    "windows-x86_64" = @{
      signature = $signature
      url = $downloadUrl
    }
  }
}

$latestPath = Join-Path $nsisDir "latest.json"
$json = $latest | ConvertTo-Json -Depth 6
[System.IO.File]::WriteAllText($latestPath, $json)

# Upload with a stable no-space asset name so the latest.json URL stays predictable.
$uploadSetup = Join-Path $nsisDir $assetName
Copy-Item -Force $setupPath $uploadSetup

$tag = "v$Version"
$notes = @"
## JW Converter v$Version

Download the Windows installer below. From v0.1.2 onward, the app can check GitHub Releases and install updates when you click **Update**.

### What's new
- Full UI revamp — cobalt-style left rail (Audio / Images / Settings)
- Near-black canvas, pill controls, monospace chrome
- Attachment file queues kept for batch status + retry

### Notes
- Windows 10/11 x64 + WebView2
- Sources are never modified
- Bundled FFmpeg/FFprobe — see docs/ffmpeg-licensing.md
- macOS CI builds are unsigned (Gatekeeper may require right-click → Open)
"@

gh release create $tag $uploadSetup $latestPath `
  --repo $Repo `
  --title "JW Converter v$Version" `
  --notes $notes

Write-Host "Published https://github.com/$Repo/releases/tag/$tag"
