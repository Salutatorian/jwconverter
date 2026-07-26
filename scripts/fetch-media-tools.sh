#!/usr/bin/env bash
# Fetch FFmpeg/FFprobe (and best-effort ImageMagick) for CI / local packaging.
# Usage: scripts/fetch-media-tools.sh [windows|macos|linux]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/src-tauri/binaries"
RES_MAGICK="$ROOT/src-tauri/resources/imagemagick"
OS="${1:-}"

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
    echo "CI Windows job should cache or restore committed packaging layout."
    exit 0
    ;;
  macos)
    TRIPLE="aarch64-apple-darwin"
    if [[ "$(uname -m)" == "x86_64" ]]; then
      TRIPLE="x86_64-apple-darwin"
    fi

    if ! command -v ffmpeg >/dev/null 2>&1 || ! command -v ffprobe >/dev/null 2>&1; then
      brew install ffmpeg
    fi
    cp "$(command -v ffmpeg)" "$BIN/ffmpeg"
    cp "$(command -v ffprobe)" "$BIN/ffprobe"
    chmod +x "$BIN/ffmpeg" "$BIN/ffprobe"
    cp "$BIN/ffmpeg" "$BIN/ffmpeg-$TRIPLE"
    cp "$BIN/ffprobe" "$BIN/ffprobe-$TRIPLE"

    if command -v magick >/dev/null 2>&1; then
      MAGICK_BIN="$(command -v magick)"
      mkdir -p "$BIN/imagemagick" "$RES_MAGICK"
      cp "$MAGICK_BIN" "$BIN/imagemagick/magick"
      cp "$MAGICK_BIN" "$RES_MAGICK/magick"
      chmod +x "$BIN/imagemagick/magick" "$RES_MAGICK/magick"
      if [[ ! -f "$RES_MAGICK/policy.xml" ]]; then
        cat > "$RES_MAGICK/policy.xml" <<'XML'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE policymap [
<!ELEMENT policymap (policy)*>
<!ELEMENT policy EMPTY>
<!ATTLIST policy domain (delegate|coder|filter|path|resource) #IMPLIED
  name CDATA #IMPLIED pattern CDATA #IMPLIED rights CDATA #IMPLIED
  value CDATA #IMPLIED>
]>
<policymap>
  <policy domain="path" rights="none" pattern="@*"/>
</policymap>
XML
        cp "$RES_MAGICK/policy.xml" "$BIN/imagemagick/policy.xml"
      fi
    else
      echo "WARN: magick not on PATH — image conversion may be unavailable in this build."
    fi
    ;;
  linux)
    TRIPLE="x86_64-unknown-linux-gnu"
    curl -fsSL "https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz" -o "$TMP/ffmpeg.tar.xz"
    tar -xJf "$TMP/ffmpeg.tar.xz" -C "$TMP"
    SRC="$(find "$TMP" -maxdepth 1 -type d -name 'ffmpeg-*-amd64-static' | head -n1)"
    cp "$SRC/ffmpeg" "$BIN/ffmpeg"
    cp "$SRC/ffprobe" "$BIN/ffprobe"
    chmod +x "$BIN/ffmpeg" "$BIN/ffprobe"
    cp "$BIN/ffmpeg" "$BIN/ffmpeg-$TRIPLE"
    cp "$BIN/ffprobe" "$BIN/ffprobe-$TRIPLE"

    if command -v magick >/dev/null 2>&1; then
      MAGICK_BIN="$(command -v magick)"
      mkdir -p "$BIN/imagemagick" "$RES_MAGICK"
      cp "$MAGICK_BIN" "$BIN/imagemagick/magick"
      cp "$MAGICK_BIN" "$RES_MAGICK/magick"
      chmod +x "$BIN/imagemagick/magick" "$RES_MAGICK/magick"
    elif command -v convert >/dev/null 2>&1; then
      echo "WARN: found ImageMagick 6 convert; prefer ImageMagick 7 magick."
    else
      echo "WARN: magick not on PATH — image conversion may be unavailable in this build."
    fi
    ;;
  *)
    echo "Unsupported OS: $OS"
    exit 1
    ;;
esac

echo "==> Done. Binaries in $BIN"
ls -la "$BIN" | head -n 40
