#!/usr/bin/env bash
# End-to-end test of both clients on a local validator.
#
#   ./scripts/e2e-clients.sh
#
# Starts solana-test-validator with the program preloaded, then:
#   1. drives the Rust CLI through every command on market A, with the Rust
#      bot quoting it through a session key while the TypeScript CLI takes;
#   2. drives the TypeScript CLI through the same commands on market B, with
#      the TypeScript bot quoting while the Rust CLI takes.
# Needs solana-test-validator on PATH and target/deploy/matchbook_program.so.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD

SO=target/deploy/matchbook_program.so
[ -f "$SO" ] || { echo "build the program first (cargo build-sbf)"; exit 1; }
PROGRAM_ID=$(sed -n 's/.*declare_id!("\([^"]*\)").*/\1/p' program/src/lib.rs)
PORT=${PORT:-8899}
URL=http://127.0.0.1:$PORT

WORK=$(mktemp -d)
echo "work dir $WORK"
solana-test-validator --reset --quiet --ledger "$WORK/ledger" --rpc-port "$PORT" \
  --bpf-program "$PROGRAM_ID" "$SO" >"$WORK/validator.log" 2>&1 &
VALIDATOR=$!
BOTS=()
cleanup() {
  for p in "${BOTS[@]}"; do kill "$p" 2>/dev/null || true; done
  kill "$VALIDATOR" 2>/dev/null || true
}
trap cleanup EXIT
for _ in $(seq 1 120); do
  curl -s "$URL" -X POST -H 'content-type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' | grep -q '"ok"' && break
  sleep 0.5
done

cargo build --release -q -p matchbook-client
(cd ts-sdk && npm run -s build) && (cd ts-client && npm run -s build)
RS() { "$ROOT/target/release/matchbook" -u "$URL" "$@"; }
RS_MM() { "$ROOT/target/release/matchbook-mm" -u "$URL" "$@"; }
TS() { node "$ROOT/ts-client/dist/cli.js" -u "$URL" "$@"; }
TS_MM() { node "$ROOT/ts-client/dist/mm-bot.js" -u "$URL" "$@"; }
step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
field() { sed -n "s/^$1 \([^ ]*\).*/\1/p"; }

# Runs one client ($1 = RS or TS) through every command on a new market.
# The bot and the taker come from the other client.
exercise() {
  local CLI=$1 OTHER=$2 MM=$3 tag=$4
  local maker="$WORK/$tag-maker.json" taker="$WORK/$tag-taker.json" session="$WORK/$tag-session.json"
  local M T
  M=$($CLI keygen "$maker")
  T=$($CLI keygen "$taker")
  $CLI -k "$maker" airdrop 50 >/dev/null
  $CLI -k "$maker" airdrop 5 --to "$T" >/dev/null

  step "$tag: mints, market, seat, deposit"
  local BASE QUOTE MARKET
  BASE=$($CLI -k "$maker" token create-mint --decimals 9)
  QUOTE=$($CLI -k "$maker" token create-mint --decimals 6)
  $CLI -k "$maker" token mint "$BASE" 1000
  $CLI -k "$maker" token mint "$QUOTE" 200000
  $CLI -k "$maker" token mint "$BASE" 100 --to "$T"
  $CLI -k "$maker" token mint "$QUOTE" 50000 --to "$T"
  MARKET=$($CLI -k "$maker" market create --base-mint "$BASE" --quote-mint "$QUOTE" \
    --base-lot-size 1000000 --quote-lot-size 1 --taker-fee-bps 10 --maker-rebate-bps 2 \
    --referral-share-bps 2000 | field market)
  $CLI -k "$maker" market show "$MARKET"
  $CLI -k "$maker" seat claim "$MARKET"
  $CLI -k "$maker" deposit "$MARKET" --base 500 --quote 100000
  $CLI -k "$maker" balance "$MARKET"

  step "$tag: orders of every kind"
  $CLI -k "$maker" place "$MARKET" sell 2 151 --type post-only --client-id 11
  $CLI -k "$maker" place "$MARKET" buy 2 149 --client-id 12
  $CLI -k "$maker" place "$MARKET" buy 1 148 --type post-only --reverse-spread-bps 50
  $CLI -k "$maker" place "$MARKET" sell 1 160 --expires-in 1h --client-id 13
  $CLI -k "$maker" place "$MARKET" buy 1 155 --type post-only-slide
  local ID
  ID=$($CLI -k "$maker" place "$MARKET" sell 3 152 | field order | tr -d :)
  $CLI -k "$maker" reduce "$MARKET" "$ID" 1.5
  $CLI -k "$maker" book "$MARKET"
  $CLI -k "$maker" cancel "$MARKET" "$ID"
  $CLI -k "$maker" cancel-client "$MARKET" 13
  $CLI -k "$maker" place "$MARKET" buy 1 150.5 --type ioc
  $CLI -k "$maker" place "$MARKET" buy 5 1 --type fok && { echo "FOK should have failed"; exit 1; } || echo "FOK rejected as expected"

  step "$tag: stops and a bracket"
  $CLI -k "$maker" stop "$MARKET" sell 1 --trigger 140 --when below
  $CLI -k "$maker" bracket "$MARKET" sell 1 --stop-loss 139 --take-profit 170
  $CLI -k "$maker" orders "$MARKET"
  $CLI -k "$maker" --json orders "$MARKET" | grep -q ocoWith
  # A trade at 151 or above sets this one off, with no keeper.
  $CLI -k "$maker" stop "$MARKET" buy 0.5 --trigger 151 --when above --limit 151.5
  $OTHER -k "$taker" swap "$MARKET" buy 50
  $CLI -k "$maker" orders "$MARKET" | grep -q 'when >= 151 ' && { echo "stop did not trigger"; exit 1; } || echo "stop triggered by the swap"

  step "$tag: session key"
  $CLI -k "$maker" session create "$MARKET" --out "$session" --expires-in 1h --fund 0.5
  $CLI -k "$maker" session list "$MARKET"
  $CLI -k "$maker" place "$MARKET" sell 1 158 --signer "$session" --owner "$M"
  $CLI -k "$maker" cancel-all "$MARKET" --side sell --signer "$session" --owner "$M"
  $CLI -k "$maker" cancel-all "$MARKET"

  step "$tag: bot quotes with the session key, the other client takes"
  $MM -k "$maker" --market "$MARKET" --session-keypair "$session" --owner "$M" \
    --price fixed:150 --levels 3 --size 1 --spread-bps 10 --step-bps 10 \
    --max-position 20 --skew-bps 20 --interval-ms 400 --cycles 25 >"$WORK/$tag-bot.log" 2>&1 &
  local BOT=$!
  BOTS+=("$BOT")
  sleep 4
  $OTHER -k "$taker" book "$MARKET"
  $OTHER -k "$taker" swap "$MARKET" buy 160
  sleep 2
  $OTHER -k "$taker" swap "$MARKET" sell 0.7
  wait "$BOT"
  cat "$WORK/$tag-bot.log"
  grep -q "filled: bought" "$WORK/$tag-bot.log"
  grep -q "filled: sold" "$WORK/$tag-bot.log"
  grep -q "pulling" "$WORK/$tag-bot.log"
  $CLI -k "$maker" orders "$MARKET"

  step "$tag: dry run against the book, then withdraw, fees, status"
  $CLI -k "$maker" place "$MARKET" buy 1 149 --type post-only
  $CLI -k "$maker" place "$MARKET" sell 1 151 --type post-only
  $MM -k "$maker" --market "$MARKET" --price book --size 1 --cycles 1 --dry-run
  local SK
  SK=$($CLI -k "$maker" session list "$MARKET" | head -1 | cut -d' ' -f1)
  $CLI -k "$maker" session revoke "$MARKET" "$SK"
  $CLI -k "$maker" withdraw "$MARKET" --base 1 --quote 10
  $CLI -k "$maker" market collect-fees "$MARKET"
  $CLI -k "$maker" market set-status "$MARKET" cancel-only
  $CLI -k "$maker" place "$MARKET" buy 1 149 && { echo "cancel-only should reject"; exit 1; } || echo "cancel-only rejected as expected"
  $CLI -k "$maker" cancel-all "$MARKET"
  $CLI -k "$maker" withdraw "$MARKET" --all
  $CLI -k "$maker" balance "$MARKET"
  $CLI -k "$maker" market show "$MARKET"
}

exercise RS TS RS_MM rust
exercise TS RS TS_MM ts
step "all good"
