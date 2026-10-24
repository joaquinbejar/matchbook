#!/bin/bash
# Matchbook Devnet Deployment Script
#
# This script deploys the Matchbook program to Solana Devnet.
# Prerequisites:
#   - Solana CLI installed
#   - Anchor CLI installed
#   - Sufficient SOL balance for deployment (~5 SOL)
#
# Usage:
#   ./scripts/deploy-devnet.sh

set -e

echo "=== Matchbook Devnet Deployment ==="
echo ""

# Check for required tools
command -v solana >/dev/null 2>&1 || { echo "Error: solana CLI not found"; exit 1; }
command -v anchor >/dev/null 2>&1 || { echo "Error: anchor CLI not found"; exit 1; }

# Configure for devnet
echo "Configuring Solana CLI for devnet..."
solana config set --url https://api.devnet.solana.com

# Check current keypair
KEYPAIR=$(solana config get keypair | awk '{print $2}')
echo "Using keypair: $KEYPAIR"

# Check balance
BALANCE=$(solana balance | awk '{print $1}')
echo "Current balance: $BALANCE SOL"

# Airdrop if balance is low
if (( $(echo "$BALANCE < 2" | bc -l) )); then
    echo "Balance low, requesting airdrop..."
    solana airdrop 2 || echo "Airdrop failed, you may need to use a faucet"
    sleep 5
    BALANCE=$(solana balance | awk '{print $1}')
    echo "New balance: $BALANCE SOL"
fi

# Build the program
echo ""
echo "Building program..."
anchor build

# Get program ID
PROGRAM_ID=$(solana-keygen pubkey target/deploy/matchbook_program-keypair.json 2>/dev/null || echo "")
if [ -z "$PROGRAM_ID" ]; then
    echo "Generating new program keypair..."
    solana-keygen new -o target/deploy/matchbook_program-keypair.json --no-bip39-passphrase --force
    PROGRAM_ID=$(solana-keygen pubkey target/deploy/matchbook_program-keypair.json)
fi
echo "Program ID: $PROGRAM_ID"

# Deploy
echo ""
echo "Deploying to devnet..."
anchor deploy --provider.cluster devnet

echo ""
echo "=== Deployment Complete ==="
echo "Program ID: $PROGRAM_ID"
echo ""
echo "Next steps:"
echo "  1. Run ./scripts/init-market.sh to create a test market"
echo "  2. Run anchor test --provider.cluster devnet to run integration tests"
