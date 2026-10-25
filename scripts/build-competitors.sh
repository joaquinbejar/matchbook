#!/usr/bin/env bash
# Builds Manifest, Phoenix v1 and OpenBook v2 from source at pinned commits and copies the
# binaries to target/competitors/, where program/tests/compare_*.rs pick them up.
#
# Each program is built with the platform tools of its own era: OpenBook v2 overflows its
# stack frames under newer compilers, and Phoenix links solana-program 1.14, whose CPI ABI
# predates StableInstruction. Phoenix gets a small patch (competitors/phoenix-stable-cpi.patch)
# that passes CPIs through the stable layout today's runtime reads; it changes no logic and
# adds a few hundred compute units per CPI at most.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# Outside this workspace, or cargo would treat the checkouts as members of it.
WORK="${COMPETITORS_DIR:-${TMPDIR:-/tmp}/matchbook-competitors}"
OUT="$ROOT/target/competitors"
mkdir -p "$WORK" "$OUT"

fetch() { # name url commit
  local dir="$WORK/$1"
  if [ ! -d "$dir/.git" ]; then
    git clone --quiet "$2" "$dir"
  fi
  git -C "$dir" fetch --quiet origin "$3" 2>/dev/null || true
  git -C "$dir" checkout --quiet --force "$3"
  git -C "$dir" clean --quiet -fd
}

fetch manifest https://github.com/CKS-Systems/manifest 51f80839e76a1a0e32e96b3a0114f1e72b49a842
(cd "$WORK/manifest/programs/manifest" && cargo build-sbf --tools-version v1.52)
cp "$WORK/manifest/target/deploy/manifest.so" "$OUT/"

fetch phoenix-v1 https://github.com/Ellipsis-Labs/phoenix-v1 6ae54af5f07f4d0ae8430b08cbe81d10432cd34c
git -C "$WORK/phoenix-v1" apply "$ROOT/scripts/competitors/phoenix-stable-cpi.patch"
(cd "$WORK/phoenix-v1" && cargo build-sbf --tools-version v1.37)
cp "$WORK/phoenix-v1/target/deploy/phoenix.so" "$OUT/"

fetch openbook-v2 https://github.com/openbook-dex/openbook-v2 f3e17421e675b083b584867594bf3cf4f675d156
(cd "$WORK/openbook-v2/programs/openbook-v2" && cargo build-sbf --tools-version v1.37 --features enable-gpl)
cp "$WORK/openbook-v2/target/deploy/openbook_v2.so" "$OUT/"

echo "Built: $(ls "$OUT")"
echo "Run: cargo test -p matchbook-program --test compare_manifest --test compare_phoenix --test compare_openbook -- --nocapture"
