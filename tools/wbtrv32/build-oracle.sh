#!/usr/bin/env bash
# Builds wbtrv32's own DAT-to-SQLite converter (util/DatabaseConverter.cc), the oracle the
# store's differential tests compare Daydream's conversions against.
#
# The checkout and the binary go in a cache directory keyed by the pinned commit, so a
# second run (or a CI run with the directory restored from its cache) does nothing:
#   ${DAYDREAM_WBTRV32_CACHE:-~/.cache/daydream/wbtrv32}/<commit>/
# Prints two lines for $GITHUB_ENV / eval:
#   DAYDREAM_WBTRV32_DIR=<checkout>  DAYDREAM_WBTRV32_CONVERTER=<binary>
set -euo pipefail

# mbbsemu/wbtrv32 at the commit the store's key code was ported from (UPSTREAM.md).
COMMIT=4a439579952c51c59a61f0fa7d39f0c26681508f
REPO=https://github.com/mbbsemu/wbtrv32

cache="${DAYDREAM_WBTRV32_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}/daydream/wbtrv32}/$COMMIT"
src="$cache/src"
bin="$cache/database_converter"

if [[ ! -x "$bin" ]]; then
  mkdir -p "$cache"
  if [[ ! -d "$src/.git" ]]; then
    rm -rf "$src"
    git init -q "$src"
    git -C "$src" remote add origin "$REPO"
    git -C "$src" fetch -q --depth 1 origin "$COMMIT"
    git -C "$src" checkout -q FETCH_HEAD
  fi
  obj="$cache/obj"
  mkdir -p "$obj"
  cc -O2 -c "$src/sqlite/sqlite3.c" -o "$obj/sqlite3.o" -DSQLITE_THREADSAFE=1
  # _FORTIFY_SOURCE off: upstream's toWideString passes mbstowcs a byte count as the wide
  # buffer length, which glibc's fortified build aborts on (harmless on Windows).
  srcs=()
  for f in "$src"/btrieve/*.cc; do
    case "$f" in *_test.cc|*/TestBase.cc) continue ;; esac
    srcs+=("$f")
  done
  c++ -std=c++20 -O2 -U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=0 -I"$src" \
    "${srcs[@]}" "$src/util/DatabaseConverter.cc" "$obj/sqlite3.o" \
    -o "$bin.tmp" -lpthread -ldl
  mv "$bin.tmp" "$bin"
fi

echo "DAYDREAM_WBTRV32_DIR=$src"
echo "DAYDREAM_WBTRV32_CONVERTER=$bin"
