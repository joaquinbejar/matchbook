//! CancelAllOrders instruction for batch cancellation of orders.
//!
//! This instruction cancels all open orders for a user in a single transaction,
//! useful for market makers and emergency situations.
//!
//! # Cancellation Flow
//!
//! 1. Validate authority (owner or delegate)
//! 2. Iterate through OpenOrders slots
//! 3. For each active order matching the side filter:
//!    - Remove order from book
//!    - Calculate and release locked funds
//!    - Clear order slot
//!    - Push Out event
//! 4. Stop when limit reached or no more orders
//!
//! # Notes
//!
//! - Limit parameter prevents compute budget exhaustion
//! - May need multiple calls to cancel all orders
//! - Cancellation is allowed even if market is paused (CancelOnly mode)
//! - Cancellation is NOT allowed if market is closed

use anchor_lang::prelude::*;

use crate::error::MatchbookError;
use crate::state::{OutEvent, OutReason, Side, MAX_ORDERS};

use super::cancel_order::{push_out_event, remove_order_from_book};

/// Parameters for cancelling all orders.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CancelAllOrdersParams {
    /// Side filter: None = both sides, Some(side) = only that side.
    pub side: Option<Side>,
    /// Maximum number of orders to cancel (for compute budget).
    pub limit: u8,
}

impl CancelAllOrdersParams {
    /// Validates the cancel all orders parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if limit is zero.
    pub fn validate(&self) -> Result<()> {
        require!(self.limit > 0, MatchbookError::InvalidQuantity);
        Ok(())
    }

    /// Returns true if the given side matches the filter.
    #[must_use]
    pub fn matches_side(&self, side: Side) -> bool {
        match self.side {
            None => true,
            Some(filter_side) => filter_side == side,
        }
    }
}

/// Handler for the CancelAllOrders instruction.
///
/// Cancels all orders for a user up to the specified limit.
///
/// # Arguments
///
/// * `ctx` - The instruction context containing all accounts
/// * `params` - Cancel parameters (side filter, limit)
///
/// # Returns
///
/// The number of orders cancelled.
///
/// # Errors
///
/// Returns an error if:
/// - Market is closed
/// - Authority is not authorized
/// - Limit is zero
pub fn handler(ctx: Context<crate::CancelAllOrders>, params: CancelAllOrdersParams) -> Result<u8> {
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

    let market = &ctx.accounts.market;
    let open_orders = &mut ctx.accounts.open_orders;
    let mut cancelled_count: u8 = 0;

    // Iterate through all order slots
    for slot_index in 0..MAX_ORDERS {
        // Check if we've reached the limit
        if cancelled_count >= params.limit {
            break;
        }

        // Get order slot
        let slot_idx = slot_index as u8;
        let order_slot = match open_orders.get_order(slot_idx) {
            Some(slot) if !slot.is_empty() => slot,
            _ => continue,
        };

        // Check if side matches filter
        if !params.matches_side(order_slot.side) {
            continue;
        }

        let order_id = order_slot.order_id;
        let client_order_id = order_slot.client_order_id;
        let side = order_slot.side;

        // Select the appropriate book account
        let book_account = match side {
            Side::Bid => &ctx.accounts.bids,
            Side::Ask => &ctx.accounts.asks,
        };

        // Try to remove order from book
        let (quantity, price) = match remove_order_from_book(book_account, order_id, side) {
            Ok(result) => result,
            Err(_) => {
                // Order not found in book, skip but still clear the slot
                open_orders.remove_order(slot_idx);
                continue;
            }
        };

        // Calculate locked amount to release
        let (base_released, quote_released) = match side {
            Side::Ask => {
                // Asks lock base tokens
                let base_amount = quantity.checked_mul(market.base_lot_size).unwrap_or(0);
                (base_amount, 0u64)
            }
            Side::Bid => {
                // Bids lock quote tokens
                let quote_amount = quantity
                    .checked_mul(price)
                    .and_then(|v| v.checked_mul(market.quote_lot_size))
                    .and_then(|v| v.checked_div(market.base_lot_size))
                    .unwrap_or(0);
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
        open_orders.remove_order(slot_idx);

        // Push Out event to event queue
        let out_event = OutEvent::new(
            side,
            open_orders.key(),
            order_id,
            client_order_id,
            base_released,
            quote_released,
            OutReason::Cancelled,
        );

        // Try to push event, but don't fail if queue is full
        let _ = push_out_event(&ctx.accounts.event_queue, out_event);

        cancelled_count = cancelled_count.saturating_add(1);
    }

    // Emit batch cancellation log
    msg!(
        "Cancelled {} orders, side_filter={:?}",
        cancelled_count,
        params.side
    );

    Ok(cancelled_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cancel_all_orders_params_valid() {
        let params = CancelAllOrdersParams {
            side: None,
            limit: 10,
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_cancel_all_orders_params_zero_limit() {
        let params = CancelAllOrdersParams {
            side: None,
            limit: 0,
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_cancel_all_orders_params_with_side_filter() {
        let params = CancelAllOrdersParams {
            side: Some(Side::Bid),
            limit: 5,
        };
        assert!(params.validate().is_ok());
        assert!(params.matches_side(Side::Bid));
        assert!(!params.matches_side(Side::Ask));
    }

    #[test]
    fn test_cancel_all_orders_params_no_side_filter() {
        let params = CancelAllOrdersParams {
            side: None,
            limit: 5,
        };
        assert!(params.matches_side(Side::Bid));
        assert!(params.matches_side(Side::Ask));
    }
}
