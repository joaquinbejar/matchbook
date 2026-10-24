//! Instruction handlers.

use matchbook_engine::{permission, Market, MarketParams, MarketStatus, Side, SwapParams};
use pinocchio::{cpi::Signer, error::ProgramError, AccountView, Address, ProgramResult};
use pinocchio_token::TokenInterface;

use crate::error::{engine, MatchbookError};
use crate::events::LogSink;
use crate::instruction::{CreateMarketArgs, DelegateArgs, Header, Ix, OrderArgs, Reader, SwapArgs};
use crate::utils::{
    ensure_nodes, mint_decimals, now, require_key, require_market, require_signer, set_return_data,
    token_amount, transfer, vault_seeds,
};

/// Maximum orders or cancels in one batch.
pub const MAX_BATCH: u8 = 32;

/// Program entry.
pub fn process_instruction(
    _program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    let (h, mut r) = Header::parse(data)?;
    let r = &mut r;
    match h.ix {
        Ix::CreateMarket => create_market(accounts, r),
        Ix::ClaimSeat => claim_seat(accounts),
        Ix::Deposit => deposit(accounts, h, r),
        Ix::Withdraw => withdraw(accounts, h, r),
        Ix::PlaceOrder => place_order(accounts, h, r),
        Ix::CancelOrder | Ix::CancelAll | Ix::CancelByClientId | Ix::ReduceOrder => {
            cancel(accounts, h, r)
        }
        Ix::BatchUpdate => batch_update(accounts, h, r),
        Ix::Swap => swap(accounts, h, r),
        Ix::CollectFees => collect_fees(accounts),
        Ix::SetStatus => set_status(accounts, h),
        Ix::PruneExpired => prune_expired(accounts, h),
        Ix::SetDelegate => set_delegate(accounts, h, r),
        Ix::RemoveDelegate => remove_delegate(accounts, h, r),
    }
}

/// Reads the clock at most once, and only when an order can expire.
#[derive(Default)]
struct LazyClock(Option<u64>);

impl LazyClock {
    fn now(&mut self, m: &Market<'_>, incoming_expires: bool) -> Result<u64, ProgramError> {
        if !m.needs_clock(incoming_expires) {
            return Ok(0);
        }
        if let Some(t) = self.0 {
            return Ok(t);
        }
        let t = now()?;
        self.0 = Some(t);
        Ok(t)
    }
}

fn owner_bytes(a: &AccountView) -> [u8; 32] {
    a.address().to_bytes()
}

fn create_market(accounts: &mut [AccountView], r: &mut Reader<'_>) -> ProgramResult {
    let [payer, market, base_mint, quote_mint, base_vault, quote_vault, vault_authority, base_tp, quote_tp, _ata, system_program, ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    require_signer(payer)?;
    require_signer(market)?;
    let a: CreateMarketArgs = r.pod()?;
    if a.reserved != 0 {
        return Err(MatchbookError::InvalidInstructionData.into());
    }
    if base_mint.address() == quote_mint.address() {
        return Err(MatchbookError::SameMints.into());
    }
    pinocchio_token::TokenProgram::verify(base_tp.address())?;
    pinocchio_token::TokenProgram::verify(quote_tp.address())?;
    mint_decimals(base_mint, base_tp)?;
    mint_decimals(quote_mint, quote_tp)?;

    let (va, bump) = Address::try_find_program_address(
        &[crate::VAULT_AUTHORITY_SEED, market.address().as_ref()],
        &crate::ID,
    )
    .ok_or(MatchbookError::InvalidAccount)?;
    if va != *vault_authority.address() {
        return Err(MatchbookError::InvalidAccount.into());
    }

    // Large markets are pre-created by the client with a top-level
    // `create_account` (CPI allocations are capped at 10 KiB); small ones are
    // created here.
    if !market.owned_by(&crate::ID) {
        let space = matchbook_engine::market_size(a.initial_nodes as usize) as u64;
        pinocchio_system::instructions::CreateAccount::with_minimum_balance(
            payer,
            market,
            space,
            &crate::ID,
            None,
        )?
        .invoke()?;
    }
    for (vault, mint, tp) in [
        (&*base_vault, &*base_mint, &*base_tp),
        (&*quote_vault, &*quote_mint, &*quote_tp),
    ] {
        pinocchio_associated_token_account::instructions::CreateIdempotent {
            funding_account: payer,
            account: vault,
            wallet: vault_authority,
            mint,
            system_program,
            token_program: tp,
        }
        .invoke()?;
    }

    let mut data = market.try_borrow_mut()?;
    Market::init(
        &mut data,
        &MarketParams {
            base_mint: owner_bytes(base_mint),
            quote_mint: owner_bytes(quote_mint),
            base_vault: owner_bytes(base_vault),
            quote_vault: owner_bytes(quote_vault),
            authority: a.authority,
            base_lot_size: a.base_lot_size,
            quote_lot_size: a.quote_lot_size,
            min_base_lots: a.min_base_lots,
            taker_fee_bps: a.taker_fee_bps,
            maker_rebate_bps: a.maker_rebate_bps,
            host_bump: u32::from(bump),
        },
    )
    .map_err(engine)
}

fn claim_seat(accounts: &mut [AccountView]) -> ProgramResult {
    let [payer, owner, market, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    require_signer(owner)?;
    require_market(market)?;
    ensure_nodes(market, payer, 1)?;
    let mut data = market.try_borrow_mut()?;
    let mut m = Market::load_mut(&mut data).map_err(engine)?;
    m.claim_seat(&owner_bytes(owner)).map_err(engine)?;
    Ok(())
}

/// Which side of the market a mint/vault pair belongs to.
fn resolve_vault(
    m: &Market<'_>,
    mint: &AccountView,
    vault: &AccountView,
    tp: &AccountView,
) -> Result<bool, ProgramError> {
    let h = m.header();
    let is_base = if mint.address().as_array() == &h.base_mint {
        require_key(vault, &h.base_vault)?;
        true
    } else if mint.address().as_array() == &h.quote_mint {
        require_key(vault, &h.quote_vault)?;
        false
    } else {
        return Err(MatchbookError::InvalidAccount.into());
    };
    if !vault.owned_by(tp.address()) {
        return Err(MatchbookError::InvalidAccount.into());
    }
    Ok(is_base)
}

fn deposit(accounts: &mut [AccountView], h: Header, r: &mut Reader<'_>) -> ProgramResult {
    let [owner, market, user_token, vault, mint, tp, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    require_signer(owner)?;
    require_market(market)?;
    let hint = h.hint;
    let amount = r.u64()?;
    let is_base = {
        let mut data = market.try_borrow_mut()?;
        let m = Market::load_mut(&mut data).map_err(engine)?;
        resolve_vault(&m, mint, vault, tp)?
    };
    let decimals = mint_decimals(mint, tp)?;
    let before = token_amount(vault)?;
    transfer(user_token, mint, vault, owner, tp, amount, decimals, None)?;
    let received = token_amount(vault)?.saturating_sub(before);
    let mut data = market.try_borrow_mut()?;
    let mut m = Market::load_mut(&mut data).map_err(engine)?;
    let t = m
        .resolve_trader(&owner_bytes(owner), Some(hint))
        .map_err(engine)?;
    let (b, q) = if is_base {
        (received, 0)
    } else {
        (0, received)
    };
    m.deposit(t, b, q).map_err(engine)
}

fn withdraw(accounts: &mut [AccountView], h: Header, r: &mut Reader<'_>) -> ProgramResult {
    let [owner, market, user_token, vault, mint, vault_authority, tp, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    require_signer(owner)?;
    require_market(market)?;
    let hint = h.hint;
    let amount = r.u64()?;
    let bump = {
        let mut data = market.try_borrow_mut()?;
        let mut m = Market::load_mut(&mut data).map_err(engine)?;
        let is_base = resolve_vault(&m, mint, vault, tp)?;
        let t = m
            .resolve_trader(&owner_bytes(owner), Some(hint))
            .map_err(engine)?;
        let (b, q) = if is_base { (amount, 0) } else { (0, amount) };
        m.withdraw(t, b, q).map_err(engine)?;
        [m.header().host_bump as u8]
    };
    let decimals = mint_decimals(mint, tp)?;
    let seeds = vault_seeds(market.address(), &bump);
    transfer(
        vault,
        mint,
        user_token,
        vault_authority,
        tp,
        amount,
        decimals,
        Some(&Signer::from(&seeds)),
    )
}

fn place_order(accounts: &mut [AccountView], h: Header, r: &mut Reader<'_>) -> ProgramResult {
    let [owner, market, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    require_signer(owner)?;
    require_market(market)?;
    let hint = h.hint;
    let p = r.pod::<OrderArgs>()?.to_params()?;
    if p.order_type != matchbook_engine::OrderType::ImmediateOrCancel
        && p.order_type != matchbook_engine::OrderType::FillOrKill
    {
        ensure_nodes(market, owner, 1)?;
    }
    let mut data = market.try_borrow_mut()?;
    let mut m = Market::load_mut(&mut data).map_err(engine)?;
    let a = m
        .resolve_actor(&owner_bytes(owner), Some(hint))
        .map_err(engine)?;
    let ts = LazyClock::default().now(&m, p.expires_at != 0 || a.expires_at != 0)?;
    let t = a.require(permission::TRADE, ts).map_err(engine)?;
    let mut s = LogSink::new();
    let out = m.place_order(t, &p, ts, &mut s).map_err(engine)?;
    set_return_data(&out.order_id.unwrap_or(0).to_le_bytes());
    Ok(())
}

fn cancel(accounts: &mut [AccountView], h: Header, r: &mut Reader<'_>) -> ProgramResult {
    let [owner, market, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    require_signer(owner)?;
    require_market(market)?;
    let mut data = market.try_borrow_mut()?;
    let mut m = Market::load_mut(&mut data).map_err(engine)?;
    let a = m
        .resolve_actor(&owner_bytes(owner), Some(h.hint))
        .map_err(engine)?;
    let ts = if a.expires_at != 0 { now()? } else { 0 };
    let t = a
        .require(permission::TRADE | permission::CANCEL, ts)
        .map_err(engine)?;
    let s = &mut LogSink::new();
    match h.ix {
        Ix::CancelOrder => m.cancel_order(t, r.u64()?, s),
        Ix::CancelAll => {
            let side = match h.a {
                0 => Some(Side::Bid),
                1 => Some(Side::Ask),
                _ => None,
            };
            m.cancel_all(t, side, s).map(|_| ())
        }
        Ix::CancelByClientId => m.cancel_by_client_id(t, r.u64()?, s).map(|_| ()),
        _ => {
            let id = r.u64()?;
            m.reduce_order(t, id, r.u64()?, s)
        }
    }
    .map_err(engine)
}

fn batch_update(accounts: &mut [AccountView], h: Header, r: &mut Reader<'_>) -> ProgramResult {
    let [owner, market, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    require_signer(owner)?;
    require_market(market)?;
    let (hint, cancel_all, n_cancels, n_orders) = (h.hint, h.a, h.b, h.c);
    if n_cancels > MAX_BATCH || n_orders > MAX_BATCH {
        return Err(MatchbookError::TooManyItems.into());
    }
    if n_orders > 0 {
        ensure_nodes(market, owner, u32::from(n_orders))?;
    }
    let any_expiry = r.any_expiry_after(usize::from(n_cancels));
    let mut data = market.try_borrow_mut()?;
    let mut m = Market::load_mut(&mut data).map_err(engine)?;
    let a = m
        .resolve_actor(&owner_bytes(owner), Some(hint))
        .map_err(engine)?;
    let ts = LazyClock::default().now(&m, any_expiry || a.expires_at != 0)?;
    let needed = if n_orders > 0 {
        permission::TRADE
    } else {
        permission::TRADE | permission::CANCEL
    };
    let t = a.require(needed, ts).map_err(engine)?;
    let s = &mut LogSink::new();
    if cancel_all != 0 {
        let side = match cancel_all {
            1 => Some(Side::Bid),
            2 => Some(Side::Ask),
            _ => None,
        };
        m.cancel_all(t, side, s).map_err(engine)?;
    }
    for _ in 0..n_cancels {
        let id = r.u64()?;
        // A cancel racing a fill is not an error for market makers.
        match m.cancel_order(t, id, s) {
            Ok(()) | Err(matchbook_engine::EngineError::OrderNotFound) => {}
            Err(e) => return Err(engine(e)),
        }
    }
    let mut ids = [0u64; MAX_BATCH as usize];
    for slot in ids.iter_mut().take(usize::from(n_orders)) {
        let p = r.pod::<OrderArgs>()?.to_params()?;
        let out = m.place_order(t, &p, ts, s).map_err(engine)?;
        *slot = out.order_id.unwrap_or(0);
    }
    if let Some(placed) = ids.get(..usize::from(n_orders)) {
        if !placed.is_empty() {
            set_return_data(bytemuck::cast_slice(placed));
        }
    }
    Ok(())
}

fn swap(accounts: &mut [AccountView], h: Header, r: &mut Reader<'_>) -> ProgramResult {
    let [user, market, user_base, user_quote, base_vault, quote_vault, base_mint, quote_mint, vault_authority, base_tp, quote_tp, ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    require_signer(user)?;
    require_market(market)?;
    let a: SwapArgs = r.pod()?;
    let side = Side::from_u8(h.a).ok_or(MatchbookError::InvalidInstructionData)?;
    let mut sink = LogSink::new();
    let (out, bump) = {
        let mut data = market.try_borrow_mut()?;
        let mut m = Market::load_mut(&mut data).map_err(engine)?;
        let ts = LazyClock::default().now(&m, false)?;
        if !resolve_vault(&m, base_mint, base_vault, base_tp)?
            || resolve_vault(&m, quote_mint, quote_vault, quote_tp)?
        {
            return Err(MatchbookError::InvalidAccount.into());
        }
        let out = m
            .swap(
                &SwapParams {
                    side,
                    base_lots: a.base_lots,
                    quote_budget: a.quote_budget,
                    limit_price: a.limit_price,
                    match_limit: if a.match_limit == 0 {
                        64
                    } else {
                        a.match_limit
                    },
                },
                ts,
                &mut sink,
            )
            .map_err(engine)?;
        (out, [m.header().host_bump as u8])
    };
    let base_dec = mint_decimals(base_mint, base_tp)?;
    let quote_dec = mint_decimals(quote_mint, quote_tp)?;
    let seeds = vault_seeds(market.address(), &bump);
    let signer = Signer::from(&seeds);
    let quote_net = out.quote_net(side);
    let (in_vault, owed) = match side {
        Side::Bid => (&*quote_vault, quote_net),
        Side::Ask => (&*base_vault, out.base_atoms),
    };
    let received = if side == Side::Bid {
        out.base_atoms
    } else {
        quote_net
    };
    if received < a.min_out {
        return Err(MatchbookError::SlippageExceeded.into());
    }
    let before = token_amount(in_vault)?;
    match side {
        Side::Bid => {
            transfer(
                user_quote,
                quote_mint,
                quote_vault,
                user,
                quote_tp,
                quote_net,
                quote_dec,
                None,
            )?;
            transfer(
                base_vault,
                base_mint,
                user_base,
                vault_authority,
                base_tp,
                out.base_atoms,
                base_dec,
                Some(&signer),
            )?;
        }
        Side::Ask => {
            transfer(
                user_base,
                base_mint,
                base_vault,
                user,
                base_tp,
                out.base_atoms,
                base_dec,
                None,
            )?;
            transfer(
                quote_vault,
                quote_mint,
                user_quote,
                vault_authority,
                quote_tp,
                quote_net,
                quote_dec,
                Some(&signer),
            )?;
        }
    }
    if token_amount(in_vault)?.saturating_sub(before) != owed {
        return Err(MatchbookError::TransferAmountMismatch.into());
    }
    Ok(())
}

fn collect_fees(accounts: &mut [AccountView]) -> ProgramResult {
    let [authority, market, destination, quote_vault, quote_mint, vault_authority, tp, ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    require_signer(authority)?;
    require_market(market)?;
    let (amount, bump) = {
        let mut data = market.try_borrow_mut()?;
        let mut m = Market::load_mut(&mut data).map_err(engine)?;
        require_key(authority, &m.header().authority).map_err(|_| MatchbookError::NotAuthority)?;
        if resolve_vault(&m, quote_mint, quote_vault, tp)? {
            return Err(MatchbookError::InvalidAccount.into());
        }
        (m.collect_fees(), [m.header().host_bump as u8])
    };
    let decimals = mint_decimals(quote_mint, tp)?;
    let seeds = vault_seeds(market.address(), &bump);
    transfer(
        quote_vault,
        quote_mint,
        destination,
        vault_authority,
        tp,
        amount,
        decimals,
        Some(&Signer::from(&seeds)),
    )
}

fn set_status(accounts: &mut [AccountView], h: Header) -> ProgramResult {
    let [authority, market, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    require_signer(authority)?;
    require_market(market)?;
    let status =
        MarketStatus::from_u32(u32::from(h.a)).ok_or(MatchbookError::InvalidInstructionData)?;
    let mut data = market.try_borrow_mut()?;
    let mut m = Market::load_mut(&mut data).map_err(engine)?;
    require_key(authority, &m.header().authority).map_err(|_| MatchbookError::NotAuthority)?;
    m.set_status(status).map_err(engine)
}

fn set_delegate(accounts: &mut [AccountView], h: Header, r: &mut Reader<'_>) -> ProgramResult {
    let [owner, market, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    require_signer(owner)?;
    require_market(market)?;
    let a: DelegateArgs = r.pod()?;
    ensure_nodes(market, owner, 1)?;
    let mut data = market.try_borrow_mut()?;
    let mut m = Market::load_mut(&mut data).map_err(engine)?;
    let t = m
        .resolve_actor(&owner_bytes(owner), Some(h.hint))
        .and_then(|x| x.require_owner())
        .map_err(engine)?;
    m.add_delegate(t, &a.delegate, a.expires_at, a.permissions)
        .map_err(engine)?;
    Ok(())
}

fn remove_delegate(accounts: &mut [AccountView], h: Header, r: &mut Reader<'_>) -> ProgramResult {
    let [signer, market, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    require_signer(signer)?;
    require_market(market)?;
    let delegate: [u8; 32] = r.pod()?;
    let mut data = market.try_borrow_mut()?;
    let mut m = Market::load_mut(&mut data).map_err(engine)?;
    let a = m
        .resolve_actor(&owner_bytes(signer), Some(h.hint))
        .map_err(engine)?;
    // A delegate may only revoke itself, whatever its permissions or expiry.
    if a.delegate && delegate != owner_bytes(signer) {
        return Err(engine(matchbook_engine::EngineError::NotPermitted));
    }
    m.remove_delegate(a.trader, &delegate).map_err(engine)
}

fn prune_expired(accounts: &mut [AccountView], h: Header) -> ProgramResult {
    let [market, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    require_market(market)?;
    let limit = h.hint;
    let ts = now()?;
    let mut data = market.try_borrow_mut()?;
    let mut m = Market::load_mut(&mut data).map_err(engine)?;
    m.prune_expired(ts, limit, &mut LogSink::new())
        .map_err(engine)?;
    Ok(())
}
