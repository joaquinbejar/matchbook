//! The MatchBook engine as a WebAssembly module, for the browser playground.
//!
//! It holds one market in memory and exposes a flat `extern "C"` API over
//! JavaScript numbers (`f64`), so the page needs no bindings generator.
//! Traders are small integers; results that are lists go to an output
//! buffer the page reads at [`mb_out_ptr`].
//!
//! Every call runs the same engine code the on-chain program runs.

use std::cell::RefCell;

use matchbook_engine::{
    market_size, EventSink, FillEvent, Market, MarketParams, OrderOutEvent, OrderParams, OrderType,
    PlacedEvent, SelfTradeBehavior, Side, StopParams, SwapParams, TriggerDirection, NODE_SIZE,
};

/// Nodes added each time the market runs low.
const GROW_NODES: usize = 256;

struct State {
    words: Vec<u64>,
    len: usize,
    now: u64,
    out: Vec<f64>,
    events: Vec<f64>,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn owner(id: u32) -> [u8; 32] {
    let mut o = [0u8; 32];
    o[..4].copy_from_slice(&id.to_be_bytes());
    o[31] = 1;
    o
}

fn id_of(owner: &[u8; 32]) -> f64 {
    f64::from(u32::from_be_bytes([owner[0], owner[1], owner[2], owner[3]]))
}

/// Records events as rows of seven numbers: kind, then six fields.
struct Rec<'a>(&'a mut Vec<f64>);

impl EventSink for Rec<'_> {
    fn on_fill(&mut self, e: &FillEvent<'_>) {
        let taker = e.taker.map_or(-1.0, id_of);
        self.0.extend_from_slice(&[
            1.0,
            id_of(e.maker),
            e.maker_order_id as f64,
            e.price as f64,
            e.base_lots as f64,
            e.maker_remaining as f64,
            taker,
        ]);
    }
    fn on_out(&mut self, e: &OrderOutEvent) {
        self.0.extend_from_slice(&[
            2.0,
            id_of(&e.owner),
            e.order_id as f64,
            e.price as f64,
            e.base_lots as f64,
            f64::from(e.reason as u8),
            f64::from(e.side as u8),
        ]);
    }
    fn on_placed(&mut self, e: &PlacedEvent) {
        self.row(4.0, e);
    }
    fn on_reversed(&mut self, e: &PlacedEvent) {
        self.row(3.0, e);
    }
}

impl Rec<'_> {
    fn row(&mut self, kind: f64, e: &PlacedEvent) {
        self.0.extend_from_slice(&[
            kind,
            id_of(&e.owner),
            e.order_id as f64,
            e.price as f64,
            e.base_lots as f64,
            f64::from(e.side as u8),
            e.expires_at as f64,
        ]);
    }
}

fn words_as_bytes(words: &mut [u64], len: usize) -> &mut [u8] {
    let bytes = as_bytes_mut(words);
    let (head, _) = bytes.split_at_mut(len.min(bytes.len()));
    head
}

fn as_bytes_mut(words: &mut [u64]) -> &mut [u8] {
    // u64 words keep the node pool aligned for the engine.
    bytemuck::cast_slice_mut(words)
}

/// Runs `f` on the market with a fresh event list. Returns -1 when no
/// market exists, or what `f` returns.
fn with_market(f: impl FnOnce(&mut Market<'_>, u64, &mut Rec<'_>, &mut Vec<f64>) -> f64) -> f64 {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let Some(st) = s.as_mut() else {
            return -1.0;
        };
        st.events.clear();
        // Grow ahead of need, as the program does before each instruction.
        let needs = {
            let bytes = words_as_bytes(&mut st.words, st.len);
            Market::load_mut(bytes).map_or(0, |m| m.bytes_to_grow(16))
        };
        if needs > 0 {
            st.len += GROW_NODES * NODE_SIZE;
            st.words.resize(st.len.div_ceil(8), 0);
        }
        let now = st.now;
        let State {
            words,
            len,
            out,
            events,
            ..
        } = st;
        let bytes = words_as_bytes(words, *len);
        let Ok(mut m) = Market::load_mut(bytes) else {
            return -1.0;
        };
        f(&mut m, now, &mut Rec(events), out)
    })
}

fn err(e: matchbook_engine::EngineError) -> f64 {
    -(f64::from(e as u32) + 2.0)
}

fn seat(m: &Market<'_>, trader: u32) -> Option<u32> {
    m.find_trader(&owner(trader)).ok().flatten()
}

/// Creates the market. Returns 0.
#[no_mangle]
pub extern "C" fn mb_init(
    base_lot_size: f64,
    quote_lot_size: f64,
    taker_fee_bps: f64,
    maker_rebate_bps: f64,
    referral_share_bps: f64,
) -> f64 {
    let len = market_size(GROW_NODES);
    let mut words = vec![0u64; len.div_ceil(8)];
    let r = Market::init(
        words_as_bytes(&mut words, len),
        &MarketParams {
            base_lot_size: base_lot_size as u64,
            quote_lot_size: quote_lot_size as u64,
            min_base_lots: 1,
            taker_fee_bps: taker_fee_bps as u32,
            maker_rebate_bps: maker_rebate_bps as u32,
            referral_share_bps: referral_share_bps as u32,
            ..MarketParams::default()
        },
    );
    if let Err(e) = r {
        return err(e);
    }
    STATE.with(|s| {
        *s.borrow_mut() = Some(State {
            words,
            len,
            now: 1_700_000_000,
            out: Vec::new(),
            events: Vec::new(),
        });
    });
    0.0
}

/// Sets the clock (unix seconds).
#[no_mangle]
pub extern "C" fn mb_set_time(now: f64) {
    STATE.with(|s| {
        if let Some(st) = s.borrow_mut().as_mut() {
            st.now = now as u64;
        }
    });
}

/// Claims a seat for `trader` if it has none and credits its balances.
/// Returns the seat index.
#[no_mangle]
pub extern "C" fn mb_deposit(trader: f64, base_atoms: f64, quote_atoms: f64) -> f64 {
    with_market(|m, _, _, _| {
        let t = trader as u32;
        let i = match seat(m, t) {
            Some(i) => i,
            None => match m.claim_seat(&owner(t)) {
                Ok(i) => i,
                Err(e) => return err(e),
            },
        };
        match m.deposit(i, base_atoms as u64, quote_atoms as u64) {
            Ok(()) => f64::from(i),
            Err(e) => err(e),
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn order(
    side: f64,
    price: f64,
    lots: f64,
    order_type: f64,
    reverse_ppm: f64,
    expires_at: f64,
    quote_budget: f64,
) -> Option<OrderParams> {
    Some(OrderParams {
        side: Side::from_u8(side as u8)?,
        price: price as u64,
        base_lots: lots as u64,
        order_type: OrderType::from_u8(order_type as u8)?,
        self_trade: SelfTradeBehavior::CancelProvide,
        client_order_id: 0,
        expires_at: expires_at as u64,
        quote_budget: (quote_budget > 0.0).then_some(quote_budget as u64),
        match_limit: OrderParams::DEFAULT_MATCH_LIMIT,
        reverse_spread: reverse_ppm as u32,
    })
}

/// Places an order. Returns the resting order id (0 if nothing rested) or a
/// negative error. The outcome goes to the output buffer: filled lots,
/// quote atoms, fee, rested lots, rested price.
#[no_mangle]
pub extern "C" fn mb_place(
    trader: f64,
    side: f64,
    price: f64,
    lots: f64,
    order_type: f64,
    reverse_ppm: f64,
    expires_at: f64,
    quote_budget: f64,
) -> f64 {
    with_market(|m, now, rec, out| {
        let Some(i) = seat(m, trader as u32) else {
            return -1.0;
        };
        let Some(p) = order(
            side,
            price,
            lots,
            order_type,
            reverse_ppm,
            expires_at,
            quote_budget,
        ) else {
            return -1.0;
        };
        match m.place_order(i, &p, now, rec) {
            Ok(o) => {
                out.clear();
                out.extend_from_slice(&[
                    o.filled_base_lots as f64,
                    o.filled_quote_atoms as f64,
                    o.fee_atoms as f64,
                    o.rested_base_lots as f64,
                    o.rested_price as f64,
                ]);
                o.order_id.unwrap_or(0) as f64
            }
            Err(e) => err(e),
        }
    })
}

/// Places a stop (direction 0 above, 1 below). Returns its id or a
/// negative error.
#[no_mangle]
pub extern "C" fn mb_stop(
    trader: f64,
    direction: f64,
    trigger_price: f64,
    side: f64,
    price: f64,
    lots: f64,
    order_type: f64,
    oco_with: f64,
) -> f64 {
    with_market(|m, now, _, _| {
        let Some(i) = seat(m, trader as u32) else {
            return -1.0;
        };
        let direction = if direction as u8 == 0 {
            TriggerDirection::Above
        } else {
            TriggerDirection::Below
        };
        let Some(order) = order(side, price, lots, order_type, 0.0, 0.0, 0.0) else {
            return -1.0;
        };
        let p = StopParams {
            trigger_price: trigger_price as u64,
            direction,
            order,
            oco_with: oco_with as u64,
        };
        match m.place_stop(i, &p, now) {
            Ok(id) => id as f64,
            Err(e) => err(e),
        }
    })
}

/// Cancels an order or stop. Returns 0 or a negative error.
#[no_mangle]
pub extern "C" fn mb_cancel(trader: f64, order_id: f64) -> f64 {
    with_market(|m, _, rec, _| {
        let Some(i) = seat(m, trader as u32) else {
            return -1.0;
        };
        match m.cancel_order(i, order_id as u64, rec) {
            Ok(()) => 0.0,
            Err(e) => err(e),
        }
    })
}

/// A seatless swap. Returns base lots traded or a negative error; the
/// output buffer holds base atoms, quote atoms, fee, makers touched.
#[no_mangle]
pub extern "C" fn mb_swap(side: f64, lots: f64, quote_budget: f64, limit_price: f64) -> f64 {
    with_market(|m, now, rec, out| {
        let Some(side) = Side::from_u8(side as u8) else {
            return -1.0;
        };
        let p = SwapParams {
            side,
            base_lots: lots as u64,
            quote_budget: quote_budget as u64,
            limit_price: limit_price as u64,
            match_limit: 64,
        };
        match m.swap(&p, now, rec) {
            Ok(o) => {
                out.clear();
                out.extend_from_slice(&[
                    o.base_atoms as f64,
                    o.quote_atoms as f64,
                    o.fee_atoms as f64,
                    f64::from(o.makers_touched),
                ]);
                o.base_lots as f64
            }
            Err(e) => err(e),
        }
    })
}

/// Places stops whose trigger was reached. Returns how many.
#[no_mangle]
pub extern "C" fn mb_trigger_stops(limit: f64) -> f64 {
    with_market(
        |m, now, rec, _| match m.trigger_stops(now, limit as u32, rec) {
            Ok(n) => f64::from(n),
            Err(e) => err(e),
        },
    )
}

/// Removes expired orders. Returns how many.
#[no_mangle]
pub extern "C" fn mb_prune(limit: f64) -> f64 {
    with_market(
        |m, now, rec, _| match m.prune_expired(now, limit as u32, rec) {
            Ok(n) => f64::from(n),
            Err(e) => err(e),
        },
    )
}

/// Writes up to `depth` levels per side: n_bids, n_asks, then (price,
/// lots, orders) for bids best first and asks best first. Returns the
/// number of values.
#[no_mangle]
pub extern "C" fn mb_book(depth: f64) -> f64 {
    with_market(|m, now, _, out| {
        out.clear();
        out.extend_from_slice(&[0.0, 0.0]);
        for (k, side) in [Side::Bid, Side::Ask].into_iter().enumerate() {
            let mut n = 0.0;
            let _ = m.for_each_level(side, now, |l| {
                out.extend_from_slice(&[l.price as f64, l.base_lots as f64, f64::from(l.orders)]);
                n += 1.0;
                n < depth
            });
            out[k] = n;
        }
        out.len() as f64
    })
}

/// Writes a trader's orders and stops as rows of eight numbers: kind (1
/// order, 2 stop), id, side, price, lots, reverse spread (orders) or
/// trigger price (stops), expiry (orders) or direction (stops), and the
/// OCO partner flag. Returns the number of rows.
#[no_mangle]
pub extern "C" fn mb_orders(trader: f64) -> f64 {
    with_market(|m, _, _, out| {
        out.clear();
        let Some(i) = seat(m, trader as u32) else {
            return 0.0;
        };
        let mut rows = Vec::new();
        let _ = m.for_each_trader_order(i, |node, o| {
            let rev = m.order(node).map_or(0, |l| l.reverse_spread);
            rows.extend_from_slice(&[
                1.0,
                o.order_id as f64,
                f64::from(o.side as u8),
                o.price as f64,
                o.base_lots as f64,
                f64::from(rev),
                o.expires_at as f64,
                0.0,
            ]);
        });
        let _ = m.for_each_trader_stop(i, |node, s| {
            rows.extend_from_slice(&[
                2.0,
                ((s.seq() << 24) | u64::from(node)) as f64,
                if s.is_ask() { 1.0 } else { 0.0 },
                s.price as f64,
                s.base_lots as f64,
                s.trigger_price() as f64,
                if s.is_above() { 0.0 } else { 1.0 },
                if s.extra != 0 { 1.0 } else { 0.0 },
            ]);
        });
        *out = rows;
        (out.len() / 8) as f64
    })
}

/// Writes a trader's base free, quote free, base locked, quote locked.
#[no_mangle]
pub extern "C" fn mb_balances(trader: f64) -> f64 {
    with_market(|m, _, _, out| {
        out.clear();
        let Some(i) = seat(m, trader as u32) else {
            return -1.0;
        };
        let (Ok(t), Ok((bl, ql))) = (m.trader(i), m.locked(i)) else {
            return -1.0;
        };
        out.extend_from_slice(&[
            t.base_free as f64,
            t.quote_free as f64,
            bl as f64,
            ql as f64,
        ]);
        0.0
    })
}

/// Writes market stats: last price, orders, stops, traders, fees accrued,
/// base lots traded, quote atoms traded, nodes in use, bytes. Returns 1 if
/// every invariant holds.
#[no_mangle]
pub extern "C" fn mb_stats() -> f64 {
    let ok = with_market(|m, _, _, out| {
        let h = m.header();
        out.clear();
        out.extend_from_slice(&[
            h.last_price as f64,
            f64::from(h.order_count),
            f64::from(h.stop_count),
            f64::from(h.trader_count),
            h.fees_accrued as f64,
            h.total_base_lots_traded as f64,
            h.total_quote_atoms_traded as f64,
            f64::from(h.next_unused - h.free_list_len),
            0.0,
        ]);
        if m.check_invariants().is_ok() {
            1.0
        } else {
            0.0
        }
    });
    STATE.with(|s| {
        if let Some(st) = s.borrow_mut().as_mut() {
            if let Some(b) = st.out.get_mut(8) {
                *b = st.len as f64;
            }
        }
    });
    ok
}

/// Copies the events of the last call to the output buffer as rows of
/// seven numbers (see [`Rec`]). Returns the number of rows.
#[no_mangle]
pub extern "C" fn mb_events() -> f64 {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let Some(st) = s.as_mut() else {
            return 0.0;
        };
        st.out = st.events.clone();
        (st.out.len() / 7) as f64
    })
}

/// Address of the output buffer in linear memory.
#[no_mangle]
pub extern "C" fn mb_out_ptr() -> usize {
    STATE.with(|s| s.borrow().as_ref().map_or(0, |st| st.out.as_ptr() as usize))
}
