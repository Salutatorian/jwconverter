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
On macOS/Linux CI this may be a stub or a copied system `magick` binary.
Windows ships a full portable Magick tree.
EOF
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

    if ! command -v ffmpeg >/dev/null 2>&1 || ! command -v ffprobe >/dev/null 2>&1; then
      brew install ffmpeg
    fi
    cp "$(command -v ffmpeg)" "$BIN/ffmpeg"
    cp "$(command -v ffprobe)" "$BIN/ffprobe"
    chmod +x "$BIN/ffmpeg" "$BIN/ffprobe"
    cp "$BIN/ffmpeg" "$BIN/ffmpeg-$TRIPLE"
    cp "$BIN/ffprobe" "$BIN/ffprobe-$TRIPLE"

    ensure_magick_resource_dir
    if command -v magick >/dev/null 2>&1; then
      MAGICK_BIN="$(command -v magick)"
      # Resolve symlinks so we copy a real file when possible.
      if command -v realpath >/dev/null 2>&1; then
        MAGICK_BIN="$(realpath "$MAGICK_BIN" || true)"
      fi
      if [[ -n "${MAGICK_BIN:-}" && -f "$MAGICK_BIN" ]]; then
        cp "$MAGICK_BIN" "$BIN/imagemagick/magick"
        cp "$MAGICK_BIN" "$RES_MAGICK/magick"
        chmod +x "$BIN/imagemagick/magick" "$RES_MAGICK/magick"
      fi
    else
      echo "WARN: magick not on PATH — Images mode may need a system ImageMagick install."
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

    ensure_magick_resource_dir
    if command -v magick >/dev/null 2>&1; then
      MAGICK_BIN="$(command -v magick)"
      cp "$MAGICK_BIN" "$BIN/imagemagick/magick"
      cp "$MAGICK_BIN" "$RES_MAGICK/magick"
      chmod +x "$BIN/imagemagick/magick" "$RES_MAGICK/magick"
    else
      echo "WARN: magick not on PATH — Images mode may need a system ImageMagick install."
    fi
    ;;
  *)
    echo "Unsupported OS: $OS"
    exit 1
    ;;
esac

echo "==> Done. Binaries in $BIN"
ls -la "$BIN" | head -n 40
ls -la "$RES_MAGICK" | head -n 20
