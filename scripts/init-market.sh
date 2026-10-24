#!/bin/bash
# Matchbook Market Initialization Script
#
# This script creates a test market on devnet with standard parameters.
# Prerequisites:
#   - Solana CLI installed
#   - Program already deployed (run deploy-devnet.sh first)
#   - Sufficient SOL balance
#
# Usage:
#   ./scripts/init-market.sh [BASE_MINT] [QUOTE_MINT]
#
# If mints are not provided, creates new test token mints.

set -e

echo "=== Matchbook Market Initialization ==="
echo ""

# Check for required tools
command -v solana >/dev/null 2>&1 || { echo "Error: solana CLI not found"; exit 1; }
command -v spl-token >/dev/null 2>&1 || { echo "Error: spl-token CLI not found"; exit 1; }

# Configuration
BASE_LOT_SIZE=1000000      # 1 token with 6 decimals
QUOTE_LOT_SIZE=1000        # 0.001 token
TICK_SIZE=100              # 0.0001 quote per base
MIN_ORDER_SIZE=1           # Minimum 1 lot
TAKER_FEE_BPS=30           # 0.3%
MAKER_FEE_BPS=-10          # -0.1% (rebate)

echo "Market Parameters:"
echo "  Base lot size: $BASE_LOT_SIZE"
echo "  Quote lot size: $QUOTE_LOT_SIZE"
echo "  Tick size: $TICK_SIZE"
echo "  Min order size: $MIN_ORDER_SIZE"
echo "  Taker fee: ${TAKER_FEE_BPS} bps"
echo "  Maker fee: ${MAKER_FEE_BPS} bps (negative = rebate)"
echo ""

# Get or create base mint
if [ -n "$1" ]; then
    BASE_MINT=$1
    echo "Using provided base mint: $BASE_MINT"
else
    echo "Creating new base token mint..."
    BASE_MINT=$(spl-token create-token --decimals 6 2>&1 | grep "Creating token" | awk '{print $3}')
    echo "Created base mint: $BASE_MINT"
fi

# Get or create quote mint
if [ -n "$2" ]; then
    QUOTE_MINT=$2
    echo "Using provided quote mint: $QUOTE_MINT"
else
    echo "Creating new quote token mint..."
    QUOTE_MINT=$(spl-token create-token --decimals 6 2>&1 | grep "Creating token" | awk '{print $3}')
    echo "Created quote mint: $QUOTE_MINT"
fi

echo ""
echo "=== Market Configuration ==="
echo "Base Mint: $BASE_MINT"
echo "Quote Mint: $QUOTE_MINT"
echo ""
echo "To create the market, use the Matchbook SDK or run:"
echo ""
echo "  anchor run create-market -- \\"
echo "    --base-mint $BASE_MINT \\"
echo "    --quote-mint $QUOTE_MINT \\"
echo "    --base-lot-size $BASE_LOT_SIZE \\"
echo "    --quote-lot-size $QUOTE_LOT_SIZE \\"
echo "    --tick-size $TICK_SIZE \\"
echo "    --min-order-size $MIN_ORDER_SIZE \\"
echo "    --taker-fee-bps $TAKER_FEE_BPS \\"
echo "    --maker-fee-bps $MAKER_FEE_BPS"
echo ""
echo "=== Initialization Complete ==="
