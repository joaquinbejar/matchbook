//! [`jupiter_amm_interface::Amm`] for MatchBook markets.
//!
//! The adapter needs only the market account: mints, vaults and token
//! programs are all derived from its header, and quotes run the program's
//! own matching engine, so a quote is exactly what the swap executes on the
//! same account state.
//!
//! Jupiter routes through a DEX by a variant of its `Swap` enum, which only
//! Jupiter can add. Until MatchBook has one, [`MatchBookAmm`] reports
//! [`PLACEHOLDER_SWAP`]'s shape (a central limit order book swap with a
//! side), and the account metas follow MatchBook's own `Swap` instruction.
//! Callers that build transactions themselves should use
//! [`MatchBookAmm::swap_instruction`].

use std::sync::atomic::Ordering;

use anyhow::{anyhow, bail, Result};
use jupiter_amm_interface::{
    try_get_account_data, AccountMap, Amm, AmmContext, ClockRef, KeyedAccount, Quote, QuoteParams,
    Side as JupSide, Swap, SwapAndAccountMetas, SwapMode, SwapParams,
};
use matchbook_program::engine::{MarketStatus, Side, SwapOutcome, MARKET_FLAG_PERMISSIONED};
use rust_decimal::Decimal;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::ix::{self, MarketAccounts};
use crate::{MarketSnapshot, SwapArgs, PROGRAM_ID};

/// The `Swap` variant reported until Jupiter assigns MatchBook its own.
pub const PLACEHOLDER_SWAP: fn(JupSide) -> Swap = |side| Swap::Manifest { side };

/// Accounts in MatchBook's `Swap` instruction without a referrer.
pub const SWAP_ACCOUNTS_LEN: usize = 11;

/// A MatchBook market as a Jupiter AMM.
#[derive(Clone)]
pub struct MatchBookAmm {
    snapshot: MarketSnapshot,
    accounts: MarketAccounts,
    clock: ClockRef,
}

impl MatchBookAmm {
    /// The decoded market.
    #[must_use]
    pub fn snapshot(&self) -> &MarketSnapshot {
        &self.snapshot
    }

    /// The market's accounts.
    #[must_use]
    pub fn accounts(&self) -> &MarketAccounts {
        &self.accounts
    }

    fn now(&self) -> u64 {
        self.clock.unix_timestamp.load(Ordering::Relaxed).max(0) as u64
    }

    fn side_for(&self, input_mint: &Pubkey, output_mint: &Pubkey) -> Result<Side> {
        let a = &self.accounts;
        if *input_mint == a.quote_mint && *output_mint == a.base_mint {
            Ok(Side::Bid)
        } else if *input_mint == a.base_mint && *output_mint == a.quote_mint {
            Ok(Side::Ask)
        } else {
            Err(anyhow!(
                "{input_mint} -> {output_mint} is not this market's pair"
            ))
        }
    }

    /// The exact-in quote as the engine computes it.
    pub fn quote_outcome(&self, side: Side, amount_in: u64) -> Result<SwapOutcome> {
        Ok(self.snapshot.quote_exact_in(side, amount_in, self.now())?)
    }

    /// A MatchBook `Swap` instruction for an exact-in trade of `amount_in`
    /// that fails unless it receives at least `min_out`.
    pub fn swap_instruction(
        &self,
        user: &Pubkey,
        user_base: &Pubkey,
        user_quote: &Pubkey,
        side: Side,
        amount_in: u64,
        min_out: u64,
    ) -> Instruction {
        let p = self.snapshot.exact_in_params(side, amount_in);
        let args = SwapArgs {
            base_lots: p.base_lots,
            quote_budget: p.quote_budget,
            limit_price: p.limit_price,
            min_out,
            match_limit: p.match_limit,
            referrer_seat: u32::MAX,
        };
        ix::swap(
            user,
            &self.accounts,
            user_base,
            user_quote,
            side,
            &args,
            None,
        )
    }
}

impl Amm for MatchBookAmm {
    fn from_keyed_account(keyed_account: &KeyedAccount, amm_context: &AmmContext) -> Result<Self> {
        let snapshot = MarketSnapshot::new(keyed_account.key, &keyed_account.account.data)?;
        let accounts = snapshot.accounts()?;
        Ok(Self {
            snapshot,
            accounts,
            clock: amm_context.clock_ref.clone(),
        })
    }

    fn label(&self) -> String {
        "MatchBook".into()
    }

    fn program_id(&self) -> Pubkey {
        PROGRAM_ID
    }

    fn key(&self) -> Pubkey {
        self.snapshot.key()
    }

    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        vec![self.accounts.base_mint, self.accounts.quote_mint]
    }

    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        vec![self.snapshot.key()]
    }

    fn update(&mut self, account_map: &AccountMap) -> Result<()> {
        let data = try_get_account_data(account_map, &self.snapshot.key())?;
        self.snapshot.update(data)?;
        Ok(())
    }

    fn quote(&self, quote_params: &QuoteParams) -> Result<Quote> {
        if quote_params.swap_mode != SwapMode::ExactIn {
            bail!("MatchBook quotes exact-in swaps only");
        }
        let side = self.side_for(&quote_params.input_mint, &quote_params.output_mint)?;
        let out = self.quote_outcome(side, quote_params.amount)?;
        let (in_amount, out_amount) = match side {
            Side::Bid => (out.quote_net(side), out.base_atoms),
            Side::Ask => (out.base_atoms, out.quote_net(side)),
        };
        Ok(Quote {
            in_amount,
            out_amount,
            fee_amount: out.fee_atoms,
            fee_mint: self.accounts.quote_mint,
            fee_pct: Decimal::new(i64::from(self.snapshot.header().taker_fee_bps), 4),
        })
    }

    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> Result<SwapAndAccountMetas> {
        let side = self.side_for(&swap_params.source_mint, &swap_params.destination_mint)?;
        let (user_base, user_quote, jup_side) = match side {
            Side::Bid => (
                swap_params.destination_token_account,
                swap_params.source_token_account,
                JupSide::Bid,
            ),
            Side::Ask => (
                swap_params.source_token_account,
                swap_params.destination_token_account,
                JupSide::Ask,
            ),
        };
        Ok(SwapAndAccountMetas {
            swap: PLACEHOLDER_SWAP(jup_side),
            account_metas: ix::swap_accounts(
                &swap_params.token_transfer_authority,
                &self.accounts,
                &user_base,
                &user_quote,
            ),
        })
    }

    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
        Box::new(self.clone())
    }

    fn get_accounts_len(&self) -> usize {
        SWAP_ACCOUNTS_LEN
    }

    fn is_active(&self) -> bool {
        let h = self.snapshot.header();
        h.status == MarketStatus::Active as u32 && h.flags & MARKET_FLAG_PERMISSIONED == 0
    }
}
