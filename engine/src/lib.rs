//! # matchbook-engine
//!
//! A deterministic, allocation-free central limit order book that runs over a
//! plain byte buffer. The same code powers the on-chain Matchbook program and
//! any off-chain tool (simulators, backtests, SDK quoting, fuzzers), so what
//! you simulate locally is exactly what executes on Solana.
//!
//! ## Layout
//!
//! A market is a single buffer: a fixed [`MarketHeader`] followed by an array
//! of 88-byte [`Node`]s. Nodes hold three red-black trees (bids, asks, traders)
//! and a free list. The buffer can grow at any time; new nodes at the tail are
//! picked up automatically.
//!
//! ## Example
//!
//! ```
//! use matchbook_engine::*;
//!
//! let mut data = vec![0u8; market_size(64)];
//! Market::init(&mut data, &MarketParams {
//!     base_lot_size: 1_000,
//!     quote_lot_size: 1,
//!     min_base_lots: 1,
//!     taker_fee_bps: 10,
//!     maker_rebate_bps: 2,
//!     ..MarketParams::default()
//! }).unwrap();
//!
//! let mut market = Market::load_mut(&mut data).unwrap();
//! let alice = market.claim_seat(&[1; 32]).unwrap();
//! let bob = market.claim_seat(&[2; 32]).unwrap();
//! market.deposit(alice, 10_000, 0).unwrap();
//! market.deposit(bob, 0, 1_000_000).unwrap();
//!
//! // Alice rests an ask: 5 lots at price 100 (quote lots per base lot).
//! market.place_order(alice, &OrderParams::limit(Side::Ask, 100, 5), 0, &mut NoopSink).unwrap();
//! // Bob crosses it and is filled atomically: no crank, no event queue.
//! let out = market.place_order(bob, &OrderParams::limit(Side::Bid, 101, 3), 0, &mut NoopSink).unwrap();
//! assert_eq!(out.filled_base_lots, 3);
//! assert_eq!(market.trader(alice).unwrap().quote_free, 300 + 0); // 3 lots * 100, rebate rounds to 0
//! ```
#![no_std]

#[cfg(test)]
extern crate std;

mod error;
mod events;
mod header;
mod market;
mod matching;
pub mod math;
mod node;
mod rbtree;
mod stops;
mod types;

pub use error::EngineError;
pub use events::{EventSink, FillEvent, NoopSink, OrderOutEvent, OutReason, PlacedEvent};
pub use header::{MarketHeader, MarketStatus, HEADER_SIZE, MARKET_FLAG_PERMISSIONED, MARKET_MAGIC};
pub use market::{market_size, Market, MarketParams, MarketTotals};
pub use node::{
    order_id, split_order_id, DelegateLeaf, Node, NodeTag, OrderLeaf, StopLeaf, TraderLeaf,
    MAX_ORDER_SEQ, NIL, NODE_SIZE, ORDER_INDEX_BITS,
};
pub use types::{
    permission, Actor, BookLevel, OrderParams, OrderType, PlaceOutcome, RestingOrder,
    SelfTradeBehavior, Side, StopParams, SwapOutcome, SwapParams, TriggerDirection,
    AUTO_TRIGGER_LIMIT, MAX_STOP_MATCH_LIMIT, REVERSE_SPREAD_SCALE,
};

/// Result type used across the engine.
pub type Result<T> = core::result::Result<T, EngineError>;

/// Logs the remaining compute units when built with the `cu-trace` feature
/// for SBF; compiles to nothing otherwise. Used to profile the engine.
#[doc(hidden)]
#[macro_export]
macro_rules! cu {
    () => {
        #[cfg(all(feature = "cu-trace", target_os = "solana"))]
        $crate::trace::log_compute_units();
    };
}

#[doc(hidden)]
#[cfg(all(feature = "cu-trace", target_os = "solana"))]
pub mod trace {
    extern "C" {
        fn sol_log_compute_units_();
    }

    /// Logs the remaining compute units.
    #[inline(always)]
    #[allow(unsafe_code)]
    pub fn log_compute_units() {
        // SAFETY: the syscall takes no arguments.
        unsafe { sol_log_compute_units_() }
    }
}
