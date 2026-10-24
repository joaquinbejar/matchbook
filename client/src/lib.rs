//! Command-line client and market-making bot for MatchBook.
//!
//! The `matchbook` binary creates markets, moves funds, places and cancels
//! every kind of order, reads the book and manages session keys. The
//! `matchbook-mm` binary quotes a market around a reference price with a
//! session key and risk limits. Both talk to any cluster through its RPC.

pub mod chain;
pub mod mm;
pub mod price;
pub mod units;

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::chain::{cluster_url, default_keypair_path, load_keypair, Chain, Fees};

/// Connection options both binaries take.
#[derive(Debug, Clone, Args)]
pub struct ConnectArgs {
    /// Cluster: localnet, devnet, mainnet, testnet or an RPC URL.
    #[arg(
        short = 'u',
        long = "url",
        env = "MATCHBOOK_RPC",
        default_value = "localnet",
        global = true
    )]
    pub url: String,
    /// Fee payer and default owner (Solana CLI keypair file).
    #[arg(
        short = 'k',
        long = "keypair",
        env = "MATCHBOOK_KEYPAIR",
        global = true
    )]
    pub keypair: Option<PathBuf>,
    /// Compute unit limit for each transaction (0 = runtime default).
    #[arg(long, default_value_t = 0, global = true)]
    pub cu_limit: u32,
    /// Priority fee in micro-lamports per compute unit.
    #[arg(long, default_value_t = 0, global = true)]
    pub priority_fee: u64,
}

impl ConnectArgs {
    /// The keypair path, defaulting to the Solana CLI wallet.
    #[must_use]
    pub fn keypair_path(&self) -> PathBuf {
        self.keypair.clone().unwrap_or_else(default_keypair_path)
    }

    /// Connects with the keypair as fee payer.
    pub fn connect(&self) -> Result<Chain> {
        let payer = load_keypair(&self.keypair_path())?;
        Ok(Chain::new(
            &cluster_url(&self.url),
            payer,
            Fees {
                cu_limit: self.cu_limit,
                priority_micro_lamports: self.priority_fee,
            },
        ))
    }
}
