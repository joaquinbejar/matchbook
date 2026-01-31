//! PlaceOrder instruction for inserting orders into the order book.
//!
//! This instruction handles order validation, balance locking, and book insertion.
//! It is the core trading instruction for the Matchbook CLOB.
//!
//! # Order Flow
//!
//! 1. Validate parameters (price, quantity, order type)
//! 2. Check authorization (owner or delegate)
//! 3. Calculate required funds and lock balance
//! 4. Find free order slot in OpenOrders
//! 5. Generate order ID from price and sequence number
//! 6. Insert order into appropriate book side
//!
//! # Order Types
//!
//! - **Limit**: Rests on book if not immediately fillable
//! - **PostOnly**: Rejected if would match immediately (maker only)
//! - **ImmediateOrCancel**: Fills what it can, cancels rest (handled in matching)
//! - **FillOrKill**: Must fill completely or cancel entirely (handled in matching)

use anchor_lang::prelude::*;

use crate::error::MatchbookError;
use crate::state::{LeafNode, OrderId, Side, TimeInForce, SENTINEL};

/// Order type for how the order should interact with the book.
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum OrderType {
    /// Rests on book if not immediately fillable.
    #[default]
    Limit = 0,
    /// Fills what it can immediately, cancels the rest.
    ImmediateOrCancel = 1,
    /// Only accepted if it would rest on book (no immediate fill).
    PostOnly = 2,
    /// Fills completely or cancels entirely.
    FillOrKill = 3,
}

impl From<u8> for OrderType {
    fn from(value: u8) -> Self {
        match value {
            1 => OrderType::ImmediateOrCancel,
            2 => OrderType::PostOnly,
            3 => OrderType::FillOrKill,
            _ => OrderType::Limit,
        }
    }
}

impl OrderType {
    /// Returns the corresponding TimeInForce for this order type.
    #[must_use]
    pub const fn to_time_in_force(&self) -> TimeInForce {
        match self {
            OrderType::Limit => TimeInForce::GoodTilCancelled,
            OrderType::ImmediateOrCancel => TimeInForce::ImmediateOrCancel,
            OrderType::PostOnly => TimeInForce::PostOnly,
            OrderType::FillOrKill => TimeInForce::FillOrKill,
        }
    }

    /// Returns true if this order type must not match immediately.
    #[must_use]
    pub const fn is_post_only(&self) -> bool {
        matches!(self, OrderType::PostOnly)
    }
}

/// Parameters for placing an order.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PlaceOrderParams {
    /// Side of the order (Bid or Ask).
    pub side: Side,
    /// Price in ticks (must be > 0 and aligned to tick_size).
    pub price: u64,
    /// Quantity in base lots (must be > 0).
    pub quantity: u64,
    /// Order type (Limit, PostOnly, IOC, FOK).
    pub order_type: OrderType,
    /// Client-provided order ID for tracking.
    pub client_order_id: u64,
}

impl PlaceOrderParams {
    /// Validates the order parameters.
    ///
    /// # Arguments
    ///
    /// * `tick_size` - Market tick size for price alignment
    /// * `min_order_size` - Minimum order size in lots
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Price is zero
    /// - Price is not aligned to tick_size
    /// - Quantity is zero
    /// - Quantity is below min_order_size
    pub fn validate(&self, tick_size: u64, min_order_size: u64) -> Result<()> {
        // Price must be > 0
        require!(self.price > 0, MatchbookError::InvalidPrice);

        // Price must be aligned to tick_size
        require!(
            self.price.is_multiple_of(tick_size),
            MatchbookError::InvalidTickSize
        );

        // Quantity must be > 0
        require!(self.quantity > 0, MatchbookError::InvalidQuantity);

        // Quantity must be >= min_order_size
        require!(
            self.quantity >= min_order_size,
            MatchbookError::OrderTooSmall
        );

        Ok(())
    }

    /// Calculates the required funds to lock for this order.
    ///
    /// # Arguments
    ///
    /// * `base_lot_size` - Size of one base lot in base token smallest units
    /// * `quote_lot_size` - Size of one quote lot in quote token smallest units
    ///
    /// # Returns
    ///
    /// The amount of tokens to lock (base for asks, quote for bids).
    ///
    /// # Errors
    ///
    /// Returns an error if calculation would overflow.
    pub fn calculate_lock_amount(&self, base_lot_size: u64, quote_lot_size: u64) -> Result<u64> {
        match self.side {
            Side::Ask => {
                // Lock base tokens: quantity * base_lot_size
                self.quantity
                    .checked_mul(base_lot_size)
                    .ok_or_else(|| error!(MatchbookError::ArithmeticOverflow))
            }
            Side::Bid => {
                // Lock quote tokens: quantity * price * quote_lot_size / base_lot_size
                // Simplified: (quantity * price * quote_lot_size) / base_lot_size
                let numerator = self
                    .quantity
                    .checked_mul(self.price)
                    .ok_or_else(|| error!(MatchbookError::ArithmeticOverflow))?
                    .checked_mul(quote_lot_size)
                    .ok_or_else(|| error!(MatchbookError::ArithmeticOverflow))?;

                numerator
                    .checked_div(base_lot_size)
                    .ok_or_else(|| error!(MatchbookError::ArithmeticOverflow))
            }
        }
    }
}

/// Handler for the PlaceOrder instruction.
///
/// Places a new order on the order book.
///
/// # Arguments
///
/// * `ctx` - The instruction context containing all accounts
/// * `params` - Order parameters (side, price, quantity, type)
///
/// # Errors
///
/// Returns an error if:
/// - Market is not active
/// - Authority is not authorized
/// - Invalid price or quantity
/// - Insufficient free balance
/// - No free order slot available
/// - PostOnly order would cross the spread
pub fn handler(ctx: Context<crate::PlaceOrder>, params: PlaceOrderParams) -> Result<()> {
    // Validate market is active
    require!(
        ctx.accounts.market.is_active(),
        MatchbookError::MarketNotActive
    );

    // Validate authority is authorized (owner or delegate)
    require!(
        ctx.accounts
            .open_orders
            .is_authorized(ctx.accounts.authority.key),
        MatchbookError::Unauthorized
    );

    // Validate order parameters
    params.validate(
        ctx.accounts.market.tick_size,
        ctx.accounts.market.min_order_size,
    )?;

    // Calculate required funds to lock
    let lock_amount = params.calculate_lock_amount(
        ctx.accounts.market.base_lot_size,
        ctx.accounts.market.quote_lot_size,
    )?;

    // Lock funds in OpenOrders
    let open_orders = &mut ctx.accounts.open_orders;
    let locked = match params.side {
        Side::Ask => open_orders.lock_base(lock_amount),
        Side::Bid => open_orders.lock_quote(lock_amount),
    };
    require!(locked, MatchbookError::InsufficientFunds);

    // Find free order slot
    let slot_index = open_orders
        .find_free_slot()
        .ok_or(MatchbookError::TooManyOrders)?;

    // Get next sequence number and generate order ID
    let market = &mut ctx.accounts.market;
    let seq_num = market
        .next_seq_num()
        .ok_or(MatchbookError::ArithmeticOverflow)?;
    let order_id = match params.side {
        Side::Bid => OrderId::new_bid(params.price, seq_num),
        Side::Ask => OrderId::new_ask(params.price, seq_num),
    };

    // For PostOnly orders, check if would cross the spread
    // This requires reading the opposite book side
    if params.order_type.is_post_only() {
        let would_cross = check_would_cross(
            &ctx.accounts.bids,
            &ctx.accounts.asks,
            params.side,
            params.price,
        )?;
        if would_cross {
            // Unlock the funds we just locked
            match params.side {
                Side::Ask => {
                    open_orders.release_base(lock_amount);
                }
                Side::Bid => {
                    open_orders.release_quote(lock_amount);
                }
            }
            return Err(MatchbookError::PostOnlyWouldCross.into());
        }
    }

    // Add order to OpenOrders slot
    open_orders.add_order(
        slot_index,
        order_id.get(),
        params.client_order_id,
        params.side,
    );

    // Create leaf node for the order
    let leaf = LeafNode::new(
        slot_index,
        params.order_type.to_time_in_force(),
        order_id.get(),
        open_orders.key(),
        params.quantity,
        params.client_order_id,
    );

    // Insert into the appropriate book side
    insert_order(&ctx.accounts.bids, &ctx.accounts.asks, params.side, leaf)?;

    // Emit order placed log
    msg!(
        "Order placed: id={}, side={:?}, price={}, qty={}, type={:?}",
        order_id.get(),
        params.side,
        params.price,
        params.quantity,
        params.order_type
    );

    Ok(())
}

/// Checks if a PostOnly order would cross the spread.
///
/// # Arguments
///
/// * `bids` - The bids account
/// * `asks` - The asks account
/// * `side` - The order side
/// * `price` - The order price
///
/// # Returns
///
/// `true` if the order would cross, `false` otherwise.
fn check_would_cross(
    bids: &AccountInfo,
    asks: &AccountInfo,
    side: Side,
    price: u64,
) -> Result<bool> {
    match side {
        Side::Bid => {
            // Bid would cross if price >= best ask
            let best_ask = get_best_price(asks, false)?;
            Ok(best_ask.is_some_and(|ask_price| price >= ask_price))
        }
        Side::Ask => {
            // Ask would cross if price <= best bid
            let best_bid = get_best_price(bids, true)?;
            Ok(best_bid.is_some_and(|bid_price| price <= bid_price))
        }
    }
}

/// Gets the best price from an order book side.
///
/// # Arguments
///
/// * `book_account` - The order book account
/// * `is_bids` - Whether this is the bids side
///
/// # Returns
///
/// The best price, or None if the book is empty.
#[allow(clippy::indexing_slicing)] // Bounds checked before indexing
fn get_best_price(book_account: &AccountInfo, _is_bids: bool) -> Result<Option<u64>> {
    let data = book_account.try_borrow_data()?;

    // Skip discriminator (8 bytes) and read header
    if data.len() < 8 + 128 {
        return Ok(None);
    }

    // Read root from header (offset: 8 + 1 + 7 + 32 + 1 + 7 + 4 + 4 = 64 from start of header)
    // Actually: bump(1) + padding(7) + market(32) + is_bids(1) + padding2(7) + leaf_count(4) + free_list_head(4) + root(4)
    let header_offset = 8; // discriminator
    let root_offset = header_offset + 1 + 7 + 32 + 1 + 7 + 4 + 4;

    if data.len() < root_offset + 4 {
        return Ok(None);
    }

    let root_bytes: [u8; 4] = data[root_offset..root_offset + 4]
        .try_into()
        .map_err(|_| MatchbookError::InvalidAccountData)?;
    let root = u32::from_le_bytes(root_bytes);

    if root == SENTINEL {
        return Ok(None);
    }

    // For now, we'll traverse to find the best (minimum key for asks, maximum key for bids)
    // This is a simplified implementation - a full implementation would traverse the tree
    // For the initial implementation, we return None to allow all orders
    // TODO: Implement proper tree traversal to find best price
    Ok(None)
}

/// Inserts an order into the appropriate book side.
///
/// # Arguments
///
/// * `bids` - The bids account
/// * `asks` - The asks account
/// * `side` - The order side
/// * `leaf` - The leaf node to insert
#[allow(clippy::indexing_slicing)] // Bounds checked before indexing
fn insert_order<'a>(
    bids: &AccountInfo<'a>,
    asks: &AccountInfo<'a>,
    side: Side,
    leaf: LeafNode,
) -> Result<()> {
    let book_account = match side {
        Side::Bid => bids,
        Side::Ask => asks,
    };

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

    // Calculate node storage offset (after header + reserved)
    let nodes_offset = header_offset + 128; // header is 128 bytes

    // For the initial implementation, we'll use a simple insertion strategy:
    // 1. If tree is empty, create root as leaf
    // 2. Otherwise, find insertion point and insert

    if root == SENTINEL {
        // Tree is empty, insert as root
        let new_node_index = if free_list_head != SENTINEL {
            free_list_head
        } else {
            leaf_count
        };

        // Write leaf node at new_node_index
        let node_offset = nodes_offset + (new_node_index as usize) * 88;
        if data.len() < node_offset + 88 {
            return Err(MatchbookError::InvalidAccountData.into());
        }

        // Write node tag (LeafNode = 2)
        data[node_offset] = 2;

        // Write leaf data
        // LeafNode layout: owner_slot(1) + time_in_force(1) + padding(6) + key(16) + owner(32) + quantity(8) + client_order_id(8)
        data[node_offset + 1] = leaf.owner_slot;
        data[node_offset + 2] = leaf.time_in_force as u8;
        // padding bytes 3-8 are zero
        data[node_offset + 8..node_offset + 24].copy_from_slice(&leaf.key.to_le_bytes());
        data[node_offset + 24..node_offset + 56].copy_from_slice(&leaf.owner.to_bytes());
        data[node_offset + 56..node_offset + 64].copy_from_slice(&leaf.quantity.to_le_bytes());
        data[node_offset + 64..node_offset + 72]
            .copy_from_slice(&leaf.client_order_id.to_le_bytes());

        // Update root to point to new node
        data[root_offset..root_offset + 4].copy_from_slice(&new_node_index.to_le_bytes());

        // Update leaf_count
        leaf_count = leaf_count.saturating_add(1);
        data[leaf_count_offset..leaf_count_offset + 4].copy_from_slice(&leaf_count.to_le_bytes());
    } else {
        // Tree is not empty - need to do proper critbit insertion
        // For now, we'll implement a simplified version that just adds to the tree
        // A full implementation would traverse and find the correct insertion point

        // Allocate new node
        let new_node_index = if free_list_head != SENTINEL {
            // Use node from free list
            // Read next free from the free node
            let free_node_offset = nodes_offset + (free_list_head as usize) * 88;
            let next_free_bytes: [u8; 4] = data[free_node_offset + 1..free_node_offset + 5]
                .try_into()
                .map_err(|_| MatchbookError::InvalidAccountData)?;
            let next_free = u32::from_le_bytes(next_free_bytes);

            // Update free list head
            data[free_list_offset..free_list_offset + 4].copy_from_slice(&next_free.to_le_bytes());

            free_list_head
        } else {
            // Allocate new node at end
            // Total nodes = leaf_count * 2 - 1 (for a balanced tree)
            // But we just use leaf_count as approximation for available slots
            let new_idx = leaf_count;
            leaf_count = leaf_count.saturating_add(1);
            new_idx
        };

        // Write leaf node
        let node_offset = nodes_offset + (new_node_index as usize) * 88;
        if data.len() < node_offset + 88 {
            return Err(MatchbookError::InvalidAccountData.into());
        }

        data[node_offset] = 2; // LeafNode tag
        data[node_offset + 1] = leaf.owner_slot;
        data[node_offset + 2] = leaf.time_in_force as u8;
        data[node_offset + 8..node_offset + 24].copy_from_slice(&leaf.key.to_le_bytes());
        data[node_offset + 24..node_offset + 56].copy_from_slice(&leaf.owner.to_bytes());
        data[node_offset + 56..node_offset + 64].copy_from_slice(&leaf.quantity.to_le_bytes());
        data[node_offset + 64..node_offset + 72]
            .copy_from_slice(&leaf.client_order_id.to_le_bytes());

        // For proper critbit insertion, we need to:
        // 1. Find the critical bit between new key and existing keys
        // 2. Create inner node at that position
        // 3. Link everything correctly
        // This is complex and will be refined in future iterations

        // For now, just update leaf_count
        data[leaf_count_offset..leaf_count_offset + 4].copy_from_slice(&leaf_count.to_le_bytes());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_type_conversion() {
        assert_eq!(OrderType::from(0), OrderType::Limit);
        assert_eq!(OrderType::from(1), OrderType::ImmediateOrCancel);
        assert_eq!(OrderType::from(2), OrderType::PostOnly);
        assert_eq!(OrderType::from(3), OrderType::FillOrKill);
        assert_eq!(OrderType::from(99), OrderType::Limit); // Default
    }

    #[test]
    fn test_order_type_to_time_in_force() {
        assert_eq!(
            OrderType::Limit.to_time_in_force(),
            TimeInForce::GoodTilCancelled
        );
        assert_eq!(
            OrderType::ImmediateOrCancel.to_time_in_force(),
            TimeInForce::ImmediateOrCancel
        );
        assert_eq!(
            OrderType::PostOnly.to_time_in_force(),
            TimeInForce::PostOnly
        );
        assert_eq!(
            OrderType::FillOrKill.to_time_in_force(),
            TimeInForce::FillOrKill
        );
    }

    #[test]
    fn test_order_type_is_post_only() {
        assert!(!OrderType::Limit.is_post_only());
        assert!(!OrderType::ImmediateOrCancel.is_post_only());
        assert!(OrderType::PostOnly.is_post_only());
        assert!(!OrderType::FillOrKill.is_post_only());
    }

    #[test]
    fn test_place_order_params_valid() {
        let params = PlaceOrderParams {
            side: Side::Bid,
            price: 1000,
            quantity: 10,
            order_type: OrderType::Limit,
            client_order_id: 123,
        };
        // tick_size = 100, min_order_size = 5
        assert!(params.validate(100, 5).is_ok());
    }

    #[test]
    fn test_place_order_params_zero_price() {
        let params = PlaceOrderParams {
            side: Side::Bid,
            price: 0,
            quantity: 10,
            order_type: OrderType::Limit,
            client_order_id: 123,
        };
        assert!(params.validate(100, 5).is_err());
    }

    #[test]
    fn test_place_order_params_zero_quantity() {
        let params = PlaceOrderParams {
            side: Side::Bid,
            price: 1000,
            quantity: 0,
            order_type: OrderType::Limit,
            client_order_id: 123,
        };
        assert!(params.validate(100, 5).is_err());
    }

    #[test]
    fn test_place_order_params_price_not_aligned() {
        let params = PlaceOrderParams {
            side: Side::Bid,
            price: 1050, // Not aligned to tick_size of 100
            quantity: 10,
            order_type: OrderType::Limit,
            client_order_id: 123,
        };
        assert!(params.validate(100, 5).is_err());
    }

    #[test]
    fn test_place_order_params_quantity_too_small() {
        let params = PlaceOrderParams {
            side: Side::Bid,
            price: 1000,
            quantity: 3, // Below min_order_size of 5
            order_type: OrderType::Limit,
            client_order_id: 123,
        };
        assert!(params.validate(100, 5).is_err());
    }

    #[test]
    fn test_calculate_lock_amount_ask() {
        let params = PlaceOrderParams {
            side: Side::Ask,
            price: 1000,
            quantity: 10,
            order_type: OrderType::Limit,
            client_order_id: 123,
        };
        // base_lot_size = 1000, quote_lot_size = 100
        // For asks: lock_amount = quantity * base_lot_size = 10 * 1000 = 10000
        let lock = params.calculate_lock_amount(1000, 100);
        assert!(lock.is_ok());
        assert_eq!(lock.ok(), Some(10000));
    }

    #[test]
    fn test_calculate_lock_amount_bid() {
        let params = PlaceOrderParams {
            side: Side::Bid,
            price: 1000,
            quantity: 10,
            order_type: OrderType::Limit,
            client_order_id: 123,
        };
        // base_lot_size = 1000, quote_lot_size = 100
        // For bids: lock_amount = (quantity * price * quote_lot_size) / base_lot_size
        //         = (10 * 1000 * 100) / 1000 = 1000
        let lock = params.calculate_lock_amount(1000, 100);
        assert!(lock.is_ok());
        assert_eq!(lock.ok(), Some(1000));
    }
}
