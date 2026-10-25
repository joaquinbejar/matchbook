//! Program-derived addresses.

use solana_pubkey::Pubkey;

use crate::{ASSOCIATED_TOKEN_PROGRAM_ID, PROGRAM_ID};

/// The PDA that owns a market's vaults, with its bump.
#[must_use]
pub fn vault_authority(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[matchbook_program::VAULT_AUTHORITY_SEED, market.as_ref()],
        &PROGRAM_ID,
    )
}

/// The associated token account of `owner` for `mint`.
#[must_use]
pub fn associated_token_address(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    )
    .0
}

/// A market's vault for `mint`: the vault authority's associated token account.
#[must_use]
pub fn vault(market: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    associated_token_address(&vault_authority(market).0, mint, token_program)
}
