//! ConsumeEvents instruction for processing events from the event queue.
//!
//! This instruction processes events from the event queue and settles funds
//! to the appropriate OpenOrders accounts. This finalizes trades by updating balances.
//!
//! # Event Processing Flow
//!
//! 1. Read events from queue head
//! 2. For each event (up to limit):
//!    - If Fill event: settle maker and taker
//!    - If Out event: release locked funds
//!    - Pop event from queue
//! 3. Return number of events consumed
//!
//! # Notes
//!
//! - Permissionless: anyone can call
//! - OpenOrders accounts passed as remaining accounts
//! - Events processed in FIFO order
//! - ~10K CU base + ~15K CU per event
//! - Missing OpenOrders = event skipped (retry later)

use anchor_lang::prelude::*;

use crate::error::MatchbookError;
use crate::state::Side;

/// Parameters for consuming events.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ConsumeEventsParams {
    /// Maximum number of events to consume (for compute budget).
    pub limit: u16,
}

impl ConsumeEventsParams {
    /// Validates the consume events parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if limit is zero.
    pub fn validate(&self) -> Result<()> {
        require!(self.limit > 0, MatchbookError::InvalidQuantity);
        Ok(())
    }
}

/// Handler for the ConsumeEvents instruction.
///
/// Processes events from the event queue and settles funds to OpenOrders accounts.
///
/// # Arguments
///
/// * `ctx` - The instruction context containing all accounts
/// * `params` - Consume parameters (limit)
///
/// # Returns
///
/// The number of events consumed.
///
/// # Errors
///
/// Returns an error if:
/// - Limit is zero
pub fn handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, crate::ConsumeEvents<'info>>,
    params: ConsumeEventsParams,
) -> Result<u16> {
    // Validate parameters
    params.validate()?;

    let market = &ctx.accounts.market;
    let remaining_accounts = ctx.remaining_accounts;
    let mut consumed_count: u16 = 0;

    // Process events up to limit
    for _ in 0..params.limit {
        // Try to read and process the next event
        let event_result = read_next_event(&ctx.accounts.event_queue)?;

        let Some((event_type, event_data)) = event_result else {
            // No more events in queue
            break;
        };

        match event_type {
            1 => {
                // Fill event
                let processed = process_fill_event(
                    &event_data,
                    remaining_accounts,
                    market.base_lot_size,
                    market.quote_lot_size,
                )?;
                if processed {
                    pop_event(&ctx.accounts.event_queue)?;
                    consumed_count = consumed_count.saturating_add(1);
                } else {
                    // Missing OpenOrders, skip this event for now
                    break;
                }
            }
            2 => {
                // Out event
                let processed = process_out_event(&event_data, remaining_accounts)?;
                if processed {
                    pop_event(&ctx.accounts.event_queue)?;
                    consumed_count = consumed_count.saturating_add(1);
                } else {
                    // Missing OpenOrders, skip this event for now
                    break;
                }
            }
            _ => {
                // Unknown event type, pop and skip
                pop_event(&ctx.accounts.event_queue)?;
                consumed_count = consumed_count.saturating_add(1);
            }
        }
    }

    // Emit consume log
    msg!("Consumed {} events", consumed_count);

    Ok(consumed_count)
}

/// Reads the next event from the queue without removing it.
///
/// Returns None if the queue is empty.
/// Returns Some((event_type, event_data)) if an event exists.
#[allow(clippy::indexing_slicing)] // Bounds checked before indexing
fn read_next_event(event_queue: &AccountInfo) -> Result<Option<(u8, Vec<u8>)>> {
    let data = event_queue.try_borrow_data()?;

    // Skip discriminator (8 bytes)
    let header_offset = 8;

    // EventQueueHeader layout:
    // bump(1) + padding(7) + market(32) + head(4) + count(4) + seq_num(8) + reserved(64) = 120 bytes
    let head_offset = header_offset + 1 + 7 + 32;
    let count_offset = head_offset + 4;

    if data.len() < header_offset + 120 {
        return Err(MatchbookError::InvalidAccountData.into());
    }

    // Read current head and count
    let head_bytes: [u8; 4] = data[head_offset..head_offset + 4]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let head = u32::from_le_bytes(head_bytes);

    let count_bytes: [u8; 4] = data[count_offset..count_offset + 4]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let count = u32::from_le_bytes(count_bytes);

    if count == 0 {
        return Ok(None);
    }

    // Calculate events offset (after header)
    let events_offset = header_offset + 120;
    let event_size = 160;
    let remaining_space = data.len().saturating_sub(events_offset);
    let capacity = remaining_space / event_size;

    if capacity == 0 {
        return Ok(None);
    }

    // Calculate read position
    let event_offset = events_offset + (head as usize % capacity) * event_size;

    if data.len() < event_offset + event_size {
        return Err(MatchbookError::InvalidAccountData.into());
    }

    // Read event type
    let event_type = data[event_offset];

    // Read event data (excluding type byte)
    let event_data = data[event_offset + 1..event_offset + event_size].to_vec();

    Ok(Some((event_type, event_data)))
}

/// Pops the next event from the queue.
#[allow(clippy::indexing_slicing)] // Bounds checked before indexing
fn pop_event(event_queue: &AccountInfo) -> Result<()> {
    let mut data = event_queue.try_borrow_mut_data()?;

    // Skip discriminator (8 bytes)
    let header_offset = 8;

    let head_offset = header_offset + 1 + 7 + 32;
    let count_offset = head_offset + 4;

    if data.len() < header_offset + 120 {
        return Err(MatchbookError::InvalidAccountData.into());
    }

    // Read current head and count
    let head_bytes: [u8; 4] = data[head_offset..head_offset + 4]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let head = u32::from_le_bytes(head_bytes);

    let count_bytes: [u8; 4] = data[count_offset..count_offset + 4]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let count = u32::from_le_bytes(count_bytes);

    if count == 0 {
        return Ok(());
    }

    // Calculate capacity
    let events_offset = header_offset + 120;
    let event_size = 160;
    let remaining_space = data.len().saturating_sub(events_offset);
    let capacity = remaining_space / event_size;

    if capacity == 0 {
        return Ok(());
    }

    // Update head (wrap around)
    let new_head = (head as usize + 1) % capacity;
    data[head_offset..head_offset + 4].copy_from_slice(&(new_head as u32).to_le_bytes());

    // Decrement count
    let new_count = count.saturating_sub(1);
    data[count_offset..count_offset + 4].copy_from_slice(&new_count.to_le_bytes());

    Ok(())
}

/// Processes a Fill event.
///
/// Returns true if the event was processed successfully.
/// Returns false if required OpenOrders accounts are missing.
#[allow(clippy::indexing_slicing)] // Bounds checked before indexing
fn process_fill_event(
    event_data: &[u8],
    remaining_accounts: &[AccountInfo],
    base_lot_size: u64,
    quote_lot_size: u64,
) -> Result<bool> {
    // FillEvent layout (after type byte):
    // taker_side(1) + maker(32) + maker_order_id(16) + maker_client_order_id(8) +
    // taker(32) + taker_order_id(16) + taker_client_order_id(8) +
    // price(8) + quantity(8) + taker_fee(8) + maker_rebate(8)

    if event_data.len() < 145 {
        return Err(MatchbookError::InvalidAccountData.into());
    }

    let taker_side = Side::from(event_data[0]);

    let maker_bytes: [u8; 32] = event_data[1..33]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let maker_pubkey = Pubkey::new_from_array(maker_bytes);

    let taker_bytes: [u8; 32] = event_data[57..89]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let taker_pubkey = Pubkey::new_from_array(taker_bytes);

    let price_bytes: [u8; 8] = event_data[113..121]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let price = u64::from_le_bytes(price_bytes);

    let quantity_bytes: [u8; 8] = event_data[121..129]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let quantity = u64::from_le_bytes(quantity_bytes);

    let taker_fee_bytes: [u8; 8] = event_data[129..137]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let taker_fee = u64::from_le_bytes(taker_fee_bytes);

    let maker_rebate_bytes: [u8; 8] = event_data[137..145]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let maker_rebate = u64::from_le_bytes(maker_rebate_bytes);

    // Find maker and taker OpenOrders accounts
    let maker_account = remaining_accounts.iter().find(|a| a.key == &maker_pubkey);
    let taker_account = remaining_accounts.iter().find(|a| a.key == &taker_pubkey);

    let (Some(maker_account), Some(taker_account)) = (maker_account, taker_account) else {
        // Missing OpenOrders, skip this event
        return Ok(false);
    };

    // Calculate amounts
    let base_amount = quantity.checked_mul(base_lot_size).unwrap_or(0);
    let quote_amount = quantity
        .checked_mul(price)
        .and_then(|v| v.checked_mul(quote_lot_size))
        .and_then(|v| v.checked_div(base_lot_size))
        .unwrap_or(0);

    // Settle based on taker side
    match taker_side {
        Side::Bid => {
            // Taker is buying (bid): receives base, pays quote
            // Maker is selling (ask): receives quote, pays base

            // Taker: release quote_locked, credit base_free
            settle_taker_bid(taker_account, base_amount, quote_amount, taker_fee)?;

            // Maker: release base_locked, credit quote_free + rebate
            settle_maker_ask(maker_account, base_amount, quote_amount, maker_rebate)?;
        }
        Side::Ask => {
            // Taker is selling (ask): receives quote, pays base
            // Maker is buying (bid): receives base, pays quote

            // Taker: release base_locked, credit quote_free - fee
            settle_taker_ask(taker_account, base_amount, quote_amount, taker_fee)?;

            // Maker: release quote_locked, credit base_free
            settle_maker_bid(maker_account, base_amount, quote_amount, maker_rebate)?;
        }
    }

    Ok(true)
}

/// Settles a taker who is buying (bid side).
#[allow(clippy::indexing_slicing)]
fn settle_taker_bid(
    account: &AccountInfo,
    base_amount: u64,
    _quote_amount: u64,
    _taker_fee: u64,
) -> Result<()> {
    let mut data = account.try_borrow_mut_data()?;

    // OpenOrders layout (after discriminator):
    // bump(1) + padding(7) + market(32) + owner(32) + delegate(32) +
    // base_locked(8) + quote_locked(8) + base_free(8) + quote_free(8) + ...
    let base_free_offset = 8 + 1 + 7 + 32 + 32 + 32 + 8 + 8;
    let quote_locked_offset = 8 + 1 + 7 + 32 + 32 + 32 + 8;

    if data.len() < base_free_offset + 8 {
        return Err(MatchbookError::InvalidAccountData.into());
    }

    // Read current values
    let quote_locked_bytes: [u8; 8] = data[quote_locked_offset..quote_locked_offset + 8]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let quote_locked = u64::from_le_bytes(quote_locked_bytes);

    let base_free_bytes: [u8; 8] = data[base_free_offset..base_free_offset + 8]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let base_free = u64::from_le_bytes(base_free_bytes);

    // Taker buying: receives base, quote was already locked
    // Credit base_free
    let new_base_free = base_free.saturating_add(base_amount);
    data[base_free_offset..base_free_offset + 8].copy_from_slice(&new_base_free.to_le_bytes());

    // Debit quote_locked (quote was used to pay)
    let new_quote_locked = quote_locked.saturating_sub(_quote_amount.saturating_add(_taker_fee));
    data[quote_locked_offset..quote_locked_offset + 8]
        .copy_from_slice(&new_quote_locked.to_le_bytes());

    Ok(())
}

/// Settles a taker who is selling (ask side).
#[allow(clippy::indexing_slicing)]
fn settle_taker_ask(
    account: &AccountInfo,
    _base_amount: u64,
    quote_amount: u64,
    taker_fee: u64,
) -> Result<()> {
    let mut data = account.try_borrow_mut_data()?;

    // OpenOrders layout
    let base_locked_offset = 8 + 1 + 7 + 32 + 32 + 32;
    let quote_free_offset = 8 + 1 + 7 + 32 + 32 + 32 + 8 + 8 + 8;

    if data.len() < quote_free_offset + 8 {
        return Err(MatchbookError::InvalidAccountData.into());
    }

    // Read current values
    let base_locked_bytes: [u8; 8] = data[base_locked_offset..base_locked_offset + 8]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let base_locked = u64::from_le_bytes(base_locked_bytes);

    let quote_free_bytes: [u8; 8] = data[quote_free_offset..quote_free_offset + 8]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let quote_free = u64::from_le_bytes(quote_free_bytes);

    // Taker selling: receives quote - fee, base was already locked
    // Credit quote_free (minus fee)
    let new_quote_free = quote_free.saturating_add(quote_amount.saturating_sub(taker_fee));
    data[quote_free_offset..quote_free_offset + 8].copy_from_slice(&new_quote_free.to_le_bytes());

    // Debit base_locked (base was used to pay)
    let new_base_locked = base_locked.saturating_sub(_base_amount);
    data[base_locked_offset..base_locked_offset + 8]
        .copy_from_slice(&new_base_locked.to_le_bytes());

    Ok(())
}

/// Settles a maker who was selling (ask side).
#[allow(clippy::indexing_slicing)]
fn settle_maker_ask(
    account: &AccountInfo,
    _base_amount: u64,
    quote_amount: u64,
    maker_rebate: u64,
) -> Result<()> {
    let mut data = account.try_borrow_mut_data()?;

    // OpenOrders layout
    let base_locked_offset = 8 + 1 + 7 + 32 + 32 + 32;
    let quote_free_offset = 8 + 1 + 7 + 32 + 32 + 32 + 8 + 8 + 8;

    if data.len() < quote_free_offset + 8 {
        return Err(MatchbookError::InvalidAccountData.into());
    }

    // Read current values
    let base_locked_bytes: [u8; 8] = data[base_locked_offset..base_locked_offset + 8]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let base_locked = u64::from_le_bytes(base_locked_bytes);

    let quote_free_bytes: [u8; 8] = data[quote_free_offset..quote_free_offset + 8]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let quote_free = u64::from_le_bytes(quote_free_bytes);

    // Maker selling: receives quote + rebate, base was already locked
    // Credit quote_free (plus rebate)
    let new_quote_free = quote_free.saturating_add(quote_amount.saturating_add(maker_rebate));
    data[quote_free_offset..quote_free_offset + 8].copy_from_slice(&new_quote_free.to_le_bytes());

    // Debit base_locked (base was used to pay)
    let new_base_locked = base_locked.saturating_sub(_base_amount);
    data[base_locked_offset..base_locked_offset + 8]
        .copy_from_slice(&new_base_locked.to_le_bytes());

    Ok(())
}

/// Settles a maker who was buying (bid side).
#[allow(clippy::indexing_slicing)]
fn settle_maker_bid(
    account: &AccountInfo,
    base_amount: u64,
    _quote_amount: u64,
    _maker_rebate: u64,
) -> Result<()> {
    let mut data = account.try_borrow_mut_data()?;

    // OpenOrders layout
    let quote_locked_offset = 8 + 1 + 7 + 32 + 32 + 32 + 8;
    let base_free_offset = 8 + 1 + 7 + 32 + 32 + 32 + 8 + 8;

    if data.len() < base_free_offset + 8 {
        return Err(MatchbookError::InvalidAccountData.into());
    }

    // Read current values
    let quote_locked_bytes: [u8; 8] = data[quote_locked_offset..quote_locked_offset + 8]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let quote_locked = u64::from_le_bytes(quote_locked_bytes);

    let base_free_bytes: [u8; 8] = data[base_free_offset..base_free_offset + 8]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let base_free = u64::from_le_bytes(base_free_bytes);

    // Maker buying: receives base, quote was already locked
    // Credit base_free
    let new_base_free = base_free.saturating_add(base_amount);
    data[base_free_offset..base_free_offset + 8].copy_from_slice(&new_base_free.to_le_bytes());

    // Debit quote_locked (quote was used to pay, minus rebate)
    let new_quote_locked = quote_locked.saturating_sub(_quote_amount.saturating_sub(_maker_rebate));
    data[quote_locked_offset..quote_locked_offset + 8]
        .copy_from_slice(&new_quote_locked.to_le_bytes());

    Ok(())
}

/// Processes an Out event.
///
/// Returns true if the event was processed successfully.
/// Returns false if required OpenOrders account is missing.
#[allow(clippy::indexing_slicing)] // Bounds checked before indexing
fn process_out_event(event_data: &[u8], remaining_accounts: &[AccountInfo]) -> Result<bool> {
    // OutEvent layout (after type byte):
    // side(1) + owner(32) + order_id(16) + client_order_id(8) +
    // base_released(8) + quote_released(8) + reason(1)

    if event_data.len() < 74 {
        return Err(MatchbookError::InvalidAccountData.into());
    }

    let owner_bytes: [u8; 32] = event_data[1..33]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let owner_pubkey = Pubkey::new_from_array(owner_bytes);

    let base_released_bytes: [u8; 8] = event_data[57..65]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let base_released = u64::from_le_bytes(base_released_bytes);

    let quote_released_bytes: [u8; 8] = event_data[65..73]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let quote_released = u64::from_le_bytes(quote_released_bytes);

    // Find owner OpenOrders account
    let owner_account = remaining_accounts.iter().find(|a| a.key == &owner_pubkey);

    let Some(owner_account) = owner_account else {
        // Missing OpenOrders, skip this event
        return Ok(false);
    };

    // Release locked funds to free balance
    release_funds(owner_account, base_released, quote_released)?;

    Ok(true)
}

/// Releases locked funds to free balance.
#[allow(clippy::indexing_slicing)]
fn release_funds(account: &AccountInfo, base_released: u64, quote_released: u64) -> Result<()> {
    let mut data = account.try_borrow_mut_data()?;

    // OpenOrders layout
    let base_locked_offset = 8 + 1 + 7 + 32 + 32 + 32;
    let quote_locked_offset = base_locked_offset + 8;
    let base_free_offset = quote_locked_offset + 8;
    let quote_free_offset = base_free_offset + 8;

    if data.len() < quote_free_offset + 8 {
        return Err(MatchbookError::InvalidAccountData.into());
    }

    // Read current values
    let base_locked_bytes: [u8; 8] = data[base_locked_offset..base_locked_offset + 8]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let base_locked = u64::from_le_bytes(base_locked_bytes);

    let quote_locked_bytes: [u8; 8] = data[quote_locked_offset..quote_locked_offset + 8]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let quote_locked = u64::from_le_bytes(quote_locked_bytes);

    let base_free_bytes: [u8; 8] = data[base_free_offset..base_free_offset + 8]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let base_free = u64::from_le_bytes(base_free_bytes);

    let quote_free_bytes: [u8; 8] = data[quote_free_offset..quote_free_offset + 8]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let quote_free = u64::from_le_bytes(quote_free_bytes);

    // Release base
    if base_released > 0 {
        let new_base_locked = base_locked.saturating_sub(base_released);
        let new_base_free = base_free.saturating_add(base_released);
        data[base_locked_offset..base_locked_offset + 8]
            .copy_from_slice(&new_base_locked.to_le_bytes());
        data[base_free_offset..base_free_offset + 8].copy_from_slice(&new_base_free.to_le_bytes());
    }

    // Release quote
    if quote_released > 0 {
        let new_quote_locked = quote_locked.saturating_sub(quote_released);
        let new_quote_free = quote_free.saturating_add(quote_released);
        data[quote_locked_offset..quote_locked_offset + 8]
            .copy_from_slice(&new_quote_locked.to_le_bytes());
        data[quote_free_offset..quote_free_offset + 8]
            .copy_from_slice(&new_quote_free.to_le_bytes());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consume_events_params_valid() {
        let params = ConsumeEventsParams { limit: 10 };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_consume_events_params_zero_limit() {
        let params = ConsumeEventsParams { limit: 0 };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_consume_events_params_max_limit() {
        let params = ConsumeEventsParams { limit: u16::MAX };
        assert!(params.validate().is_ok());
    }
}
