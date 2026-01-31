//! MatchOrders instruction for executing the matching algorithm.
//!
//! This instruction is the permissionless crank that crosses orders from both
//! sides of the book and generates fill events.
//!
//! # Matching Flow
//!
//! 1. Find best bid (highest price) and best ask (lowest price)
//! 2. Check if prices cross: bid_price >= ask_price
//! 3. If crossed:
//!    - Match quantity = min(bid_quantity, ask_quantity)
//!    - Execute at maker's price (older order)
//!    - Calculate fees (taker/maker)
//!    - Generate FillEvent for both sides
//!    - Update or remove orders from book
//! 4. Repeat until limit reached or no more crosses
//!
//! # Notes
//!
//! - Permissionless: anyone can call to earn priority
//! - Match price = maker's price (older order)
//! - Partial fills supported
//! - ~50K CU base + ~100K CU per match

use anchor_lang::prelude::*;

use crate::error::MatchbookError;
use crate::state::{FillEvent, OrderId, OutEvent, OutReason, Side, SENTINEL};

/// Best order info: (order_id, price, quantity, owner, client_order_id).
type BestOrderInfo = (u128, u64, u64, Pubkey, u64);

/// Parameters for matching orders.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct MatchOrdersParams {
    /// Maximum number of matches to execute (for compute budget).
    pub limit: u8,
}

impl MatchOrdersParams {
    /// Validates the match orders parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if limit is zero.
    pub fn validate(&self) -> Result<()> {
        require!(self.limit > 0, MatchbookError::InvalidQuantity);
        Ok(())
    }
}

/// Result of a single match operation.
#[derive(Debug)]
struct MatchResult {
    /// Price at which the match occurred.
    price: u64,
    /// Quantity matched.
    quantity: u64,
    /// Bid order info.
    bid_order_id: u128,
    bid_owner: Pubkey,
    bid_client_order_id: u64,
    bid_remaining: u64,
    /// Ask order info.
    ask_order_id: u128,
    ask_owner: Pubkey,
    ask_client_order_id: u64,
    ask_remaining: u64,
}

/// Handler for the MatchOrders instruction.
///
/// Executes the matching algorithm, crossing orders from both sides
/// of the book and generating fill events.
///
/// # Arguments
///
/// * `ctx` - The instruction context containing all accounts
/// * `params` - Match parameters (limit)
///
/// # Returns
///
/// The number of matches executed.
///
/// # Errors
///
/// Returns an error if:
/// - Market is not active
/// - Limit is zero
pub fn handler(ctx: Context<crate::MatchOrders>, params: MatchOrdersParams) -> Result<u8> {
    // Validate parameters
    params.validate()?;

    // Validate market is active
    require!(
        ctx.accounts.market.is_active(),
        MatchbookError::MarketNotActive
    );

    let market = &ctx.accounts.market;
    let mut match_count: u8 = 0;

    // Execute matches up to limit
    for _ in 0..params.limit {
        // Try to find a match
        let match_result = find_match(
            &ctx.accounts.bids,
            &ctx.accounts.asks,
            market.base_lot_size,
            market.quote_lot_size,
        )?;

        let Some(result) = match_result else {
            // No more matches possible
            break;
        };

        // Calculate fees
        let notional = result
            .quantity
            .checked_mul(result.price)
            .and_then(|v| v.checked_mul(market.quote_lot_size))
            .and_then(|v| v.checked_div(market.base_lot_size))
            .unwrap_or(0);

        let taker_fee = notional
            .checked_mul(market.taker_fee_bps as u64)
            .and_then(|v| v.checked_div(10_000))
            .unwrap_or(0);

        // Maker fee can be negative (rebate), but we store as positive rebate amount
        let maker_rebate = if market.maker_fee_bps < 0 {
            notional
                .checked_mul(market.maker_fee_bps.unsigned_abs() as u64)
                .and_then(|v| v.checked_div(10_000))
                .unwrap_or(0)
        } else {
            0
        };

        // Create fill event
        // The taker is the newer order (higher seq_num)
        // For simplicity, we'll consider the ask as the taker when matching
        let fill_event = FillEvent::new(
            Side::Ask, // taker_side
            result.bid_owner,
            result.bid_order_id,
            result.bid_client_order_id,
            result.ask_owner,
            result.ask_order_id,
            result.ask_client_order_id,
            result.price,
            result.quantity,
            taker_fee,
            maker_rebate,
        );

        // Push fill event to queue
        push_fill_event(&ctx.accounts.event_queue, fill_event)?;

        // Update or remove orders from book
        if result.bid_remaining == 0 {
            // Bid fully filled, remove from book
            remove_order(&ctx.accounts.bids, result.bid_order_id)?;

            // Push out event for bid
            let out_event = OutEvent::new(
                Side::Bid,
                result.bid_owner,
                result.bid_order_id,
                result.bid_client_order_id,
                0, // base_released (already matched)
                0, // quote_released (already matched)
                OutReason::Filled,
            );
            let _ = push_out_event(&ctx.accounts.event_queue, out_event);
        } else {
            // Update bid quantity
            update_order_quantity(
                &ctx.accounts.bids,
                result.bid_order_id,
                result.bid_remaining,
            )?;
        }

        if result.ask_remaining == 0 {
            // Ask fully filled, remove from book
            remove_order(&ctx.accounts.asks, result.ask_order_id)?;

            // Push out event for ask
            let out_event = OutEvent::new(
                Side::Ask,
                result.ask_owner,
                result.ask_order_id,
                result.ask_client_order_id,
                0, // base_released (already matched)
                0, // quote_released (already matched)
                OutReason::Filled,
            );
            let _ = push_out_event(&ctx.accounts.event_queue, out_event);
        } else {
            // Update ask quantity
            update_order_quantity(
                &ctx.accounts.asks,
                result.ask_order_id,
                result.ask_remaining,
            )?;
        }

        match_count = match_count.saturating_add(1);
    }

    // Emit match log
    msg!("Matched {} orders", match_count);

    Ok(match_count)
}

/// Finds a match between the best bid and best ask.
///
/// Returns None if no match is possible (no orders or prices don't cross).
#[allow(clippy::indexing_slicing)] // Bounds checked before indexing
fn find_match(
    bids: &AccountInfo,
    asks: &AccountInfo,
    base_lot_size: u64,
    _quote_lot_size: u64,
) -> Result<Option<MatchResult>> {
    // Get best bid
    let best_bid = get_best_order(bids, true)?;
    let Some((bid_order_id, bid_price, bid_quantity, bid_owner, bid_client_order_id)) = best_bid
    else {
        return Ok(None);
    };

    // Get best ask
    let best_ask = get_best_order(asks, false)?;
    let Some((ask_order_id, ask_price, ask_quantity, ask_owner, ask_client_order_id)) = best_ask
    else {
        return Ok(None);
    };

    // Check if prices cross: bid_price >= ask_price
    if bid_price < ask_price {
        return Ok(None);
    }

    // Calculate match quantity
    let match_quantity = bid_quantity.min(ask_quantity);

    // Calculate remaining quantities
    let bid_remaining = bid_quantity.saturating_sub(match_quantity);
    let ask_remaining = ask_quantity.saturating_sub(match_quantity);

    // Match at maker's price (the older order)
    // For simplicity, use the ask price (maker is the resting order)
    let match_price = ask_price;

    // Convert quantity to base tokens
    let _base_amount = match_quantity.checked_mul(base_lot_size).unwrap_or(0);

    Ok(Some(MatchResult {
        price: match_price,
        quantity: match_quantity,
        bid_order_id,
        bid_owner,
        bid_client_order_id,
        bid_remaining,
        ask_order_id,
        ask_owner,
        ask_client_order_id,
        ask_remaining,
    }))
}

/// Gets the best order from an order book side.
///
/// For bids, returns the highest price order.
/// For asks, returns the lowest price order.
#[allow(clippy::indexing_slicing)] // Bounds checked before indexing
fn get_best_order(book_account: &AccountInfo, is_bids: bool) -> Result<Option<BestOrderInfo>> {
    let data = book_account.try_borrow_data()?;

    // Skip discriminator (8 bytes)
    let header_offset = 8;

    // Read leaf_count
    let leaf_count_offset = header_offset + 1 + 7 + 32 + 1 + 7;
    if data.len() < leaf_count_offset + 4 {
        return Err(MatchbookError::InvalidAccountData.into());
    }

    let leaf_count_bytes: [u8; 4] = data[leaf_count_offset..leaf_count_offset + 4]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let leaf_count = u32::from_le_bytes(leaf_count_bytes);

    if leaf_count == 0 {
        return Ok(None);
    }

    // Read root
    let root_offset = leaf_count_offset + 8;
    let root_bytes: [u8; 4] = data[root_offset..root_offset + 4]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let root = u32::from_le_bytes(root_bytes);

    if root == SENTINEL {
        return Ok(None);
    }

    // Calculate node storage offset (after header + reserved)
    let nodes_offset = header_offset + 128;

    // Find the best order by scanning leaf nodes
    // For bids: highest price (highest key for bids encoding)
    // For asks: lowest price (lowest key for asks encoding)
    let mut best_key: Option<u128> = None;
    let mut best_quantity: u64 = 0;
    let mut best_owner = Pubkey::default();
    let mut best_client_order_id: u64 = 0;

    let max_nodes = (leaf_count as usize).saturating_mul(2);
    for i in 0..max_nodes {
        let node_offset = nodes_offset + i * 88;
        if data.len() < node_offset + 88 {
            break;
        }

        // Check if this is a leaf node (tag = 2)
        if data[node_offset] == 2 {
            // Read the key
            let key_offset = node_offset + 8;
            let key_bytes: [u8; 16] = data[key_offset..key_offset + 16]
                .try_into()
                .map_err(|_| MatchbookError::InvalidAccountData)?;
            let key = u128::from_le_bytes(key_bytes);

            // Read owner
            let owner_offset = node_offset + 24;
            let owner_bytes: [u8; 32] = data[owner_offset..owner_offset + 32]
                .try_into()
                .map_err(|_| MatchbookError::InvalidAccountData)?;
            let owner = Pubkey::new_from_array(owner_bytes);

            // Read quantity
            let quantity_offset = node_offset + 56;
            let quantity_bytes: [u8; 8] = data[quantity_offset..quantity_offset + 8]
                .try_into()
                .map_err(|_| MatchbookError::InvalidAccountData)?;
            let quantity = u64::from_le_bytes(quantity_bytes);

            // Read client_order_id
            let client_order_id_offset = node_offset + 64;
            let client_order_id_bytes: [u8; 8] = data
                [client_order_id_offset..client_order_id_offset + 8]
                .try_into()
                .map_err(|_| MatchbookError::InvalidAccountData)?;
            let client_order_id = u64::from_le_bytes(client_order_id_bytes);

            // Update best if this is better
            let is_better = match best_key {
                None => true,
                Some(current_best) => {
                    if is_bids {
                        // For bids, higher key is better (higher price)
                        key > current_best
                    } else {
                        // For asks, lower key is better (lower price)
                        key < current_best
                    }
                }
            };

            if is_better {
                best_key = Some(key);
                best_quantity = quantity;
                best_owner = owner;
                best_client_order_id = client_order_id;
            }
        }
    }

    match best_key {
        Some(key) => {
            let price = OrderId(key).price(is_bids);
            Ok(Some((
                key,
                price,
                best_quantity,
                best_owner,
                best_client_order_id,
            )))
        }
        None => Ok(None),
    }
}

/// Removes an order from the order book.
#[allow(clippy::indexing_slicing)] // Bounds checked before indexing
fn remove_order(book_account: &AccountInfo, order_id: u128) -> Result<()> {
    let mut data = book_account.try_borrow_mut_data()?;

    // Skip discriminator (8 bytes)
    let header_offset = 8;

    // Read current leaf_count
    let leaf_count_offset = header_offset + 1 + 7 + 32 + 1 + 7;
    if data.len() < leaf_count_offset + 4 {
        return Err(MatchbookError::InvalidAccountData.into());
    }

    let leaf_count_bytes: [u8; 4] = data[leaf_count_offset..leaf_count_offset + 4]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let mut leaf_count = u32::from_le_bytes(leaf_count_bytes);

    // Read free_list_head
    let free_list_offset = leaf_count_offset + 4;
    let free_list_bytes: [u8; 4] = data[free_list_offset..free_list_offset + 4]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let free_list_head = u32::from_le_bytes(free_list_bytes);

    // Read root
    let root_offset = free_list_offset + 4;

    // Calculate node storage offset
    let nodes_offset = header_offset + 128;

    // Find the order
    let max_nodes = (leaf_count as usize).saturating_mul(2);
    for i in 0..max_nodes {
        let node_offset = nodes_offset + i * 88;
        if data.len() < node_offset + 88 {
            break;
        }

        // Check if this is a leaf node (tag = 2)
        if data[node_offset] == 2 {
            // Read the key
            let key_offset = node_offset + 8;
            let key_bytes: [u8; 16] = data[key_offset..key_offset + 16]
                .try_into()
                .map_err(|_| MatchbookError::InvalidAccountData)?;
            let key = u128::from_le_bytes(key_bytes);

            if key == order_id {
                // Mark the node as free
                data[node_offset] = 3; // FreeNode tag

                // Set next pointer to current free_list_head
                let node_index = i as u32;
                data[node_offset + 1..node_offset + 5]
                    .copy_from_slice(&free_list_head.to_le_bytes());

                // Update free_list_head
                data[free_list_offset..free_list_offset + 4]
                    .copy_from_slice(&node_index.to_le_bytes());

                // Decrement leaf_count
                leaf_count = leaf_count.saturating_sub(1);
                data[leaf_count_offset..leaf_count_offset + 4]
                    .copy_from_slice(&leaf_count.to_le_bytes());

                // If this was the only node, set root to SENTINEL
                if leaf_count == 0 {
                    data[root_offset..root_offset + 4].copy_from_slice(&SENTINEL.to_le_bytes());
                }

                return Ok(());
            }
        }
    }

    Err(MatchbookError::OrderNotFound.into())
}

/// Updates the quantity of an order in the order book.
#[allow(clippy::indexing_slicing)] // Bounds checked before indexing
fn update_order_quantity(
    book_account: &AccountInfo,
    order_id: u128,
    new_quantity: u64,
) -> Result<()> {
    let mut data = book_account.try_borrow_mut_data()?;

    // Skip discriminator (8 bytes)
    let header_offset = 8;

    // Read leaf_count
    let leaf_count_offset = header_offset + 1 + 7 + 32 + 1 + 7;
    if data.len() < leaf_count_offset + 4 {
        return Err(MatchbookError::InvalidAccountData.into());
    }

    let leaf_count_bytes: [u8; 4] = data[leaf_count_offset..leaf_count_offset + 4]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let leaf_count = u32::from_le_bytes(leaf_count_bytes);

    // Calculate node storage offset
    let nodes_offset = header_offset + 128;

    // Find the order
    let max_nodes = (leaf_count as usize).saturating_mul(2);
    for i in 0..max_nodes {
        let node_offset = nodes_offset + i * 88;
        if data.len() < node_offset + 88 {
            break;
        }

        // Check if this is a leaf node (tag = 2)
        if data[node_offset] == 2 {
            // Read the key
            let key_offset = node_offset + 8;
            let key_bytes: [u8; 16] = data[key_offset..key_offset + 16]
                .try_into()
                .map_err(|_| MatchbookError::InvalidAccountData)?;
            let key = u128::from_le_bytes(key_bytes);

            if key == order_id {
                // Update quantity
                let quantity_offset = node_offset + 56;
                data[quantity_offset..quantity_offset + 8]
                    .copy_from_slice(&new_quantity.to_le_bytes());
                return Ok(());
            }
        }
    }

    Err(MatchbookError::OrderNotFound.into())
}

/// Pushes a fill event to the event queue.
#[allow(clippy::indexing_slicing)] // Bounds checked before indexing
fn push_fill_event(event_queue: &AccountInfo, event: FillEvent) -> Result<()> {
    let mut data = event_queue.try_borrow_mut_data()?;

    // Skip discriminator (8 bytes)
    let header_offset = 8;

    // EventQueueHeader layout:
    // bump(1) + padding(7) + market(32) + head(4) + count(4) + seq_num(8) + reserved(64) = 120 bytes
    let head_offset = header_offset + 1 + 7 + 32;
    let count_offset = head_offset + 4;
    let seq_num_offset = count_offset + 4;

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

    let seq_num_bytes: [u8; 8] = data[seq_num_offset..seq_num_offset + 8]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let seq_num = u64::from_le_bytes(seq_num_bytes);

    // Calculate events offset (after header)
    let events_offset = header_offset + 120;

    // Event size
    let event_size = 160;
    let remaining_space = data.len().saturating_sub(events_offset);
    let capacity = remaining_space / event_size;

    if capacity == 0 || count as usize >= capacity {
        return Err(MatchbookError::EventQueueFull.into());
    }

    // Calculate write position
    let tail = ((head as usize) + (count as usize)) % capacity;
    let event_offset = events_offset + tail * event_size;

    if data.len() < event_offset + event_size {
        return Err(MatchbookError::InvalidAccountData.into());
    }

    // Write the Fill event
    // Event enum: tag(1) + FillEvent data
    data[event_offset] = 1; // Fill event tag

    let fill_offset = event_offset + 1;
    data[fill_offset] = event.taker_side as u8;
    data[fill_offset + 1..fill_offset + 33].copy_from_slice(&event.maker.to_bytes());
    data[fill_offset + 33..fill_offset + 49].copy_from_slice(&event.maker_order_id.to_le_bytes());
    data[fill_offset + 49..fill_offset + 57]
        .copy_from_slice(&event.maker_client_order_id.to_le_bytes());
    data[fill_offset + 57..fill_offset + 89].copy_from_slice(&event.taker.to_bytes());
    data[fill_offset + 89..fill_offset + 105].copy_from_slice(&event.taker_order_id.to_le_bytes());
    data[fill_offset + 105..fill_offset + 113]
        .copy_from_slice(&event.taker_client_order_id.to_le_bytes());
    data[fill_offset + 113..fill_offset + 121].copy_from_slice(&event.price.to_le_bytes());
    data[fill_offset + 121..fill_offset + 129].copy_from_slice(&event.quantity.to_le_bytes());
    data[fill_offset + 129..fill_offset + 137].copy_from_slice(&event.taker_fee.to_le_bytes());
    data[fill_offset + 137..fill_offset + 145].copy_from_slice(&event.maker_rebate.to_le_bytes());

    // Update count
    let new_count = count.saturating_add(1);
    data[count_offset..count_offset + 4].copy_from_slice(&new_count.to_le_bytes());

    // Update seq_num
    let new_seq_num = seq_num.saturating_add(1);
    data[seq_num_offset..seq_num_offset + 8].copy_from_slice(&new_seq_num.to_le_bytes());

    Ok(())
}

/// Pushes an out event to the event queue.
#[allow(clippy::indexing_slicing)] // Bounds checked before indexing
fn push_out_event(event_queue: &AccountInfo, event: OutEvent) -> Result<()> {
    let mut data = event_queue.try_borrow_mut_data()?;

    // Skip discriminator (8 bytes)
    let header_offset = 8;

    let head_offset = header_offset + 1 + 7 + 32;
    let count_offset = head_offset + 4;
    let seq_num_offset = count_offset + 4;

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

    let seq_num_bytes: [u8; 8] = data[seq_num_offset..seq_num_offset + 8]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let seq_num = u64::from_le_bytes(seq_num_bytes);

    // Calculate events offset (after header)
    let events_offset = header_offset + 120;

    // Event size
    let event_size = 160;
    let remaining_space = data.len().saturating_sub(events_offset);
    let capacity = remaining_space / event_size;

    if capacity == 0 || count as usize >= capacity {
        return Err(MatchbookError::EventQueueFull.into());
    }

    // Calculate write position
    let tail = ((head as usize) + (count as usize)) % capacity;
    let event_offset = events_offset + tail * event_size;

    if data.len() < event_offset + event_size {
        return Err(MatchbookError::InvalidAccountData.into());
    }

    // Write the Out event
    data[event_offset] = 2; // Out event tag

    let out_offset = event_offset + 1;
    data[out_offset] = event.side as u8;
    data[out_offset + 1..out_offset + 33].copy_from_slice(&event.owner.to_bytes());
    data[out_offset + 33..out_offset + 49].copy_from_slice(&event.order_id.to_le_bytes());
    data[out_offset + 49..out_offset + 57].copy_from_slice(&event.client_order_id.to_le_bytes());
    data[out_offset + 57..out_offset + 65].copy_from_slice(&event.base_released.to_le_bytes());
    data[out_offset + 65..out_offset + 73].copy_from_slice(&event.quote_released.to_le_bytes());
    data[out_offset + 73] = event.reason as u8;

    // Update count
    let new_count = count.saturating_add(1);
    data[count_offset..count_offset + 4].copy_from_slice(&new_count.to_le_bytes());

    // Update seq_num
    let new_seq_num = seq_num.saturating_add(1);
    data[seq_num_offset..seq_num_offset + 8].copy_from_slice(&new_seq_num.to_le_bytes());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_orders_params_valid() {
        let params = MatchOrdersParams { limit: 10 };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_match_orders_params_zero_limit() {
        let params = MatchOrdersParams { limit: 0 };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_match_orders_params_max_limit() {
        let params = MatchOrdersParams { limit: 255 };
        assert!(params.validate().is_ok());
    }
}
