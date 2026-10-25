#!/usr/bin/env bash
# Builds the playground: the engine as WebAssembly, inlined into one HTML page.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -p matchbook-playground --release --target wasm32-unknown-unknown
mkdir -p playground/dist
WASM=target/wasm32-unknown-unknown/release/matchbook_playground.wasm
python3 - "$WASM" playground/index.html playground/dist/index.html <<'PY'
import base64, sys
wasm, src, dst = sys.argv[1:]
b64 = base64.b64encode(open(wasm, 'rb').read()).decode()
open(dst, 'w').write(open(src).read().replace('__WASM_BASE64__', b64))
PY
echo "playground/dist/index.html"
