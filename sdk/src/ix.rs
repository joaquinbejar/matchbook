//! Instruction builders with complete account lists.
//!
//! Trader instructions take a seat `hint`: the trader's seat index makes the
//! program skip a lookup, and [`NO_HINT`](crate::NO_HINT) always works.

use matchbook_program::engine::{MarketStatus, Side, TriggerDirection};
use matchbook_program::instruction::{
    build, CreateMarketArgs, DelegateArgs, OrderArgs, StopArgs, SwapArgs,
};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{pda, ASSOCIATED_TOKEN_PROGRAM_ID, PROGRAM_ID, SYSTEM_PROGRAM_ID};

/// The accounts every token-moving instruction of a market needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketAccounts {
    /// The market account.
    pub market: Pubkey,
    /// Base mint.
    pub base_mint: Pubkey,
    /// Quote mint.
    pub quote_mint: Pubkey,
    /// Base vault.
    pub base_vault: Pubkey,
    /// Quote vault.
    pub quote_vault: Pubkey,
    /// PDA that owns both vaults.
    pub vault_authority: Pubkey,
    /// Token program of the base mint.
    pub base_token_program: Pubkey,
    /// Token program of the quote mint.
    pub quote_token_program: Pubkey,
}

impl MarketAccounts {
    /// Derives the vaults and vault authority of a market.
    #[must_use]
    pub fn new(
        market: Pubkey,
        base_mint: Pubkey,
        quote_mint: Pubkey,
        base_token_program: Pubkey,
        quote_token_program: Pubkey,
    ) -> Self {
        let vault_authority = pda::vault_authority(&market).0;
        Self {
            market,
            base_mint,
            quote_mint,
            base_vault: pda::associated_token_address(
                &vault_authority,
                &base_mint,
                &base_token_program,
            ),
            quote_vault: pda::associated_token_address(
                &vault_authority,
                &quote_mint,
                &quote_token_program,
            ),
            vault_authority,
            base_token_program,
            quote_token_program,
        }
    }
}

fn ix(accounts: Vec<AccountMeta>, data: Vec<u8>) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts,
        data,
    }
}

fn trader(owner: &Pubkey, market: &Pubkey) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(*owner, true),
        AccountMeta::new(*market, false),
    ]
}

fn growing(owner: &Pubkey, market: &Pubkey) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(*owner, true),
        AccountMeta::new(*market, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
    ]
}

/// Creates a market. `market` signs: it is a fresh keypair the program
/// creates with `args.initial_nodes` nodes.
#[must_use]
pub fn create_market(payer: &Pubkey, a: &MarketAccounts, args: &CreateMarketArgs) -> Instruction {
    ix(
        vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(a.market, true),
            AccountMeta::new_readonly(a.base_mint, false),
            AccountMeta::new_readonly(a.quote_mint, false),
            AccountMeta::new(a.base_vault, false),
            AccountMeta::new(a.quote_vault, false),
            AccountMeta::new_readonly(a.vault_authority, false),
            AccountMeta::new_readonly(a.base_token_program, false),
            AccountMeta::new_readonly(a.quote_token_program, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        build::create_market(args),
    )
}

/// Claims a seat for `owner`. A permissioned market also needs its
/// authority's signature.
#[must_use]
pub fn claim_seat(
    payer: &Pubkey,
    owner: &Pubkey,
    market: &Pubkey,
    authority: Option<&Pubkey>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(*owner, true),
        AccountMeta::new(*market, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
    ];
    if let Some(auth) = authority {
        accounts.push(AccountMeta::new_readonly(*auth, true));
    }
    ix(accounts, build::claim_seat())
}

/// Moves `amount` of `mint` from `user_token` into the owner's seat.
#[must_use]
pub fn deposit(
    owner: &Pubkey,
    a: &MarketAccounts,
    hint: u32,
    mint: &Pubkey,
    user_token: &Pubkey,
    amount: u64,
) -> Instruction {
    let (vault, tp) = a.vault_of(mint);
    ix(
        vec![
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new(a.market, false),
            AccountMeta::new(*user_token, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(tp, false),
        ],
        build::deposit(hint, amount),
    )
}

/// Moves `amount` of `mint` from the owner's seat to `user_token`.
#[must_use]
pub fn withdraw(
    owner: &Pubkey,
    a: &MarketAccounts,
    hint: u32,
    mint: &Pubkey,
    user_token: &Pubkey,
    amount: u64,
) -> Instruction {
    let (vault, tp) = a.vault_of(mint);
    ix(
        vec![
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new(a.market, false),
            AccountMeta::new(*user_token, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(a.vault_authority, false),
            AccountMeta::new_readonly(tp, false),
        ],
        build::withdraw(hint, amount),
    )
}

impl MarketAccounts {
    /// The vault and token program for one of the market's mints (the
    /// quote side for any other mint, which the program rejects).
    fn vault_of(&self, mint: &Pubkey) -> (Pubkey, Pubkey) {
        if *mint == self.base_mint {
            (self.base_vault, self.base_token_program)
        } else {
            (self.quote_vault, self.quote_token_program)
        }
    }
}

/// Places an order. The signer is the seat's owner or a delegate; a
/// `referrer` (an account with a seat on this market) earns part of the
/// taker fee, and `o.referrer_seat` is its seat hint.
#[must_use]
pub fn place_order(
    signer: &Pubkey,
    market: &Pubkey,
    hint: u32,
    o: &OrderArgs,
    referrer: Option<&Pubkey>,
) -> Instruction {
    let mut accounts = growing(signer, market);
    if let Some(r) = referrer {
        accounts.push(AccountMeta::new_readonly(*r, false));
    }
    ix(accounts, build::place_order(hint, o))
}

/// Cancels an order or stop by id.
#[must_use]
pub fn cancel_order(signer: &Pubkey, market: &Pubkey, hint: u32, order_id: u64) -> Instruction {
    ix(trader(signer, market), build::cancel_order(hint, order_id))
}

/// Cancels every order of the seat on `side`, or on both sides with `None`.
#[must_use]
pub fn cancel_all(signer: &Pubkey, market: &Pubkey, hint: u32, side: Option<Side>) -> Instruction {
    ix(
        trader(signer, market),
        build::cancel_all(hint, side.map_or(2, |s| s as u8)),
    )
}

/// Cancels every order and stop with this client order id.
#[must_use]
pub fn cancel_by_client_id(
    signer: &Pubkey,
    market: &Pubkey,
    hint: u32,
    client_order_id: u64,
) -> Instruction {
    ix(
        trader(signer, market),
        build::cancel_by_client_id(hint, client_order_id),
    )
}

/// Shrinks an order in place, keeping its time priority.
#[must_use]
pub fn reduce_order(
    signer: &Pubkey,
    market: &Pubkey,
    hint: u32,
    order_id: u64,
    new_base_lots: u64,
) -> Instruction {
    ix(
        trader(signer, market),
        build::reduce_order(hint, order_id, new_base_lots),
    )
}

/// What [`batch_update`] cancels before its other cancels and orders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CancelAll {
    /// Nothing.
    #[default]
    None = 0,
    /// Every bid.
    Bids = 1,
    /// Every ask.
    Asks = 2,
    /// Every order.
    Both = 3,
}

/// Cancels and places up to 255 orders each in one instruction, the usual
/// way a market maker requotes.
#[must_use]
pub fn batch_update(
    signer: &Pubkey,
    market: &Pubkey,
    hint: u32,
    cancel_all: CancelAll,
    cancels: &[u64],
    orders: &[OrderArgs],
) -> Instruction {
    ix(
        growing(signer, market),
        build::batch_update(hint, cancel_all as u8, cancels, orders),
    )
}

/// Trades straight from the user's token accounts, with no seat.
#[must_use]
pub fn swap(
    user: &Pubkey,
    a: &MarketAccounts,
    user_base: &Pubkey,
    user_quote: &Pubkey,
    side: Side,
    args: &SwapArgs,
    referrer: Option<&Pubkey>,
) -> Instruction {
    let mut accounts = swap_accounts(user, a, user_base, user_quote);
    if let Some(r) = referrer {
        accounts.push(AccountMeta::new_readonly(*r, false));
    }
    ix(accounts, build::swap(side as u8, args))
}

/// The account list of [`swap`] without a referrer.
#[must_use]
pub fn swap_accounts(
    user: &Pubkey,
    a: &MarketAccounts,
    user_base: &Pubkey,
    user_quote: &Pubkey,
) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(*user, true),
        AccountMeta::new(a.market, false),
        AccountMeta::new(*user_base, false),
        AccountMeta::new(*user_quote, false),
        AccountMeta::new(a.base_vault, false),
        AccountMeta::new(a.quote_vault, false),
        AccountMeta::new_readonly(a.base_mint, false),
        AccountMeta::new_readonly(a.quote_mint, false),
        AccountMeta::new_readonly(a.vault_authority, false),
        AccountMeta::new_readonly(a.base_token_program, false),
        AccountMeta::new_readonly(a.quote_token_program, false),
    ]
}

/// Lets `args.delegate` trade or cancel for the owner's seat.
#[must_use]
pub fn set_delegate(
    owner: &Pubkey,
    market: &Pubkey,
    hint: u32,
    args: &DelegateArgs,
) -> Instruction {
    ix(growing(owner, market), build::set_delegate(hint, args))
}

/// Revokes a delegate. Signed by the seat's owner or by the delegate.
#[must_use]
pub fn remove_delegate(
    signer: &Pubkey,
    market: &Pubkey,
    hint: u32,
    delegate: &Pubkey,
) -> Instruction {
    ix(
        trader(signer, market),
        build::remove_delegate(hint, &delegate.to_bytes()),
    )
}

/// Places a stop that waits for a trade to reach `args.trigger_price`.
#[must_use]
pub fn place_stop(
    signer: &Pubkey,
    market: &Pubkey,
    hint: u32,
    direction: TriggerDirection,
    args: &StopArgs,
) -> Instruction {
    ix(
        growing(signer, market),
        build::place_stop(hint, direction as u8, args),
    )
}

/// Places up to `limit` stops whose trigger was reached. Permissionless.
#[must_use]
pub fn trigger_stops(market: &Pubkey, limit: u32) -> Instruction {
    ix(
        vec![AccountMeta::new(*market, false)],
        build::trigger_stops(limit),
    )
}

/// Removes up to `limit` expired orders. Permissionless.
#[must_use]
pub fn prune_expired(market: &Pubkey, limit: u32) -> Instruction {
    ix(
        vec![AccountMeta::new(*market, false)],
        build::prune_expired(limit),
    )
}

/// Sends the accrued fees to `destination`, a quote token account.
#[must_use]
pub fn collect_fees(authority: &Pubkey, a: &MarketAccounts, destination: &Pubkey) -> Instruction {
    ix(
        vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(a.market, false),
            AccountMeta::new(*destination, false),
            AccountMeta::new(a.quote_vault, false),
            AccountMeta::new_readonly(a.quote_mint, false),
            AccountMeta::new_readonly(a.vault_authority, false),
            AccountMeta::new_readonly(a.quote_token_program, false),
        ],
        build::collect_fees(),
    )
}

/// Changes the market's status.
#[must_use]
pub fn set_status(authority: &Pubkey, market: &Pubkey, status: MarketStatus) -> Instruction {
    ix(trader(authority, market), build::set_status(status as u8))
}
