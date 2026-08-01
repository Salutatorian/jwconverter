#!/usr/bin/env bash
# Fetch FFmpeg/FFprobe (and best-effort ImageMagick) for CI / local packaging.
# Usage: scripts/fetch-media-tools.sh [windows|macos|linux]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/src-tauri/binaries"
RES_MAGICK="$ROOT/src-tauri/resources/imagemagick"
OS="${1:-}"

POLICY_XML='<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE policymap [
<!ELEMENT policymap (policy)*>
<!ELEMENT policy EMPTY>
<!ATTLIST policy domain (delegate|coder|filter|path|resource) #IMPLIED
  name CDATA #IMPLIED pattern CDATA #IMPLIED rights CDATA #IMPLIED
  value CDATA #IMPLIED>
]>
<policymap>
  <policy domain="path" rights="none" pattern="@*"/>
</policymap>'

# Tauri requires resources/imagemagick/ to exist when listed in tauri.conf.json.
ensure_magick_resource_dir() {
  mkdir -p "$RES_MAGICK" "$BIN/imagemagick"
  if [[ ! -f "$RES_MAGICK/policy.xml" ]]; then
    printf '%s\n' "$POLICY_XML" > "$RES_MAGICK/policy.xml"
  fi
  if [[ ! -f "$BIN/imagemagick/policy.xml" ]]; then
    cp "$RES_MAGICK/policy.xml" "$BIN/imagemagick/policy.xml"
  fi
  if [[ ! -f "$RES_MAGICK/README.txt" ]]; then
    cat > "$RES_MAGICK/README.txt" <<'EOF'
ImageMagick portable tree for packaging.
Windows ships a full portable Magick tree.
On macOS CI we try to bundle magick + dylibs via dylibbundler.
On Linux CI Magick may still require a system install unless bundled.
EOF
  fi
}

# Download standalone yt-dlp for Links mode packaging (externalBin).
fetch_ytdlp() {
  local triple="$1"
  echo "Downloading yt-dlp for $triple..."
  curl -fsSL "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp" -o "$BIN/yt-dlp"
  chmod +x "$BIN/yt-dlp"
  cp "$BIN/yt-dlp" "$BIN/yt-dlp-$triple"
}

# Copy Homebrew magick and rewrite dylib refs to @executable_path/lib (portable).
bundle_magick_macos() {
  ensure_magick_resource_dir
  if ! command -v magick >/dev/null 2>&1; then
    brew install imagemagick || true
  fi
  if ! command -v magick >/dev/null 2>&1; then
    echo "WARN: magick not available — Images mode needs a system ImageMagick install."
    return 0
  fi

  MAGICK_BIN="$(command -v magick)"
  if command -v realpath >/dev/null 2>&1; then
    MAGICK_BIN="$(realpath "$MAGICK_BIN" || true)"
  fi
  if [[ -z "${MAGICK_BIN:-}" || ! -f "$MAGICK_BIN" ]]; then
    echo "WARN: could not resolve magick binary path."
    return 0
  fi

  rm -rf "$RES_MAGICK/lib" "$BIN/imagemagick/lib"
  mkdir -p "$RES_MAGICK/lib" "$BIN/imagemagick/lib"
  cp "$MAGICK_BIN" "$RES_MAGICK/magick"
  chmod +x "$RES_MAGICK/magick"

  if command -v dylibbundler >/dev/null 2>&1 || brew install dylibbundler; then
    # -od overwrite dest, -b bundle deps, -x binary, -d lib dir, -p install name prefix
    dylibbundler -od -b -x "$RES_MAGICK/magick" -d "$RES_MAGICK/lib" -p "@executable_path/lib/" \
      || echo "WARN: dylibbundler failed — Magick may still depend on Homebrew."
  else
    echo "WARN: dylibbundler unavailable — Magick may still depend on Homebrew."
  fi

  cp "$RES_MAGICK/magick" "$BIN/imagemagick/magick"
  chmod +x "$BIN/imagemagick/magick"
  if [[ -d "$RES_MAGICK/lib" ]]; then
    cp -R "$RES_MAGICK/lib/." "$BIN/imagemagick/lib/" 2>/dev/null || true
  fi

  # Prefer a fully portable Magick; warn (do not fail the build) if Cellar links remain.
  if command -v otool >/dev/null 2>&1; then
    if otool -L "$RES_MAGICK/magick" 2>/dev/null | grep -q '/opt/homebrew/Cellar\|/usr/local/Cellar'; then
      echo "WARN: bundled magick still links Homebrew Cellar — Images may need system Magick."
      otool -L "$RES_MAGICK/magick" || true
    else
      echo "OK: magick dylibs look portable."
    fi
  fi
}

if [[ -z "$OS" ]]; then
  case "$(uname -s)" in
    Darwin) OS=macos ;;
    Linux) OS=linux ;;
    MINGW*|MSYS*|CYGWIN*|Windows_NT) OS=windows ;;
    *) echo "Unknown OS; pass windows|macos|linux"; exit 1 ;;
  esac
fi

mkdir -p "$BIN"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "==> Fetching media tools for $OS"

case "$OS" in
  windows)
    echo "Windows: place Gyan ffmpeg + portable Magick manually (see binaries/README.md)."
    exit 0
    ;;
  macos)
    TRIPLE="aarch64-apple-darwin"
    if [[ "$(uname -m)" == "x86_64" ]]; then
      TRIPLE="x86_64-apple-darwin"
    fi

    # Static/universal builds from evermeet — no Homebrew dylib deps.
    echo "Downloading static FFmpeg/FFprobe from evermeet.cx..."
    curl -fsJL "https://evermeet.cx/ffmpeg/getrelease/ffmpeg/zip" -o "$TMP/ffmpeg.zip"
    curl -fsJL "https://evermeet.cx/ffmpeg/getrelease/ffprobe/zip" -o "$TMP/ffprobe.zip"
    unzip -qo "$TMP/ffmpeg.zip" -d "$TMP/ffmpeg"
    unzip -qo "$TMP/ffprobe.zip" -d "$TMP/ffprobe"
    FFMPEG_SRC="$(find "$TMP/ffmpeg" -type f -name ffmpeg | head -n1)"
    FFPROBE_SRC="$(find "$TMP/ffprobe" -type f -name ffprobe | head -n1)"
    if [[ -z "$FFMPEG_SRC" || -z "$FFPROBE_SRC" ]]; then
      echo "ERROR: evermeet zip did not contain ffmpeg/ffprobe"
      exit 1
    fi
    cp "$FFMPEG_SRC" "$BIN/ffmpeg"
    cp "$FFPROBE_SRC" "$BIN/ffprobe"
    chmod +x "$BIN/ffmpeg" "$BIN/ffprobe"
    # Tauri externalBin expects triple-suffixed names at build time.
    cp "$BIN/ffmpeg" "$BIN/ffmpeg-$TRIPLE"
    cp "$BIN/ffprobe" "$BIN/ffprobe-$TRIPLE"
    cp "$BIN/ffmpeg" "$BIN/ffmpeg-aarch64-apple-darwin"
    cp "$BIN/ffprobe" "$BIN/ffprobe-aarch64-apple-darwin"
    cp "$BIN/ffmpeg" "$BIN/ffmpeg-x86_64-apple-darwin"
    cp "$BIN/ffprobe" "$BIN/ffprobe-x86_64-apple-darwin"

    if command -v otool >/dev/null 2>&1; then
      if otool -L "$BIN/ffmpeg" 2>/dev/null | grep -q '/opt/homebrew/Cellar\|/usr/local/Cellar'; then
        echo "ERROR: ffmpeg still links Homebrew Cellar — not a static build."
        otool -L "$BIN/ffmpeg" || true
        exit 1
      fi
    fi
    # Sanity: static builds are multi‑MB, not ~400KB Homebrew stubs.
    FFMPEG_SIZE="$(wc -c < "$BIN/ffmpeg" | tr -d ' ')"
    if [[ "$FFMPEG_SIZE" -lt 5000000 ]]; then
      echo "ERROR: ffmpeg size ${FFMPEG_SIZE} looks too small for a static build."
      exit 1
    fi

    bundle_magick_macos
    fetch_ytdlp "$TRIPLE"
    ;;
  linux)
    TRIPLE="x86_64-unknown-linux-gnu"
    # Prefer BtbN (GitHub CDN) — johnvansickle.com intermittently returns HTML
    # and breaks CI with `xz: File format not recognized`.
    FFMPEG_URL="https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-linux64-gpl.tar.xz"
    echo "Downloading static FFmpeg/FFprobe from BtbN..."
    if ! curl -fsSL "$FFMPEG_URL" -o "$TMP/ffmpeg.tar.xz"; then
      echo "WARN: BtbN download failed; falling back to johnvansickle.com"
      curl -fsSL "https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz" -o "$TMP/ffmpeg.tar.xz"
    fi
    # Reject HTML/error bodies masquerading as archives.
    if ! xz -t "$TMP/ffmpeg.tar.xz" 2>/dev/null; then
      echo "ERROR: downloaded FFmpeg archive is not valid xz (upstream may have returned HTML)."
      file "$TMP/ffmpeg.tar.xz" || true
      head -c 200 "$TMP/ffmpeg.tar.xz" || true
      exit 1
    fi
    tar -xJf "$TMP/ffmpeg.tar.xz" -C "$TMP"
    FFMPEG_SRC="$(find "$TMP" -type f -name ffmpeg | head -n1)"
    FFPROBE_SRC="$(find "$TMP" -type f -name ffprobe | head -n1)"
    if [[ -z "$FFMPEG_SRC" || -z "$FFPROBE_SRC" ]]; then
      echo "ERROR: archive did not contain ffmpeg/ffprobe binaries"
      exit 1
    fi
    cp "$FFMPEG_SRC" "$BIN/ffmpeg"
    cp "$FFPROBE_SRC" "$BIN/ffprobe"
    chmod +x "$BIN/ffmpeg" "$BIN/ffprobe"
    cp "$BIN/ffmpeg" "$BIN/ffmpeg-$TRIPLE"
    cp "$BIN/ffprobe" "$BIN/ffprobe-$TRIPLE"

    ensure_magick_resource_dir
    if command -v magick >/dev/null 2>&1; then
      MAGICK_BIN="$(command -v magick)"
      cp "$MAGICK_BIN" "$BIN/imagemagick/magick"
      cp "$MAGICK_BIN" "$RES_MAGICK/magick"
      chmod +x "$BIN/imagemagick/magick" "$RES_MAGICK/magick"
      echo "WARN: Linux Magick is copied from PATH and may need system libs in AppImage."
    else
      echo "WARN: magick not on PATH — Images mode may need a system ImageMagick install."
    fi
    fetch_ytdlp "$TRIPLE"
    ;;
  *)
    echo "Unsupported OS: $OS"
    exit 1
    ;;
esac

echo "==> Done. Binaries in $BIN"
ls -la "$BIN" | head -n 40
ls -la "$RES_MAGICK" 2>/dev/null | head -n 20 || true
