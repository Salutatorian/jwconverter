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

### Downloads
- **Windows:** signed installer below (recommended)
- **macOS:** `.dmg` for Apple Silicon (unsigned) — attached by CI; if missing, wait a few minutes for the Build workflow, then refresh. First open: right-click → **Open** (Gatekeeper)
- **Linux:** AppImage when CI succeeds

From v0.5.2 onward, Windows installs auto-update on launch when a newer release exists (full-screen progress). You can still use **Settings → Updates** manually.

### What's new in 1.0.1
- **Links multi-downloads** (playlists or pasted batches) finish as **one .zip** in your destination folder — no loose file spam
- Single-URL downloads still save as one media file

### Notes
- Windows 10/11 x64 + WebView2
- Sources are never modified
- Bundled FFmpeg/FFprobe — see third_party/FFMPEG-LICENSING.md
- Bundled yt-dlp — see third_party/THIRD_PARTY_YTDLP.txt
- macOS not notarized yet
- Images on a clean Mac may still need system Magick if Magick dylib bundling is incomplete
- Links: DRM / private videos need your own cookies.txt; live caps 5–60 min
"@

gh release create $tag $uploadSetup $latestPath `
  --repo $Repo `
  --title "JW Converter v$Version" `
  --notes $notes

Write-Host "Published https://github.com/$Repo/releases/tag/$tag"
