//! CancelOrder instruction for removing orders from the order book.
//!
//! This instruction handles order cancellation, releasing locked funds back
//! to the user's free balance and emitting an Out event for off-chain tracking.
//!
//! # Cancellation Flow
//!
//! 1. Validate authority (owner or delegate)
//! 2. Find order in OpenOrders by order_id
//! 3. Remove order from the book (bids or asks)
//! 4. Calculate and release locked funds
//! 5. Clear order slot in OpenOrders
//! 6. Push Out event to event queue
//!
//! # Notes
//!
//! - Cancellation is allowed even if market is paused (CancelOnly mode)
//! - Cancellation is NOT allowed if market is closed

use anchor_lang::prelude::*;

use crate::error::MatchbookError;
use crate::state::{OutEvent, OutReason, Side, SENTINEL};

/// Parameters for cancelling an order.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CancelOrderParams {
    /// Order ID to cancel.
    pub order_id: u128,
    /// Side of the order (Bid or Ask).
    pub side: Side,
}

impl CancelOrderParams {
    /// Validates the cancel order parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if order_id is zero.
    pub fn validate(&self) -> Result<()> {
        require!(self.order_id != 0, MatchbookError::InvalidClientOrderId);
        Ok(())
    }
}

/// Handler for the CancelOrder instruction.
///
/// Cancels a specific order from the order book.
///
/// # Arguments
///
/// * `ctx` - The instruction context containing all accounts
/// * `params` - Cancel parameters (order_id, side)
///
/// # Errors
///
/// Returns an error if:
/// - Market is closed
/// - Authority is not authorized
/// - Order not found in OpenOrders
/// - Order not found in book
pub fn handler(ctx: Context<crate::CancelOrder>, params: CancelOrderParams) -> Result<()> {
    // Validate parameters
    params.validate()?;

    // Validate market allows cancellations (not closed)
    require!(
        ctx.accounts.market.status.allows_cancellations(),
        MatchbookError::MarketClosed
    );

    // Validate authority is authorized (owner or delegate)
    require!(
        ctx.accounts
            .open_orders
            .is_authorized(ctx.accounts.authority.key),
        MatchbookError::Unauthorized
    );

    // Find order in OpenOrders
    let open_orders = &mut ctx.accounts.open_orders;
    let slot_index = open_orders
        .find_order(params.order_id)
        .ok_or(MatchbookError::OrderNotFound)?;

    // Get order slot to retrieve client_order_id
    let order_slot = open_orders
        .get_order(slot_index)
        .ok_or(MatchbookError::OrderNotFound)?;
    let client_order_id = order_slot.client_order_id;

    // Remove order from the book and get the quantity
    let book_account = match params.side {
        Side::Bid => &ctx.accounts.bids,
        Side::Ask => &ctx.accounts.asks,
    };

    let (quantity, price) = remove_order_from_book(book_account, params.order_id, params.side)?;

    // Calculate locked amount to release
    let market = &ctx.accounts.market;
    let (base_released, quote_released) = match params.side {
        Side::Ask => {
            // Asks lock base tokens
            let base_amount = quantity
                .checked_mul(market.base_lot_size)
                .ok_or(MatchbookError::ArithmeticOverflow)?;
            (base_amount, 0u64)
        }
        Side::Bid => {
            // Bids lock quote tokens
            let quote_amount = quantity
                .checked_mul(price)
                .ok_or(MatchbookError::ArithmeticOverflow)?
                .checked_mul(market.quote_lot_size)
                .ok_or(MatchbookError::ArithmeticOverflow)?
                .checked_div(market.base_lot_size)
                .ok_or(MatchbookError::ArithmeticOverflow)?;
            (0u64, quote_amount)
        }
    };

    // Release locked funds to free balance
    if base_released > 0 {
        open_orders.release_base(base_released);
    }
    if quote_released > 0 {
        open_orders.release_quote(quote_released);
    }

    // Clear order slot in OpenOrders
    open_orders.remove_order(slot_index);

    // Push Out event to event queue
    let out_event = OutEvent::new(
        params.side,
        open_orders.key(),
        params.order_id,
        client_order_id,
        base_released,
        quote_released,
        OutReason::Cancelled,
    );

    push_out_event(&ctx.accounts.event_queue, out_event)?;

    // Emit cancellation log
    msg!(
        "Order cancelled: id={}, side={:?}, base_released={}, quote_released={}",
        params.order_id,
        params.side,
        base_released,
        quote_released
    );

    Ok(())
}

/// Removes an order from the order book and returns its quantity and price.
///
/// # Arguments
///
/// * `book_account` - The order book account (bids or asks)
/// * `order_id` - The order ID to remove
/// * `side` - The side of the order
///
/// # Returns
///
/// A tuple of (quantity, price) for the removed order.
#[allow(clippy::indexing_slicing)] // Bounds checked before indexing
fn remove_order_from_book(
    book_account: &AccountInfo,
    order_id: u128,
    side: Side,
) -> Result<(u64, u64)> {
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
    let root_bytes: [u8; 4] = data[root_offset..root_offset + 4]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let root = u32::from_le_bytes(root_bytes);

    if root == SENTINEL {
        return Err(MatchbookError::OrderNotFound.into());
    }

    // Calculate node storage offset (after header + reserved)
    let nodes_offset = header_offset + 128; // header is 128 bytes

    // Search for the order in the tree
    // For simplicity, we'll do a linear scan of leaf nodes
    // A proper implementation would traverse the critbit tree
    let is_bid = side.is_bid();
    let mut found_index: Option<u32> = None;
    let mut found_quantity: u64 = 0;
    let mut found_price: u64 = 0;

    // Scan through potential nodes (up to leaf_count * 2 for inner + leaf nodes)
    let max_nodes = (leaf_count as usize).saturating_mul(2);
    for i in 0..max_nodes {
        let node_offset = nodes_offset + i * 88;
        if data.len() < node_offset + 88 {
            break;
        }

        // Check if this is a leaf node (tag = 2)
        if data[node_offset] == 2 {
            // Read the key from the leaf node
            // LeafNode layout: tag(1) + owner_slot(1) + time_in_force(1) + padding(5) + key(16) + ...
            let key_offset = node_offset + 8;
            let key_bytes: [u8; 16] = data[key_offset..key_offset + 16]
                .try_into()
                .map_err(|_| MatchbookError::InvalidAccountData)?;
            let key = u128::from_le_bytes(key_bytes);

            if key == order_id {
                // Found the order, read quantity
                let quantity_offset = node_offset + 56;
                let quantity_bytes: [u8; 8] = data[quantity_offset..quantity_offset + 8]
                    .try_into()
                    .map_err(|_| MatchbookError::InvalidAccountData)?;
                found_quantity = u64::from_le_bytes(quantity_bytes);

                // Extract price from order_id
                found_price = crate::state::OrderId(order_id).price(is_bid);

                found_index = Some(i as u32);
                break;
            }
        }
    }

    let node_index = found_index.ok_or(MatchbookError::OrderNotFound)?;

    // Mark the node as free
    let node_offset = nodes_offset + (node_index as usize) * 88;

    // Set tag to FreeNode (3)
    data[node_offset] = 3;

    // Set next pointer to current free_list_head
    data[node_offset + 1..node_offset + 5].copy_from_slice(&free_list_head.to_le_bytes());

    // Update free_list_head to point to this node
    data[free_list_offset..free_list_offset + 4].copy_from_slice(&node_index.to_le_bytes());

    // Decrement leaf_count
    leaf_count = leaf_count.saturating_sub(1);
    data[leaf_count_offset..leaf_count_offset + 4].copy_from_slice(&leaf_count.to_le_bytes());

    // If this was the only node (root), set root to SENTINEL
    if leaf_count == 0 {
        data[root_offset..root_offset + 4].copy_from_slice(&SENTINEL.to_le_bytes());
    }

    Ok((found_quantity, found_price))
}

/// Pushes an Out event to the event queue.
///
/// # Arguments
///
/// * `event_queue` - The event queue account
/// * `event` - The Out event to push
#[allow(clippy::indexing_slicing)] // Bounds checked before indexing
fn push_out_event(event_queue: &AccountInfo, event: OutEvent) -> Result<()> {
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

    // Calculate capacity based on remaining space
    // Event size is approximately 160 bytes (based on Event enum)
    let event_size = 160;
    let remaining_space = data.len().saturating_sub(events_offset);
    let capacity = remaining_space / event_size;

    if capacity == 0 {
        return Err(MatchbookError::EventQueueFull.into());
    }

    // Check if queue is full
    if count as usize >= capacity {
        return Err(MatchbookError::EventQueueFull.into());
    }

    // Calculate write position (tail = (head + count) % capacity)
    let tail = ((head as usize) + (count as usize)) % capacity;
    let event_offset = events_offset + tail * event_size;

    if data.len() < event_offset + event_size {
        return Err(MatchbookError::InvalidAccountData.into());
    }

    // Write the Out event
    // Event enum: tag(1) + OutEvent data
    // OutEvent: side(1) + owner(32) + order_id(16) + client_order_id(8) + base_released(8) + quote_released(8) + reason(1)
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
    fn test_cancel_order_params_valid() {
        let params = CancelOrderParams {
            order_id: 12345,
            side: Side::Bid,
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_cancel_order_params_zero_order_id() {
        let params = CancelOrderParams {
            order_id: 0,
            side: Side::Bid,
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_cancel_order_params_ask_side() {
        let params = CancelOrderParams {
            order_id: 99999,
            side: Side::Ask,
        };
        assert!(params.validate().is_ok());
    }
}
