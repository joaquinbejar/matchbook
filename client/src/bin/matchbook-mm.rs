//! `matchbook-mm`: a market-making bot for MatchBook.
//!
//! It quotes a ladder of post-only orders around a reference price and
//! replaces them in one `BatchUpdate` instruction whenever the price moves
//! or a quote fills. It signs with a session key, so the key it runs with
//! can trade the seat but never withdraw from it. See `--help` and the
//! module docs of `matchbook_client::mm` for the quoting and risk rules.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{ensure, Context, Result};
use clap::Parser;
use matchbook_client::chain::{
    cluster_url, load_keypair, Chain, Fees, MarketCtx, Pubkey, MAX_TX_SIZE,
};
use matchbook_client::mm::{
    build_ladder, check_risk, moved, Inventory, Ladder, QuoteParams, RiskLimits, RiskVerdict,
};
use matchbook_client::price::PriceSource;
use matchbook_client::ConnectArgs;
use matchbook_sdk::engine::{RestingOrder, Side};
use matchbook_sdk::ix::{self, CancelAll};
use matchbook_sdk::OrderArgs;
use solana_keypair::Keypair;
use solana_message::Instruction;
use solana_signer::Signer;

/// Most order ids one batch cancels; more cancel every order of the seat.
const MAX_CANCELS: usize = 32;
/// Client order id the bot tags its quotes with ("MM").
const DEFAULT_CLIENT_ID: u64 = 0x4d4d;
/// Levels per side. The orders of a requote that do not fit one
/// transaction go out in more.
const MAX_LEVELS: u32 = 16;

#[derive(Parser)]
#[command(name = "matchbook-mm", version, about = "MatchBook market-making bot")]
struct Args {
    #[command(flatten)]
    connect: ConnectArgs,
    /// Market to quote.
    #[arg(long, env = "MATCHBOOK_MARKET")]
    market: Pubkey,
    /// Session key to trade with (it also pays fees). Without one the bot
    /// trades as the --keypair wallet itself.
    #[arg(long, env = "MATCHBOOK_SESSION")]
    session_keypair: Option<PathBuf>,
    /// Seat owner the session key trades for (default: the --keypair wallet).
    #[arg(long)]
    owner: Option<Pubkey>,
    /// Reference price: fixed:<p>, random:<start>[:<step bps>], book, or
    /// http:<url>#<json.path>.
    #[arg(long, default_value = "book")]
    price: String,
    /// Levels per side (at most 16).
    #[arg(long, default_value_t = 3)]
    levels: u32,
    /// Distance of the first level from the reference (bps).
    #[arg(long, default_value_t = 10.0)]
    spread_bps: f64,
    /// Distance between levels (bps).
    #[arg(long, default_value_t = 10.0)]
    step_bps: f64,
    /// Size of the first level (base tokens).
    #[arg(long)]
    size: f64,
    /// Each level is this many times the previous one.
    #[arg(long, default_value_t = 1.0)]
    size_growth: f64,
    /// Base tokens to aim for (default: what the seat holds at start).
    #[arg(long)]
    target_base: Option<f64>,
    /// Largest distance from the target (base tokens) before the bot stops
    /// quoting the side that grows the position.
    #[arg(long, default_value_t = f64::INFINITY)]
    max_position: f64,
    /// How far the ladder shifts at a full max-position imbalance (bps).
    #[arg(long, default_value_t = 0.0)]
    skew_bps: f64,
    /// Most quote tokens committed per side (0 = all the seat holds).
    #[arg(long, default_value_t = 0.0)]
    max_notional: f64,
    /// Requote when the reference moves this far (bps).
    #[arg(long, default_value_t = 5.0)]
    requote_bps: f64,
    /// Time between cycles, in milliseconds.
    #[arg(long, default_value_t = 2_000)]
    interval_ms: u64,
    /// Cancel everything and exit after losing this many quote tokens of
    /// equity, marked at the reference price (0 = off).
    #[arg(long, default_value_t = 0.0)]
    max_loss: f64,
    /// Pull quotes while the reference price is older than this (seconds).
    #[arg(long, default_value_t = 30)]
    max_price_age: u64,
    /// Pull quotes for a cycle when the reference jumps this far (bps, 0 = off).
    #[arg(long, default_value_t = 0.0)]
    max_jump_bps: f64,
    /// Client order id of the bot's quotes; other orders of the seat are left alone.
    #[arg(long, default_value_t = DEFAULT_CLIENT_ID)]
    client_id: u64,
    /// Stop after this many cycles (0 = run until Ctrl-C).
    #[arg(long, default_value_t = 0)]
    cycles: u64,
    /// Print the quotes without sending anything.
    #[arg(long)]
    dry_run: bool,
    /// Leave the quotes on the book when exiting.
    #[arg(long)]
    keep_quotes: bool,
}

fn log(msg: impl AsRef<str>) {
    let t = matchbook_client::chain::now();
    eprintln!(
        "[{:02}:{:02}:{:02}] {}",
        (t / 3600) % 24,
        (t / 60) % 60,
        t % 60,
        msg.as_ref()
    );
}

struct Bot {
    chain: Chain,
    signer: Keypair,
    owner: Pubkey,
    market: MarketCtx,
    seat: u32,
    params: QuoteParams,
    limits: RiskLimits,
    source: PriceSource,
    client_id: u64,
    dry_run: bool,
    // State between cycles.
    start_equity: Option<f64>,
    last_quoted: f64,
    last_reference: f64,
    last_price_at: Instant,
    last_lots: u64,
    last_count: usize,
    last_inventory: Option<Inventory>,
}

impl Bot {
    fn my_orders(&self) -> Result<Vec<RestingOrder>> {
        Ok(self
            .market
            .snap
            .seat_orders(self.seat)?
            .into_iter()
            .filter(|o| o.client_order_id == self.client_id)
            .collect())
    }

    /// The seat's inventory. What the ladder can use is the free balance
    /// plus what cancelling `mine` releases, never funds locked elsewhere.
    fn inventory(&self, mine: &[RestingOrder]) -> Result<Inventory> {
        let (_, t) = self
            .market
            .snap
            .seat(&self.owner)?
            .context("the seat is gone")?;
        let (lb, lq) = self.market.snap.locked(self.seat)?;
        // Cancelling more than one batch holds cancels every order of the
        // seat (see `cancel_ix`), so all of them are released.
        let all;
        let released = if mine.len() > MAX_CANCELS {
            all = self.market.snap.seat_orders(self.seat)?;
            &all[..]
        } else {
            mine
        };
        let u = &self.market.units;
        let (mut rb, mut rq) = (0u64, 0u64);
        for o in released {
            match o.side {
                Side::Ask => rb = rb.saturating_add(o.base_lots.saturating_mul(u.base_lot_size)),
                Side::Bid => {
                    rq = rq.saturating_add(
                        o.base_lots
                            .saturating_mul(o.price)
                            .saturating_mul(u.quote_lot_size),
                    );
                }
            }
        }
        Ok(Inventory {
            base: u.base_to_f64(t.base_free.saturating_add(lb)),
            quote: u.quote_to_f64(t.quote_free.saturating_add(lq)),
            base_available: u.base_to_f64(t.base_free.saturating_add(rb)),
            quote_available: u.quote_to_f64(t.quote_free.saturating_add(rq)),
        })
    }

    fn cancel_ix(&self, mine: &[RestingOrder]) -> (CancelAll, Vec<u64>) {
        if mine.len() > MAX_CANCELS {
            (CancelAll::Both, Vec::new())
        } else {
            (CancelAll::None, mine.iter().map(|o| o.order_id).collect())
        }
    }

    fn batch_ix(
        &self,
        cancel_all: CancelAll,
        cancels: &[u64],
        orders: &[OrderArgs],
    ) -> Instruction {
        ix::batch_update(
            &self.signer.pubkey(),
            &self.market.snap.key(),
            self.seat,
            cancel_all,
            cancels,
            orders,
        )
    }

    /// Cancels and places the ladder. The first transaction carries the
    /// cancels and as many orders as fit in [`MAX_TX_SIZE`]; the rest follow
    /// in as many more as needed.
    fn send_batch(&self, cancel_all: CancelAll, cancels: &[u64], ladder: &Ladder) -> Result<()> {
        let orders = ladder.orders(self.client_id);
        let payer = self.signer.pubkey();
        let fits = |c: CancelAll, cs: &[u64], os: &[OrderArgs]| {
            self.chain.tx_size(&payer, &[self.batch_ix(c, cs, os)]) <= MAX_TX_SIZE
        };
        let mut rest = &orders[..];
        let (mut all, mut cs) = (cancel_all, cancels);
        let mut sent = 0;
        loop {
            let mut n = 0;
            while rest.get(..=n).is_some_and(|os| fits(all, cs, os)) {
                n += 1;
            }
            ensure!(
                n > 0 || rest.is_empty(),
                "one order does not fit in a transaction"
            );
            let (now, later) = rest.split_at(n);
            let e = self
                .chain
                .execute_from(&self.signer, &[self.batch_ix(all, cs, now)], &[])?;
            sent += 1;
            log(format!(
                "requoted ({sent}): {} cancels, {} orders, {} CU, {}",
                cs.len(),
                now.len(),
                e.compute_units.unwrap_or(0),
                e.signature
            ));
            rest = later;
            (all, cs) = (CancelAll::None, &[]);
            if rest.is_empty() {
                return Ok(());
            }
        }
    }

    fn pull(&mut self, why: &str) -> Result<()> {
        let mine = self.my_orders()?;
        if mine.is_empty() || self.dry_run {
            return Ok(());
        }
        log(format!("pulling {} quotes: {why}", mine.len()));
        let (all, cancels) = self.cancel_ix(&mine);
        self.send_batch(all, &cancels, &Ladder::default())?;
        self.last_quoted = 0.0;
        Ok(())
    }

    /// One cycle. Returns false to stop.
    fn cycle(&mut self) -> Result<bool> {
        self.chain.refresh(&mut self.market)?;
        let reference = match self.source.next(&self.market) {
            Ok(p) => {
                self.last_price_at = Instant::now();
                p
            }
            Err(e) => {
                log(format!("price source failed: {e:#}"));
                self.last_reference
            }
        };
        let mine = self.my_orders()?;
        let inv = self.inventory(&mine)?;
        if let Some(prev) = self.last_inventory {
            let db = inv.base - prev.base;
            if db.abs() > 1e-12 {
                log(format!(
                    "filled: {} {:.9} base, inventory {:.6} base / {:.6} quote",
                    if db > 0.0 { "bought" } else { "sold" },
                    db.abs(),
                    inv.base,
                    inv.quote
                ));
            }
        }
        self.last_inventory = Some(inv);
        let equity = inv.equity(reference);
        let start = *self.start_equity.get_or_insert(equity);
        let verdict = check_risk(
            &self.limits,
            start,
            equity,
            self.last_price_at.elapsed().as_secs(),
            self.last_reference,
            reference,
        );
        match verdict {
            RiskVerdict::Halt(why) => {
                self.pull(&why)?;
                log(format!("halting: {why}"));
                return Ok(false);
            }
            RiskVerdict::Pause(why) => {
                self.pull(&why)?;
                self.last_reference = reference;
                return Ok(true);
            }
            RiskVerdict::Quote => {}
        }
        self.last_reference = reference;

        let ladder = build_ladder(reference, &inv, &self.params, &self.market.units);
        let lots: u64 = mine.iter().map(|o| o.base_lots).sum();
        let filled = mine.len() != self.last_count || lots != self.last_lots;
        if !(moved(self.last_quoted, reference, self.params.requote_bps) || filled) {
            return Ok(true);
        }
        let u = &self.market.units;
        let show = |qs: &[matchbook_client::mm::Quote]| {
            qs.iter()
                .map(|q| {
                    format!(
                        "{}x{}",
                        u.price_to_string(q.price),
                        u.lots_to_string(q.base_lots)
                    )
                })
                .collect::<Vec<_>>()
                .join(" ")
        };
        log(format!(
            "ref {reference:.6} center {:.6} inv {:.6}/{:.6} pnl {:+.6} | bids {} | asks {}",
            ladder.center,
            inv.base,
            inv.quote,
            equity - start,
            show(&ladder.bids),
            show(&ladder.asks)
        ));
        if self.dry_run {
            self.last_quoted = reference;
            return Ok(true);
        }
        let (all, cancels) = self.cancel_ix(&mine);
        self.send_batch(all, &cancels, &ladder)?;
        self.chain.refresh(&mut self.market)?;
        let now_mine = self.my_orders()?;
        self.last_count = now_mine.len();
        self.last_lots = now_mine.iter().map(|o| o.base_lots).sum();
        self.last_quoted = reference;
        Ok(true)
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let a = Args::parse();
    ensure!(
        a.levels <= MAX_LEVELS,
        "at most {MAX_LEVELS} levels per side"
    );
    ensure!(a.size > 0.0, "--size must be positive");
    let fees = Fees {
        cu_limit: a.connect.cu_limit,
        priority_micro_lamports: a.connect.priority_fee,
    };
    let url = cluster_url(&a.connect.url);
    let (signer, owner) = match &a.session_keypair {
        Some(p) => {
            let s = load_keypair(p)?;
            let owner = match a.owner {
                Some(o) => o,
                None => load_keypair(&a.connect.keypair_path())
                    .context("pass --owner (the seat owner) with a session key")?
                    .pubkey(),
            };
            (s, owner)
        }
        None => {
            let k = load_keypair(&a.connect.keypair_path())?;
            let o = k.pubkey();
            (k, o)
        }
    };
    let chain = Chain::new(&url, signer.insecure_clone(), fees);
    let market = chain.market(&a.market)?;
    let seat = market.require_seat(&owner)?;
    if signer.pubkey() != owner {
        let t = matchbook_client::chain::now();
        let ok = market.snap.delegates().iter().any(|d| {
            d.seat == seat
                && d.delegate == signer.pubkey()
                && d.permissions & matchbook_sdk::engine::permission::TRADE != 0
                && (d.expires_at == 0 || d.expires_at > t)
        });
        ensure!(
            ok,
            "{} is not a live trading session key of {owner}; create one with `matchbook session create`",
            signer.pubkey()
        );
    }
    let mut bot = Bot {
        chain,
        signer,
        owner,
        seat,
        params: QuoteParams {
            levels: a.levels,
            spread_bps: a.spread_bps,
            step_bps: a.step_bps,
            size: a.size,
            size_growth: a.size_growth,
            target_base: 0.0,
            max_position: a.max_position,
            skew_bps: a.skew_bps,
            max_notional: a.max_notional,
            requote_bps: a.requote_bps,
        },
        limits: RiskLimits {
            max_loss: a.max_loss,
            max_price_age_secs: a.max_price_age,
            max_jump_bps: a.max_jump_bps,
        },
        source: PriceSource::parse(&a.price)?,
        client_id: a.client_id,
        dry_run: a.dry_run,
        market,
        start_equity: None,
        last_quoted: 0.0,
        last_reference: 0.0,
        last_price_at: Instant::now(),
        last_lots: 0,
        last_count: 0,
        last_inventory: None,
    };
    bot.params.target_base = match a.target_base {
        Some(t) => t,
        None => bot.inventory(&[])?.base,
    };
    log(format!(
        "quoting {} for seat {seat} of {owner} with {}{}",
        a.market,
        bot.signer.pubkey(),
        if a.dry_run { " (dry run)" } else { "" }
    ));
    log(format!(
        "target {} base, max position {}, {} levels, spread {} bps, step {} bps",
        bot.params.target_base, a.max_position, a.levels, a.spread_bps, a.step_bps
    ));

    let stop = Arc::new(AtomicBool::new(false));
    {
        let stop = stop.clone();
        ctrlc::set_handler(move || stop.store(true, Ordering::SeqCst))?;
    }
    let mut n = 0u64;
    while !stop.load(Ordering::SeqCst) {
        match bot.cycle() {
            Ok(true) => {}
            Ok(false) => break,
            Err(e) => log(format!("cycle failed: {e:#}")),
        }
        n += 1;
        if a.cycles > 0 && n >= a.cycles {
            break;
        }
        let until = Instant::now() + Duration::from_millis(a.interval_ms);
        while Instant::now() < until && !stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    if !a.keep_quotes {
        bot.chain.refresh(&mut bot.market)?;
        bot.pull("exiting")?;
    }
    log("stopped");
    Ok(())
}
