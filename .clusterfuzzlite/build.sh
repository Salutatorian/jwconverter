#!/bin/bash -eu

# Fenrir/ClusterFuzzLite build entrypoint.
# The repo is mounted at $SRC; a single wrapping top-level folder may be present.

ROOT="$SRC"
if [ ! -d "$ROOT/src-tauri/fuzz" ]; then
  for d in "$ROOT"/*/; do
    if [ -d "${d}src-tauri/fuzz" ]; then
      ROOT="${d%/}"
      break
    fi
  done
fi

cd "$ROOT/src-tauri/fuzz"

cargo fuzz build -O

RELEASE_DIR="target/x86_64-unknown-linux-gnu/release"
for target in progress_fuzzer path_fuzzer loudness_fuzzer; do
  cp "$RELEASE_DIR/$target" "$OUT/"
  if [ -d "corpus/$target" ]; then
    zip -q -j "$OUT/${target}_seed_corpus.zip" "corpus/$target"/*
  fi
done
