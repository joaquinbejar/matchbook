//! The market maker's quoting and risk logic, free of I/O so it can be
//! tested on its own.
//!
//! Each cycle the bot turns a reference price and its inventory into a
//! ladder of post-only quotes:
//!
//! - The first bid and ask sit `spread_bps` from the reference price, and
//!   each further level `step_bps` beyond the previous one.
//! - Inventory skews the ladder: a seat holding more base than its target
//!   quotes lower (sells more, buys less), and the reverse when short.
//! - Past `max_position` from the target the bot stops quoting the side
//!   that would grow the position.
//! - No side ever asks for more than the seat holds, nor more than
//!   `max_notional` quote tokens.

use matchbook_sdk::engine::{OrderType, SelfTradeBehavior, Side};
use matchbook_sdk::OrderArgs;

use crate::units::Units;

/// Quoting parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuoteParams {
    /// Levels per side.
    pub levels: u32,
    /// Distance of the first level from the reference price (bps).
    pub spread_bps: f64,
    /// Distance between levels (bps).
    pub step_bps: f64,
    /// Size of the first level in base tokens.
    pub size: f64,
    /// Each level is this many times the size of the previous one.
    pub size_growth: f64,
    /// Base tokens the bot aims to hold.
    pub target_base: f64,
    /// Largest distance from `target_base` (base tokens) it accepts.
    pub max_position: f64,
    /// How far (bps) the ladder moves at a full `max_position` imbalance.
    pub skew_bps: f64,
    /// Most quote tokens committed per side (0 = no limit).
    pub max_notional: f64,
    /// Requote when the reference price moves this far (bps).
    pub requote_bps: f64,
}

impl Default for QuoteParams {
    fn default() -> Self {
        Self {
            levels: 3,
            spread_bps: 10.0,
            step_bps: 10.0,
            size: 1.0,
            size_growth: 1.0,
            target_base: 0.0,
            max_position: f64::INFINITY,
            skew_bps: 0.0,
            max_notional: 0.0,
            requote_bps: 5.0,
        }
    }
}

/// What the seat holds, in tokens: in total for risk, and what the next
/// ladder can use.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Inventory {
    /// Base tokens, free plus locked in asks and stops.
    pub base: f64,
    /// Quote tokens, free plus locked in bids and stops.
    pub quote: f64,
    /// Base tokens the ladder can sell: free plus what the requote's own
    /// cancels release. Funds locked in other orders or stops stay out.
    pub base_available: f64,
    /// Quote tokens the ladder can spend, counted the same way.
    pub quote_available: f64,
}

impl Inventory {
    /// An inventory with everything available, as when nothing is locked.
    #[must_use]
    pub fn free(base: f64, quote: f64) -> Self {
        Self {
            base,
            quote,
            base_available: base,
            quote_available: quote,
        }
    }

    /// Value in quote tokens at `price`.
    #[must_use]
    pub fn equity(&self, price: f64) -> f64 {
        self.base * price + self.quote
    }
}

/// One quote: book price and base lots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quote {
    /// Side.
    pub side: Side,
    /// Price in quote lots per base lot.
    pub price: u64,
    /// Size in base lots.
    pub base_lots: u64,
}

/// The ladder for one cycle.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ladder {
    /// Bids, best first.
    pub bids: Vec<Quote>,
    /// Asks, best first.
    pub asks: Vec<Quote>,
    /// The skewed center price (quote tokens per base token).
    pub center: f64,
}

impl Ladder {
    /// Every quote as an order: post-only-slide, so a quote that would
    /// cross reprices behind the book instead of failing the batch.
    #[must_use]
    pub fn orders(&self, client_order_id: u64) -> Vec<OrderArgs> {
        self.bids
            .iter()
            .chain(&self.asks)
            .map(|q| OrderArgs {
                order_type: OrderType::PostOnlySlide as u8,
                self_trade: SelfTradeBehavior::CancelProvide as u8,
                client_order_id,
                ..OrderArgs::limit(q.side as u8, q.price, q.base_lots)
            })
            .collect()
    }

    /// Number of quotes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bids.len() + self.asks.len()
    }

    /// True with no quotes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Builds the ladder around `reference` (quote tokens per base token).
#[must_use]
pub fn build_ladder(reference: f64, inv: &Inventory, p: &QuoteParams, u: &Units) -> Ladder {
    let mut out = Ladder::default();
    if !(reference.is_finite() && reference > 0.0) || p.levels == 0 {
        return out;
    }
    let deviation = inv.base - p.target_base;
    let ratio = if p.max_position.is_finite() && p.max_position > 0.0 {
        (deviation / p.max_position).clamp(-1.0, 1.0)
    } else {
        0.0
    };
    let center = reference * (1.0 - ratio * p.skew_bps / 10_000.0);
    out.center = center;
    let quote_bids = deviation < p.max_position;
    let quote_asks = -deviation < p.max_position;

    let mut base_left = inv.base_available.max(0.0);
    let mut quote_left = inv.quote_available.max(0.0);
    if p.max_notional > 0.0 {
        quote_left = quote_left.min(p.max_notional);
    }
    let mut ask_notional_left = if p.max_notional > 0.0 {
        p.max_notional
    } else {
        f64::INFINITY
    };
    // Room left before a side would push the position past its limit.
    let mut buy_room = (p.max_position - deviation).max(0.0);
    let mut sell_room = (p.max_position + deviation).max(0.0);

    let mut size = p.size;
    for level in 0..p.levels {
        let off = (p.spread_bps + f64::from(level) * p.step_bps) / 10_000.0;
        let bid_px = u.price_f64_to_lots(center * (1.0 - off)).max(1);
        // Asks round up so a tight spread never collapses onto the bid.
        let ask_px = u.price_f64_to_lots(center * (1.0 + off)).max(bid_px + 1);
        if quote_bids {
            let px = u.price_to_f64(bid_px);
            let want = size
                .min(buy_room)
                .min(quote_left / px.max(f64::MIN_POSITIVE));
            let lots = lots_of(want, u);
            if lots > 0 && out.bids.last().is_none_or(|q: &Quote| q.price > bid_px) {
                let tokens = u.lots_to_f64(lots);
                quote_left -= tokens * px;
                buy_room -= tokens;
                out.bids.push(Quote {
                    side: Side::Bid,
                    price: bid_px,
                    base_lots: lots,
                });
            }
        }
        if quote_asks {
            let px = u.price_to_f64(ask_px);
            let want = size
                .min(sell_room)
                .min(base_left)
                .min(ask_notional_left / px.max(f64::MIN_POSITIVE));
            let lots = lots_of(want, u);
            if lots > 0 && out.asks.last().is_none_or(|q: &Quote| q.price < ask_px) {
                let tokens = u.lots_to_f64(lots);
                base_left -= tokens;
                sell_room -= tokens;
                ask_notional_left -= tokens * px;
                out.asks.push(Quote {
                    side: Side::Ask,
                    price: ask_px,
                    base_lots: lots,
                });
            }
        }
        size *= p.size_growth;
    }
    out
}

fn lots_of(tokens: f64, u: &Units) -> u64 {
    if !(tokens.is_finite() && tokens > 0.0) {
        return 0;
    }
    // A hair under to absorb float error, then floor.
    let lots = tokens * 10f64.powi(i32::from(u.base_decimals)) / u.base_lot_size as f64;
    (lots + 1e-9).floor() as u64
}

/// True when the price moved enough since the last quote to requote.
#[must_use]
pub fn moved(last: f64, now: f64, requote_bps: f64) -> bool {
    last <= 0.0 || ((now - last) / last).abs() * 10_000.0 >= requote_bps
}

/// Risk limits checked every cycle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiskLimits {
    /// Stop when equity falls this many quote tokens below its start (0 = off).
    pub max_loss: f64,
    /// Pull quotes when the reference price is older than this (seconds).
    pub max_price_age_secs: u64,
    /// Pull quotes when the reference moves this much in one cycle (bps, 0 = off).
    pub max_jump_bps: f64,
}

/// What the risk check decided.
#[derive(Debug, Clone, PartialEq)]
pub enum RiskVerdict {
    /// Keep quoting.
    Quote,
    /// Cancel everything this cycle and try again next one.
    Pause(String),
    /// Cancel everything and exit.
    Halt(String),
}

/// Checks the limits for one cycle.
#[must_use]
pub fn check_risk(
    limits: &RiskLimits,
    start_equity: f64,
    equity: f64,
    price_age_secs: u64,
    last_reference: f64,
    reference: f64,
) -> RiskVerdict {
    if limits.max_loss > 0.0 && start_equity - equity >= limits.max_loss {
        return RiskVerdict::Halt(format!(
            "loss {:.6} reached the limit {:.6}",
            start_equity - equity,
            limits.max_loss
        ));
    }
    if price_age_secs > limits.max_price_age_secs {
        return RiskVerdict::Pause(format!("reference price is {price_age_secs}s old"));
    }
    if limits.max_jump_bps > 0.0
        && last_reference > 0.0
        && ((reference - last_reference) / last_reference).abs() * 10_000.0 > limits.max_jump_bps
    {
        return RiskVerdict::Pause(format!(
            "reference jumped from {last_reference} to {reference}"
        ));
    }
    RiskVerdict::Quote
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units() -> Units {
        Units {
            base_lot_size: 1_000_000,
            quote_lot_size: 1,
            base_decimals: 9,
            quote_decimals: 6,
        }
    }

    fn rich() -> Inventory {
        Inventory::free(100.0, 100_000.0)
    }

    #[test]
    fn symmetric_ladder() {
        let p = QuoteParams {
            target_base: 100.0,
            ..QuoteParams::default()
        };
        let l = build_ladder(150.0, &rich(), &p, &units());
        let bids: Vec<u64> = l.bids.iter().map(|q| q.price).collect();
        let asks: Vec<u64> = l.asks.iter().map(|q| q.price).collect();
        assert_eq!(bids, [149_850, 149_700, 149_550]);
        assert_eq!(asks, [150_150, 150_300, 150_450]);
        assert!(l.bids.iter().chain(&l.asks).all(|q| q.base_lots == 1_000));
        assert_eq!(l.orders(7).len(), 6);
    }

    #[test]
    fn long_inventory_skews_down_and_stops_buying() {
        let p = QuoteParams {
            target_base: 50.0,
            max_position: 40.0,
            skew_bps: 20.0,
            ..QuoteParams::default()
        };
        let l = build_ladder(150.0, &rich(), &p, &units());
        assert!(l.bids.is_empty(), "past max position: no bids");
        assert!(l.center < 150.0);
        assert_eq!(l.asks.len(), 3);
    }

    #[test]
    fn balances_and_notional_cap_the_size() {
        let inv = Inventory::free(1.5, 200.0);
        let p = QuoteParams {
            target_base: 1.5,
            ..QuoteParams::default()
        };
        let l = build_ladder(150.0, &inv, &p, &units());
        // 200 quote buys one full level and a partial one; 1.5 base sells
        // one level and a half.
        assert_eq!(l.bids.len(), 2);
        assert_eq!(l.asks.iter().map(|q| q.base_lots).sum::<u64>(), 1_500);
        let capped = build_ladder(
            150.0,
            &rich(),
            &QuoteParams {
                target_base: 100.0,
                max_notional: 300.0,
                ..QuoteParams::default()
            },
            &units(),
        );
        let notional: f64 = capped
            .asks
            .iter()
            .map(|q| units().lots_to_f64(q.base_lots) * units().price_to_f64(q.price))
            .sum();
        assert!(notional <= 300.0 + 1e-9);
    }

    #[test]
    fn risk_checks() {
        let l = RiskLimits {
            max_loss: 10.0,
            max_price_age_secs: 30,
            max_jump_bps: 100.0,
        };
        assert_eq!(
            check_risk(&l, 1000.0, 995.0, 1, 150.0, 150.1),
            RiskVerdict::Quote
        );
        assert!(matches!(
            check_risk(&l, 1000.0, 989.0, 1, 150.0, 150.0),
            RiskVerdict::Halt(_)
        ));
        assert!(matches!(
            check_risk(&l, 1000.0, 1000.0, 31, 150.0, 150.0),
            RiskVerdict::Pause(_)
        ));
        assert!(matches!(
            check_risk(&l, 1000.0, 1000.0, 1, 150.0, 152.0),
            RiskVerdict::Pause(_)
        ));
        assert!(moved(150.0, 150.1, 5.0));
        assert!(!moved(150.0, 150.05, 5.0));
    }

    #[test]
    fn locked_funds_outside_the_requote_are_not_quoted() {
        // 10 base held, 9 of them in someone else's ask: 1 left to sell.
        let inv = Inventory {
            base_available: 1.0,
            ..Inventory::free(10.0, 0.0)
        };
        let p = QuoteParams {
            target_base: 10.0,
            size: 5.0,
            ..QuoteParams::default()
        };
        let l = build_ladder(150.0, &inv, &p, &units());
        assert_eq!(l.asks.iter().map(|q| q.base_lots).sum::<u64>(), 1_000);
    }
}
