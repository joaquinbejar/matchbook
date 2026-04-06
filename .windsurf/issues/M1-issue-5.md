# M1 Issue #5: OpenOrders Account Structure

## Overview

Implement the OpenOrders account that tracks a user's open orders and token balances within a specific market. Each user needs one OpenOrders account per market they trade on.

## Acceptance Criteria

- [ ] `OpenOrders` struct with all required fields:
  - `bump`: u8
  - `authority`: Pubkey (owner)
  - `market`: Pubkey
  - `base_free` / `base_locked`: u64
  - `quote_free` / `quote_locked`: u64
  - `orders`: array of order slots (order_id, side, client_order_id)
  - `delegate`: Option<Pubkey>
- [ ] PDA derivation: `["open_orders", market, authority]`
- [ ] Order slot management (find free slot, clear slot)
- [ ] Balance invariants: free + locked = total deposited
- [ ] Delegate support for third-party trading
- [ ] Unit tests for slot management and balance tracking

## Technical Approach

### 1. Project Structure

```
program/src/
├── state/
│   ├── mod.rs              # Update exports
│   ├── market.rs           # Existing
│   ├── orderbook.rs        # Existing
│   ├── event_queue.rs      # Existing
│   └── open_orders.rs      # New - OpenOrders structures
```

### 2. Order Slot Structure

Each order slot stores minimal info for O(1) lookup:

```rust
/// Order slot in OpenOrders account
#[derive(Default)]
pub struct OrderSlot {
    /// Order ID (0 = empty slot)
    pub order_id: u128,
    /// Client order ID
    pub client_order_id: u64,
    /// Side (bid/ask)
    pub side: Side,
}
```

### 3. OpenOrders Structure

```rust
#[account]
pub struct OpenOrders {
    /// Bump seed for PDA derivation.
    pub bump: u8,
    /// Market this account is for.
    pub market: Pubkey,
    /// Owner wallet (authority).
    pub owner: Pubkey,
    /// Delegate that can place/cancel orders (optional).
    pub delegate: Pubkey,  // Pubkey::default() = no delegate
    /// Base tokens locked in open orders.
    pub base_locked: u64,
    /// Quote tokens locked in open orders.
    pub quote_locked: u64,
    /// Base tokens available for withdrawal.
    pub base_free: u64,
    /// Quote tokens available for withdrawal.
    pub quote_free: u64,
    /// Accumulated referrer rebates.
    pub referrer_rebates: u64,
    /// Number of active orders.
    pub num_orders: u8,
    /// Reserved for future use.
    pub reserved: [u8; 64],
    /// Order slots (fixed size array).
    pub orders: [OrderSlot; MAX_ORDERS],
}
```

### 4. Constants

```rust
/// Maximum number of orders per OpenOrders account
pub const MAX_ORDERS: usize = 128;

/// Seed prefix for OpenOrders PDA derivation
pub const OPEN_ORDERS_SEED: &[u8] = b"open_orders";
```

### 5. Key Methods

```rust
impl OpenOrders {
    /// Find a free order slot
    pub fn find_free_slot(&self) -> Option<u8>;
    
    /// Add an order to a slot
    pub fn add_order(&mut self, slot: u8, order_id: u128, side: Side, client_order_id: u64);
    
    /// Remove an order from a slot
    pub fn remove_order(&mut self, slot: u8);
    
    /// Check if caller is authorized (owner or delegate)
    pub fn is_authorized(&self, signer: &Pubkey) -> bool;
    
    /// Lock base tokens for an ask order
    pub fn lock_base(&mut self, amount: u64) -> Option<()>;
    
    /// Lock quote tokens for a bid order
    pub fn lock_quote(&mut self, amount: u64) -> Option<()>;
    
    /// Release locked tokens back to free
    pub fn release_base(&mut self, amount: u64) -> Option<()>;
    pub fn release_quote(&mut self, amount: u64) -> Option<()>;
}
```

### 6. Space Calculation

```
Fixed fields: 1 + 32 + 32 + 32 + 8*5 + 1 + 64 = 202 bytes
Per order slot: 16 + 8 + 1 = 25 bytes
For 128 orders: 202 + 128 * 25 = 3,402 bytes
With discriminator: 8 + 3,402 = 3,410 bytes
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `program/src/state/open_orders.rs` | Create | OpenOrders structures and operations |
| `program/src/state/mod.rs` | Modify | Export open_orders types |
| `program/src/lib.rs` | Modify | Re-export open_orders types |

## Tests Needed

- `test_open_orders_pda_derivation` - Verify PDA seeds
- `test_find_free_slot_empty` - Find slot in empty account
- `test_find_free_slot_partial` - Find slot with some orders
- `test_find_free_slot_full` - No slot when full
- `test_add_remove_order` - Add and remove order from slot
- `test_is_authorized_owner` - Owner is authorized
- `test_is_authorized_delegate` - Delegate is authorized
- `test_is_authorized_other` - Random signer not authorized
- `test_lock_release_base` - Lock and release base tokens
- `test_lock_release_quote` - Lock and release quote tokens
- `test_lock_insufficient` - Lock fails with insufficient balance

## Notes

- Fixed 128 order slots to avoid dynamic allocation
- Slot index stored in LeafNode for O(1) lookup on cancel
- Delegate can place/cancel but not withdraw
- Pubkey::default() means no delegate set
- Balance operations use checked arithmetic
