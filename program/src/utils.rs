//! Account checks, token CPIs, market growth and logging.

use matchbook_engine::Market;
use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    AccountView, Address, ProgramResult, Resize,
};
use pinocchio_token::instructions::TransferChecked;

use crate::error::{engine, MatchbookError};

/// Writes `data` as one `Program data:` log entry.
#[inline(always)]
pub(crate) fn log_data(data: &[u8]) {
    #[cfg(target_os = "solana")]
    {
        let slices = [data];
        // SAFETY: `slices` is a valid array of one byte slice that outlives the
        // syscall; the runtime only reads from it.
        #[allow(unsafe_code)]
        unsafe {
            pinocchio::syscalls::sol_log_data(slices.as_ptr() as *const u8, slices.len() as u64);
        }
    }
    #[cfg(not(target_os = "solana"))]
    let _ = data;
}

/// Sets the instruction's return data.
#[inline(always)]
pub(crate) fn set_return_data(data: &[u8]) {
    #[cfg(target_os = "solana")]
    // SAFETY: `data` is a valid byte slice; the runtime copies it.
    #[allow(unsafe_code)]
    unsafe {
        pinocchio::syscalls::sol_set_return_data(data.as_ptr(), data.len() as u64);
    }
    #[cfg(not(target_os = "solana"))]
    let _ = data;
}

pub(crate) fn require_signer(a: &AccountView) -> ProgramResult {
    if a.is_signer() {
        Ok(())
    } else {
        Err(MatchbookError::MissingSignature.into())
    }
}

pub(crate) fn require_market(a: &AccountView) -> ProgramResult {
    if a.owned_by(&crate::ID) {
        Ok(())
    } else {
        Err(MatchbookError::InvalidMarketOwner.into())
    }
}

pub(crate) fn require_key(a: &AccountView, expected: &[u8; 32]) -> ProgramResult {
    if a.address().as_array() == expected {
        Ok(())
    } else {
        Err(MatchbookError::InvalidAccount.into())
    }
}

/// Token amount of an SPL Token or Token-2022 account.
pub(crate) fn token_amount(a: &AccountView) -> Result<u64, ProgramError> {
    let d = a.try_borrow()?;
    let b = d
        .get(64..72)
        .ok_or(MatchbookError::InvalidAccount)?
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccount)?;
    Ok(u64::from_le_bytes(b))
}

/// Decimals of a mint and a check that `token_program` owns it.
pub(crate) fn mint_decimals(mint: &AccountView, token_program: &AccountView) -> Result<u8, ProgramError> {
    if !mint.owned_by(token_program.address()) {
        return Err(MatchbookError::InvalidMint.into());
    }
    let d = mint.try_borrow()?;
    d.get(44).copied().ok_or_else(|| MatchbookError::InvalidMint.into())
}

/// Seeds of the vault authority PDA.
pub(crate) fn vault_seeds<'a>(market: &'a Address, bump: &'a [u8; 1]) -> [Seed<'a>; 3] {
    [
        Seed::from(crate::VAULT_AUTHORITY_SEED),
        Seed::from(market.as_ref()),
        Seed::from(bump),
    ]
}

/// `transfer_checked` through whichever token program owns the mint.
#[allow(clippy::too_many_arguments)]
pub(crate) fn transfer(
    from: &AccountView,
    mint: &AccountView,
    to: &AccountView,
    authority: &AccountView,
    token_program: &AccountView,
    amount: u64,
    decimals: u8,
    signer: Option<&Signer>,
) -> ProgramResult {
    if amount == 0 {
        return Ok(());
    }
    let ix = TransferChecked::new(from, mint, to, authority, amount, decimals);
    match signer {
        Some(s) => ix.invoke_signed_with_program(core::slice::from_ref(s), token_program.address()),
        None => ix.invoke_signed_with_program(&[], token_program.address()),
    }
}

/// Grows the market so that `needed` nodes are free, charging rent to `payer`.
pub(crate) fn ensure_nodes(market: &mut AccountView, payer: &AccountView, needed: u32) -> ProgramResult {
    let extra = {
        let mut data = market.try_borrow_mut()?;
        let m = Market::load_mut(&mut data).map_err(engine)?;
        m.bytes_to_grow(needed)
    };
    if extra == 0 {
        return Ok(());
    }
    let new_len = market.data_len() + extra;
    let rent = Rent::get()?.try_minimum_balance(new_len)?;
    let have = market.lamports();
    if rent > have {
        require_signer(payer)?;
        pinocchio_system::instructions::Transfer {
            from: payer,
            to: market,
            lamports: rent - have,
        }
        .invoke()?;
    }
    market.resize(new_len)
}

/// Current unix time as u64.
pub(crate) fn now() -> Result<u64, ProgramError> {
    let c = pinocchio::sysvars::clock::Clock::get()?;
    Ok(u64::try_from(c.unix_timestamp).unwrap_or(0))
}
