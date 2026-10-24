#!/usr/bin/env bash
# Builds and deploys the MatchBook program.
#
#   ./scripts/deploy.sh [devnet|mainnet-beta|localhost|<rpc url>]
#
# The program address comes from target/deploy/matchbook_program-keypair.json.
# Without that file a new keypair is generated and its address is written into
# the program and the TypeScript SDK (the Rust SDK reads it from the program),
# so every client agrees on the address before the build.
set -euo pipefail
cd "$(dirname "$0")/.."

CLUSTER="${1:-devnet}"
case "$CLUSTER" in
  devnet) URL=https://api.devnet.solana.com ;;
  mainnet-beta) URL=https://api.mainnet-beta.solana.com ;;
  localhost) URL=http://127.0.0.1:8899 ;;
  *) URL="$CLUSTER" ;;
esac

for tool in solana solana-keygen cargo-build-sbf; do
  command -v "$tool" >/dev/null || { echo "error: $tool not found (install the Solana CLI)"; exit 1; }
done

KEYPAIR=target/deploy/matchbook_program-keypair.json
if [ ! -f "$KEYPAIR" ]; then
  mkdir -p target/deploy
  solana-keygen new --no-bip39-passphrase --silent -o "$KEYPAIR"
fi
PROGRAM_ID=$(solana-keygen pubkey "$KEYPAIR")
CURRENT=$(sed -n 's/.*declare_id!("\([^"]*\)").*/\1/p' program/src/lib.rs)
if [ "$PROGRAM_ID" != "$CURRENT" ]; then
  echo "Setting the program address to $PROGRAM_ID"
  sed -i.bak "s/$CURRENT/$PROGRAM_ID/" program/src/lib.rs ts-sdk/src/constants.ts
  rm -f program/src/lib.rs.bak ts-sdk/src/constants.ts.bak
fi

cargo build-sbf --manifest-path program/Cargo.toml

PAYER=$(solana -u "$URL" address)
BALANCE=$(solana -u "$URL" balance | awk '{print $1}')
echo "Deploying $PROGRAM_ID to $URL from $PAYER (balance $BALANCE SOL)"
solana -u "$URL" program deploy --program-id "$KEYPAIR" target/deploy/matchbook_program.so
echo
echo "Deployed. Create a demo market with:"
echo "  cd ts-sdk && npm install && RPC_URL=$URL npx tsx examples/demo-market.ts"
