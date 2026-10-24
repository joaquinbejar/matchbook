# Matchbook Deployment Scripts

This directory contains scripts for deploying and managing Matchbook on Solana.

## Prerequisites

- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools) v1.18+
- [Anchor CLI](https://www.anchor-lang.com/docs/installation) v0.30+
- [SPL Token CLI](https://spl.solana.com/token) (for token operations)

## Scripts

### deploy-devnet.sh

Deploys the Matchbook program to Solana Devnet.

```bash
./scripts/deploy-devnet.sh
```

**What it does:**
1. Configures Solana CLI for devnet
2. Checks balance and requests airdrop if needed
3. Builds the program with Anchor
4. Deploys to devnet

**Requirements:**
- ~5 SOL for deployment (will airdrop if balance < 2 SOL)

### init-market.sh

Initializes a test market with standard parameters.

```bash
# Create new token mints and market
./scripts/init-market.sh

# Use existing mints
./scripts/init-market.sh <BASE_MINT> <QUOTE_MINT>
```

**Default Market Parameters:**
| Parameter | Value | Description |
|-----------|-------|-------------|
| Base lot size | 1,000,000 | 1 token (6 decimals) |
| Quote lot size | 1,000 | 0.001 token |
| Tick size | 100 | 0.0001 quote per base |
| Min order size | 1 | 1 lot minimum |
| Taker fee | 30 bps | 0.3% |
| Maker fee | -10 bps | 0.1% rebate |

## Deployment Workflow

1. **Deploy to Devnet:**
   ```bash
   ./scripts/deploy-devnet.sh
   ```

2. **Create Test Market:**
   ```bash
   ./scripts/init-market.sh
   ```

3. **Run Integration Tests:**
   ```bash
   anchor test --provider.cluster devnet
   ```

## Mainnet Deployment

For mainnet deployment, use the Anchor CLI directly with appropriate security measures:

```bash
# Configure for mainnet
solana config set --url https://api.mainnet-beta.solana.com

# Use a secure keypair
solana config set --keypair /path/to/deployer-keypair.json

# Deploy with upgrade authority
anchor deploy --provider.cluster mainnet

# Transfer upgrade authority to multisig (recommended)
solana program set-upgrade-authority <PROGRAM_ID> \
  --new-upgrade-authority <MULTISIG_ADDRESS>
```

## Troubleshooting

### Airdrop Failed
Devnet airdrops may fail due to rate limits. Use the [Solana Faucet](https://faucet.solana.com/) instead.

### Insufficient Balance
Deployment requires ~5 SOL. Check balance with:
```bash
solana balance
```

### Program Already Deployed
To redeploy, use:
```bash
anchor upgrade target/deploy/matchbook_program.so --program-id <PROGRAM_ID>
```
