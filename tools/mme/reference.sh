#!/usr/bin/env bash
# NMR's own MME export of the stock 1.11p world (the data file MMUD Explorer ships), the
# reference the MME export tests compare against. Downloaded once into the cache, checked
# against its pinned sha256, and printed as DAYDREAM_MME_REFERENCE=<path> (for $GITHUB_ENV).
set -euo pipefail

COMMIT=0ab268418370bba6b39cdc624774fb36e839caa3
SHA256=eba20f06af8dc08df755eb6dcd9763b023794f12e809cbab6762236d64d7345f
URL="https://raw.githubusercontent.com/jnfisher/MMUD-Explorer/${COMMIT}/data-v1.11p.mdb"

dir="${XDG_CACHE_HOME:-$HOME/.cache}/daydream/mme"
file="$dir/data-v1.11p-${SHA256:0:12}.mdb"
mkdir -p "$dir"
if ! echo "$SHA256  $file" | sha256sum -c --status 2>/dev/null; then
  curl -fsSL --retry 3 -o "$file.part" "$URL"
  echo "$SHA256  $file.part" | sha256sum -c --status || {
    echo "data-v1.11p.mdb does not match its pinned sha256" >&2
    exit 1
  }
  mv "$file.part" "$file"
fi
echo "DAYDREAM_MME_REFERENCE=$file"
