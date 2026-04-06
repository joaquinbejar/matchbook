# M1 Issue #3: OrderBook Accounts (B+ Tree Structure)

## Overview

Implement the OrderBook account structures for Bids and Asks using a B+ tree data structure optimized for Solana's compute constraints. This is a critical data structure that stores all orders in the market.

## Acceptance Criteria

- [ ] `OrderBookSide` struct with B+ tree implementation:
  - `bump`: u8
  - `market`: Pubkey
  - `is_bids`: bool
  - `leaf_count`: u32
  - `free_leaf_head`: u32
  - `root`: u32
  - `nodes`: array of `AnyNode`
- [ ] `InnerNode` struct (internal B+ tree nodes)
- [ ] `LeafNode` struct for orders:
  - `key`: u128 (price-time priority)
  - `owner`: Pubkey
  - `quantity`: u64
  - `client_order_id`: u64
  - `owner_slot`: u8
  - `time_in_force`: TimeInForce
- [ ] PDA derivation: `["bids", market]` and `["asks", market]`
- [ ] Key encoding: `(price << 64) | seq_num` for bids (inverted for asks)
- [ ] Insert, remove, and iterate operations
- [ ] Unit tests for B+ tree operations

## Technical Approach

### 1. Project Structure

```
program/src/
├── state/
│   ├── mod.rs              # Update exports
│   ├── market.rs           # Existing
│   └── orderbook.rs        # New - OrderBook structures
```

### 2. Node Types

Based on `.internalDoc/03-onchain-design.md`, we use a critbit tree (a form of radix tree) rather than a traditional B+ tree. This is more efficient for Solana's compute model.

```rust
/// Tag for node type discrimination
#[repr(u8)]
pub enum NodeTag {
    Uninitialized = 0,
    InnerNode = 1,
    LeafNode = 2,
    FreeNode = 3,
    LastFreeNode = 4,
}

/// Union type for nodes in the order book
#[zero_copy]
#[repr(C)]
pub union AnyNode {
    pub inner: InnerNode,
    pub leaf: LeafNode,
    pub free: FreeNode,
}

/// Inner node for tree navigation
#[zero_copy]
#[repr(C)]
pub struct InnerNode {
    pub tag: u8,
    pub padding: [u8; 3],
    pub prefix_len: u32,
    pub key: u128,
    pub children: [u32; 2],
    pub padding2: [u8; 56],
}

/// Leaf node containing order data
#[zero_copy]
#[repr(C)]
pub struct LeafNode {
    pub tag: u8,
    pub owner_slot: u8,
    pub time_in_force: u8,
    pub padding: [u8; 1],
    pub key: u128,
    pub owner: Pubkey,
    pub quantity: u64,
    pub client_order_id: u64,
    pub padding2: [u8; 16],
}

/// Free node in the free list
#[zero_copy]
#[repr(C)]
pub struct FreeNode {
    pub tag: u8,
    pub padding: [u8; 3],
    pub next: u32,
    pub padding2: [u8; 80],
}
```

### 3. OrderBookSide Header

```rust
#[account(zero_copy)]
#[repr(C)]
pub struct OrderBookSide {
    pub bump: u8,
    pub padding: [u8; 7],
    pub market: Pubkey,
    pub is_bids: u8,
    pub padding2: [u8; 7],
    pub leaf_count: u32,
    pub free_list_head: u32,
    pub root: u32,
    pub padding3: [u8; 4],
    pub reserved: [u8; 64],
    // nodes follow in account data
}
```

### 4. Key Encoding

For price-time priority ordering:
- **Bids**: Higher prices first, then earlier time. Key = `(!price << 64) | seq_num`
- **Asks**: Lower prices first, then earlier time. Key = `(price << 64) | seq_num`

```rust
impl OrderId {
    pub fn new_bid(price: u64, seq_num: u64) -> Self {
        Self(((!price as u128) << 64) | (seq_num as u128))
    }

    pub fn new_ask(price: u64, seq_num: u64) -> Self {
        Self(((price as u128) << 64) | (seq_num as u128))
    }

    pub fn price(&self, is_bid: bool) -> u64 {
        let raw = (self.0 >> 64) as u64;
        if is_bid { !raw } else { raw }
    }

    pub fn seq_num(&self) -> u64 {
        self.0 as u64
    }
}
```

### 5. TimeInForce Enum

```rust
#[repr(u8)]
pub enum TimeInForce {
    GoodTilCancelled = 0,
    ImmediateOrCancel = 1,
    FillOrKill = 2,
    PostOnly = 3,
}
```

### 6. Tree Operations

The critbit tree operations:
- `insert(key, leaf)` - O(log n) insert
- `remove(key)` - O(log n) remove
- `find(key)` - O(log n) lookup
- `min()` / `max()` - O(log n) best price
- `iter()` - In-order traversal

### 7. Space Calculation

```
Header: 1 + 7 + 32 + 1 + 7 + 4 + 4 + 4 + 4 + 64 = 128 bytes
Per node: 88 bytes (aligned)

For 10,000 orders:
  ~15,000 nodes (leaves + inner nodes)
  15,000 * 88 + 128 = 1,320,128 bytes ≈ 1.3 MB
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `program/src/state/orderbook.rs` | Create | OrderBookSide, nodes, tree operations |
| `program/src/state/mod.rs` | Modify | Export orderbook types |
| `program/src/lib.rs` | Modify | Re-export orderbook types |

## Tests Needed

- `test_order_id_encoding` - Verify key encoding for bids/asks
- `test_order_id_price_extraction` - Verify price extraction
- `test_orderbook_pda_derivation` - Verify PDA seeds
- `test_node_size` - Verify node sizes are correct (88 bytes)
- `test_insert_single` - Insert one order
- `test_insert_multiple_ascending` - Insert orders in ascending price
- `test_insert_multiple_descending` - Insert orders in descending price
- `test_remove_leaf` - Remove an order
- `test_find_min_max` - Find best bid/ask
- `test_iterate_orders` - Iterate in price-time priority

## Notes

- Using critbit tree (radix tree) as it's more efficient for Solana than B+ tree
- Zero-copy deserialization for performance
- Free list for node reuse without reallocation
- Node size is 88 bytes to fit nicely in cache lines
- Max tree depth ~64 for u128 keys (but typically much less)
