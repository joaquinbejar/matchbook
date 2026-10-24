//! `matchbook`: command-line client for the MatchBook order book.
//!
//! Prices are quote tokens per base token and sizes are base tokens; the
//! client converts them to the market's lots. Run `matchbook --help`.

use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{ensure, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use matchbook_client::chain::{
    load_keypair, now, write_keypair, Chain, Executed, MarketCtx, Pubkey,
};
use matchbook_client::units::Units;
use matchbook_client::ConnectArgs;
use matchbook_sdk::engine::{
    permission, MarketStatus, OrderType, SelfTradeBehavior, Side, TriggerDirection,
    MARKET_FLAG_PERMISSIONED, REVERSE_SPREAD_SCALE,
};
use matchbook_sdk::events::Event;
use matchbook_sdk::ix::{self, MarketAccounts};
use matchbook_sdk::{pda, CreateMarketArgs, DelegateArgs, OrderArgs, StopArgs, SwapArgs, NO_HINT};
use solana_keypair::Keypair;
use solana_signer::Signer;

#[derive(Parser)]
#[command(name = "matchbook", version, about = "MatchBook order book client")]
struct Cli {
    #[command(flatten)]
    connect: ConnectArgs,
    /// Print machine-readable JSON where a command supports it.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show the wallet address and SOL balance.
    Address,
    /// Write a new keypair file (Solana CLI format).
    Keygen {
        /// Where to write it.
        out: PathBuf,
    },
    /// Request SOL from the cluster faucet (localnet, devnet).
    Airdrop {
        /// SOL to request.
        sol: f64,
        /// Recipient (default: the wallet).
        #[arg(long)]
        to: Option<Pubkey>,
    },
    /// Test tokens: create mints, mint, read balances.
    #[command(subcommand)]
    Token(TokenCmd),
    /// Create, inspect and administer markets.
    #[command(subcommand)]
    Market(MarketCmd),
    /// Show the order book.
    Book {
        /// Market address.
        market: Pubkey,
        /// Levels per side.
        #[arg(long, default_value_t = 10)]
        depth: usize,
    },
    /// Claim a seat (needed before deposits and resting orders).
    #[command(subcommand)]
    Seat(SeatCmd),
    /// Show a seat's free and locked balances and the wallet's tokens.
    Balance {
        /// Market address.
        market: Pubkey,
        /// Seat owner (default: the wallet).
        #[arg(long)]
        owner: Option<Pubkey>,
    },
    /// Move tokens from the wallet into the seat.
    Deposit {
        /// Market address.
        market: Pubkey,
        /// Base tokens.
        #[arg(long)]
        base: Option<String>,
        /// Quote tokens.
        #[arg(long)]
        quote: Option<String>,
    },
    /// Move tokens from the seat back to the wallet.
    Withdraw {
        /// Market address.
        market: Pubkey,
        /// Base tokens.
        #[arg(long)]
        base: Option<String>,
        /// Quote tokens.
        #[arg(long)]
        quote: Option<String>,
        /// Withdraw every free token on both sides.
        #[arg(long, conflicts_with_all = ["base", "quote"])]
        all: bool,
    },
    /// Place a limit, post-only, IOC, FOK or reverse order.
    Place(PlaceArgs),
    /// Cancel one order or stop by id.
    Cancel {
        /// Market address.
        market: Pubkey,
        /// Order or stop id.
        order_id: u64,
        #[command(flatten)]
        trader: TraderArgs,
    },
    /// Cancel every order of the seat, on one side or both.
    CancelAll {
        /// Market address.
        market: Pubkey,
        /// Only this side.
        #[arg(long)]
        side: Option<SideArg>,
        #[command(flatten)]
        trader: TraderArgs,
    },
    /// Cancel every order and stop with a client order id.
    CancelClient {
        /// Market address.
        market: Pubkey,
        /// Client order id.
        client_order_id: u64,
        #[command(flatten)]
        trader: TraderArgs,
    },
    /// Shrink an order in place, keeping its time priority.
    Reduce {
        /// Market address.
        market: Pubkey,
        /// Order id.
        order_id: u64,
        /// New size in base tokens.
        new_size: String,
        #[command(flatten)]
        trader: TraderArgs,
    },
    /// List a seat's resting orders and stops.
    Orders {
        /// Market address.
        market: Pubkey,
        /// Seat owner (default: the wallet).
        #[arg(long)]
        owner: Option<Pubkey>,
    },
    /// Place a stop order that waits for a trade at its trigger price.
    Stop(StopCmdArgs),
    /// Place a stop-loss and a take-profit that cancel each other (OCO).
    Bracket(BracketArgs),
    /// Trade straight from the wallet's token accounts, with no seat.
    Swap {
        /// Market address.
        market: Pubkey,
        /// buy or sell.
        side: SideArg,
        /// Buy: quote tokens to spend. Sell: base tokens to sell.
        amount: String,
        /// Worst accepted shortfall from the quote (bps).
        #[arg(long, default_value_t = 50)]
        slippage_bps: u64,
    },
    /// Session keys: delegates that trade for a seat but cannot withdraw.
    #[command(subcommand)]
    Session(SessionCmd),
}

#[derive(Subcommand)]
enum TokenCmd {
    /// Create a mint with the wallet as mint authority.
    CreateMint {
        /// Decimals.
        #[arg(long, default_value_t = 6)]
        decimals: u8,
        /// Use Token-2022.
        #[arg(long)]
        token_2022: bool,
    },
    /// Mint tokens (the wallet must be the mint authority).
    Mint {
        /// Mint address.
        mint: Pubkey,
        /// Tokens to mint.
        amount: String,
        /// Recipient (default: the wallet).
        #[arg(long)]
        to: Option<Pubkey>,
    },
    /// Show a token balance.
    Balance {
        /// Mint address.
        mint: Pubkey,
        /// Owner (default: the wallet).
        #[arg(long)]
        owner: Option<Pubkey>,
    },
}

#[derive(Subcommand)]
enum MarketCmd {
    /// Create a market. Prints its address.
    Create {
        /// Base mint.
        #[arg(long)]
        base_mint: Pubkey,
        /// Quote mint.
        #[arg(long)]
        quote_mint: Pubkey,
        /// Base atoms per lot (the size increment).
        #[arg(long)]
        base_lot_size: u64,
        /// Quote atoms per lot (with the base lot, sets the tick).
        #[arg(long)]
        quote_lot_size: u64,
        /// Minimum order size in lots.
        #[arg(long, default_value_t = 1)]
        min_base_lots: u64,
        /// Taker fee (bps).
        #[arg(long, default_value_t = 10)]
        taker_fee_bps: u32,
        /// Maker rebate (bps), paid from the taker fee.
        #[arg(long, default_value_t = 2)]
        maker_rebate_bps: u32,
        /// Share of the taker fee paid to referrers (bps of the fee).
        #[arg(long, default_value_t = 0)]
        referral_share_bps: u16,
        /// Nodes to preallocate (orders, seats, delegates, stops). Grows on demand.
        #[arg(long, default_value_t = 64)]
        initial_nodes: u32,
        /// Only seats the authority approves; no seatless swaps.
        #[arg(long)]
        permissioned: bool,
        /// Fee collector and status authority (default: the wallet).
        #[arg(long)]
        authority: Option<Pubkey>,
    },
    /// Show a market's parameters and statistics.
    Show {
        /// Market address.
        market: Pubkey,
    },
    /// Change the status (authority only).
    SetStatus {
        /// Market address.
        market: Pubkey,
        /// New status.
        status: StatusArg,
    },
    /// Send the accrued fees to a quote token account (authority only).
    CollectFees {
        /// Market address.
        market: Pubkey,
        /// Destination quote token account (default: the wallet's ATA).
        #[arg(long)]
        to: Option<Pubkey>,
    },
    /// Place stops whose trigger was reached (permissionless).
    TriggerStops {
        /// Market address.
        market: Pubkey,
        /// Most stops to place.
        #[arg(long, default_value_t = 8)]
        limit: u32,
    },
    /// Remove expired orders (permissionless).
    Prune {
        /// Market address.
        market: Pubkey,
        /// Most orders to remove.
        #[arg(long, default_value_t = 32)]
        limit: u32,
    },
}

#[derive(Subcommand)]
enum SeatCmd {
    /// Claim a seat for the wallet.
    Claim {
        /// Market address.
        market: Pubkey,
        /// Authority keypair, for permissioned markets.
        #[arg(long)]
        authority_keypair: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum SessionCmd {
    /// Create a session key: a new keypair allowed to trade for the seat.
    Create {
        /// Market address.
        market: Pubkey,
        /// Where to write the session keypair.
        #[arg(long, default_value = "session.json")]
        out: PathBuf,
        /// Lifetime: 90s, 30m, 12h, 7d or seconds (0 = no expiry).
        #[arg(long, default_value = "1d")]
        expires_in: String,
        /// Only cancel and reduce, never place.
        #[arg(long)]
        cancel_only: bool,
        /// SOL to send the session key for fees.
        #[arg(long, default_value_t = 0.05)]
        fund: f64,
    },
    /// List the delegates of a seat.
    List {
        /// Market address.
        market: Pubkey,
        /// Seat owner (default: the wallet).
        #[arg(long)]
        owner: Option<Pubkey>,
    },
    /// Revoke a delegate.
    Revoke {
        /// Market address.
        market: Pubkey,
        /// The delegate's address.
        delegate: Pubkey,
    },
}

/// Who signs a trading instruction.
#[derive(clap::Args, Clone)]
struct TraderArgs {
    /// Sign with this keypair, a session key of the seat's owner.
    #[arg(long)]
    signer: Option<PathBuf>,
    /// The seat's owner when signing with a session key (default: the wallet).
    #[arg(long)]
    owner: Option<Pubkey>,
}

#[derive(clap::Args)]
struct PlaceArgs {
    /// Market address.
    market: Pubkey,
    /// buy or sell.
    side: SideArg,
    /// Size in base tokens.
    size: String,
    /// Limit price in quote tokens per base token.
    price: String,
    /// Order type.
    #[arg(long = "type", value_enum, default_value_t = TypeArg::Limit)]
    order_type: TypeArg,
    /// Client order id, echoed in events and usable to cancel.
    #[arg(long, default_value_t = 0)]
    client_id: u64,
    /// Expire a resting order after this long (90s, 30m, 12h, 7d).
    #[arg(long)]
    expires_in: Option<String>,
    /// Make it a reverse order: each fill re-posts on the other side this
    /// many bps from the fill price.
    #[arg(long)]
    reverse_spread_bps: Option<f64>,
    /// What to do when it would match an order of the same seat.
    #[arg(long, value_enum, default_value_t = SelfTradeArg::CancelProvide)]
    self_trade: SelfTradeArg,
    /// For IOC buys: most quote tokens to spend, fees included.
    #[arg(long)]
    quote_budget: Option<String>,
    #[command(flatten)]
    trader: TraderArgs,
}

#[derive(clap::Args)]
struct StopCmdArgs {
    /// Market address.
    market: Pubkey,
    /// Side of the order placed when it triggers.
    side: SideArg,
    /// Size in base tokens.
    size: String,
    /// Trigger price.
    #[arg(long)]
    trigger: String,
    /// Trigger when a trade is at or above, or at or below, the trigger.
    #[arg(long, value_enum)]
    when: WhenArg,
    /// Limit price of the triggered order (default: trigger ± slippage).
    #[arg(long)]
    limit: Option<String>,
    /// Without --limit: how far past the trigger the order may fill (bps).
    #[arg(long, default_value_t = 100)]
    slippage_bps: u64,
    /// Order type when triggered (default: IOC without --limit, limit with).
    #[arg(long = "type", value_enum)]
    order_type: Option<TypeArg>,
    /// Pair with this stop as one-cancels-other.
    #[arg(long)]
    oco_with: Option<u64>,
    /// Client order id.
    #[arg(long, default_value_t = 0)]
    client_id: u64,
    #[command(flatten)]
    trader: TraderArgs,
}

#[derive(clap::Args)]
struct BracketArgs {
    /// Market address.
    market: Pubkey,
    /// sell closes a long position, buy closes a short one.
    side: SideArg,
    /// Size in base tokens.
    size: String,
    /// Stop-loss trigger price.
    #[arg(long)]
    stop_loss: String,
    /// Take-profit trigger price.
    #[arg(long)]
    take_profit: String,
    /// How far past each trigger its order may fill (bps).
    #[arg(long, default_value_t = 100)]
    slippage_bps: u64,
    /// Client order id for both stops.
    #[arg(long, default_value_t = 0)]
    client_id: u64,
    #[command(flatten)]
    trader: TraderArgs,
}

#[derive(Clone, Copy, ValueEnum)]
enum SideArg {
    Buy,
    Sell,
}

impl From<SideArg> for Side {
    fn from(s: SideArg) -> Self {
        match s {
            SideArg::Buy => Side::Bid,
            SideArg::Sell => Side::Ask,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum TypeArg {
    Limit,
    PostOnly,
    PostOnlySlide,
    Ioc,
    Fok,
}

impl From<TypeArg> for OrderType {
    fn from(t: TypeArg) -> Self {
        match t {
            TypeArg::Limit => OrderType::Limit,
            TypeArg::PostOnly => OrderType::PostOnly,
            TypeArg::PostOnlySlide => OrderType::PostOnlySlide,
            TypeArg::Ioc => OrderType::ImmediateOrCancel,
            TypeArg::Fok => OrderType::FillOrKill,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum SelfTradeArg {
    CancelProvide,
    DecrementTake,
    Abort,
}

impl From<SelfTradeArg> for SelfTradeBehavior {
    fn from(s: SelfTradeArg) -> Self {
        match s {
            SelfTradeArg::CancelProvide => SelfTradeBehavior::CancelProvide,
            SelfTradeArg::DecrementTake => SelfTradeBehavior::DecrementTake,
            SelfTradeArg::Abort => SelfTradeBehavior::Abort,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum WhenArg {
    Above,
    Below,
}

#[derive(Clone, Copy, ValueEnum)]
enum StatusArg {
    Active,
    CancelOnly,
    Closed,
}

/// Parses 90s, 30m, 12h, 7d or plain seconds.
fn parse_duration(s: &str) -> Result<u64> {
    let s = s.trim();
    let (num, mult) = match s.chars().last() {
        Some('s') => (&s[..s.len() - 1], 1),
        Some('m') => (&s[..s.len() - 1], 60),
        Some('h') => (&s[..s.len() - 1], 3_600),
        Some('d') => (&s[..s.len() - 1], 86_400),
        _ => (s, 1),
    };
    Ok(u64::from_str(num).with_context(|| format!("bad duration {s}"))? * mult)
}

fn side_name(s: Side) -> &'static str {
    match s {
        Side::Bid => "buy",
        Side::Ask => "sell",
    }
}

fn print_executed(e: &Executed, u: &Units) {
    println!("signature {}", e.signature);
    if let Some(cu) = e.compute_units {
        println!("compute units {cu}");
    }
    for ev in &e.events {
        match ev {
            Event::Fill(f) => println!(
                "  fill {} @ {} against order {} of {}",
                u.lots_to_string(f.base_lots),
                u.price_to_string(f.price),
                f.maker_order_id,
                f.maker
            ),
            Event::Out(o) => println!(
                "  out {} {} @ {} (order {}, {:?})",
                side_name(o.side),
                u.lots_to_string(o.base_lots),
                u.price_to_string(o.price),
                o.order_id,
                o.reason
            ),
            Event::Reversed(r) => println!(
                "  reversed into {} {} @ {} (order {})",
                side_name(r.side),
                u.lots_to_string(r.base_lots),
                u.price_to_string(r.price),
                r.order_id
            ),
        }
    }
}

struct App {
    chain: Chain,
    json: bool,
}

/// The signer of a trading instruction and the seat it acts for.
struct Acting {
    signer: Option<Keypair>,
    owner: Pubkey,
}

impl Acting {
    fn key(&self, chain: &Chain) -> Pubkey {
        self.signer
            .as_ref()
            .map_or_else(|| chain.payer_key(), Signer::pubkey)
    }
}

impl App {
    fn acting(&self, t: &TraderArgs) -> Result<Acting> {
        let signer = t.signer.as_deref().map(load_keypair).transpose()?;
        let owner = t.owner.unwrap_or_else(|| self.chain.payer_key());
        if signer.is_none() {
            ensure!(
                owner == self.chain.payer_key(),
                "--owner needs --signer (a session key)"
            );
        }
        Ok(Acting { signer, owner })
    }

    fn send_as(&self, a: &Acting, ixs: &[solana_message::Instruction]) -> Result<Executed> {
        match &a.signer {
            Some(s) => self.chain.execute(ixs, &[s]),
            None => self.chain.execute(ixs, &[]),
        }
    }

    fn token_accounts(&self, m: &MarketCtx, owner: &Pubkey) -> (Pubkey, Pubkey) {
        let a = &m.accounts;
        (
            pda::associated_token_address(owner, &a.base_mint, &a.base_token_program),
            pda::associated_token_address(owner, &a.quote_mint, &a.quote_token_program),
        )
    }

    fn run(&self, cmd: Cmd) -> Result<()> {
        let c = &self.chain;
        match cmd {
            Cmd::Address => {
                let me = c.payer_key();
                println!("{me}");
                println!("{} SOL", c.sol_balance(&me)? as f64 / 1e9);
            }
            Cmd::Keygen { .. } => unreachable!("handled before connecting"),
            Cmd::Airdrop { sol, to } => {
                let to = to.unwrap_or_else(|| c.payer_key());
                let sig = c.airdrop(&to, (sol * 1e9) as u64)?;
                println!("airdropped {sol} SOL to {to}: {sig}");
            }
            Cmd::Token(t) => self.token(t)?,
            Cmd::Market(m) => self.market(m)?,
            Cmd::Book { market, depth } => self.book(&market, depth)?,
            Cmd::Seat(SeatCmd::Claim {
                market,
                authority_keypair,
            }) => {
                let auth = authority_keypair.as_deref().map(load_keypair).transpose()?;
                let me = c.payer_key();
                let ix = ix::claim_seat(
                    &me,
                    &me,
                    &market,
                    auth.as_ref().map(|a| a.pubkey()).as_ref(),
                );
                let signers: Vec<&Keypair> = auth.iter().collect();
                let sig = c.send(&[ix], &signers)?;
                let seat = c.market(&market)?.require_seat(&me)?;
                println!("seat {seat} claimed: {sig}");
            }
            Cmd::Balance { market, owner } => self.balance(&market, owner)?,
            Cmd::Deposit {
                market,
                base,
                quote,
            } => self.move_funds(&market, base, quote, true)?,
            Cmd::Withdraw {
                market,
                base,
                quote,
                all,
            } => {
                if all {
                    let m = c.market(&market)?;
                    let me = c.payer_key();
                    let (_, seat) = m.snap.seat(&me)?.context("no seat")?;
                    let b = m.units.base_to_string(seat.base_free);
                    let q = m.units.quote_to_string(seat.quote_free);
                    self.move_funds(&market, Some(b), Some(q), false)?;
                } else {
                    self.move_funds(&market, base, quote, false)?;
                }
            }
            Cmd::Place(p) => self.place(p)?,
            Cmd::Cancel {
                market,
                order_id,
                trader,
            } => {
                let a = self.acting(&trader)?;
                let m = c.market(&market)?;
                let seat = m.require_seat(&a.owner)?;
                let e =
                    self.send_as(&a, &[ix::cancel_order(&a.key(c), &market, seat, order_id)])?;
                println!("cancelled {order_id}");
                print_executed(&e, &m.units);
            }
            Cmd::CancelAll {
                market,
                side,
                trader,
            } => {
                let a = self.acting(&trader)?;
                let m = c.market(&market)?;
                let seat = m.require_seat(&a.owner)?;
                let before = m.snap.seat_orders(seat)?.len();
                let ix = ix::cancel_all(&a.key(c), &market, seat, side.map(Side::from));
                let e = self.send_as(&a, &[ix])?;
                let after = c.market(&market)?.snap.seat_orders(seat)?.len();
                println!("cancelled {} orders", before.saturating_sub(after));
                print_executed(&e, &m.units);
            }
            Cmd::CancelClient {
                market,
                client_order_id,
                trader,
            } => {
                let a = self.acting(&trader)?;
                let m = c.market(&market)?;
                let seat = m.require_seat(&a.owner)?;
                let ix = ix::cancel_by_client_id(&a.key(c), &market, seat, client_order_id);
                let e = self.send_as(&a, &[ix])?;
                println!("cancelled client id {client_order_id}");
                print_executed(&e, &m.units);
            }
            Cmd::Reduce {
                market,
                order_id,
                new_size,
                trader,
            } => {
                let a = self.acting(&trader)?;
                let m = c.market(&market)?;
                let seat = m.require_seat(&a.owner)?;
                let lots = m.units.size_to_lots(&new_size)?;
                let ix = ix::reduce_order(&a.key(c), &market, seat, order_id, lots);
                let e = self.send_as(&a, &[ix])?;
                println!("reduced {order_id} to {}", m.units.lots_to_string(lots));
                print_executed(&e, &m.units);
            }
            Cmd::Orders { market, owner } => self.orders(&market, owner)?,
            Cmd::Stop(s) => self.stop(s)?,
            Cmd::Bracket(b) => self.bracket(b)?,
            Cmd::Swap {
                market,
                side,
                amount,
                slippage_bps,
            } => self.swap(&market, side.into(), &amount, slippage_bps)?,
            Cmd::Session(s) => self.session(s)?,
        }
        Ok(())
    }

    fn token(&self, t: TokenCmd) -> Result<()> {
        let c = &self.chain;
        match t {
            TokenCmd::CreateMint {
                decimals,
                token_2022,
            } => {
                let tp = if token_2022 {
                    matchbook_sdk::TOKEN_2022_PROGRAM_ID
                } else {
                    matchbook_sdk::TOKEN_PROGRAM_ID
                };
                println!("{}", c.create_mint(decimals, &tp)?);
            }
            TokenCmd::Mint { mint, amount, to } => {
                let to = to.unwrap_or_else(|| c.payer_key());
                let dec = self.decimals(&mint)?;
                let atoms = Units::amount_to_atoms(&amount, dec)?;
                let ata = c.mint_to(&mint, &to, atoms)?;
                println!("minted {amount} to {to} ({ata})");
            }
            TokenCmd::Balance { mint, owner } => {
                let owner = owner.unwrap_or_else(|| c.payer_key());
                let tp = c.token_program_of(&mint)?;
                let ata = pda::associated_token_address(&owner, &mint, &tp);
                let dec = self.decimals(&mint)?;
                println!(
                    "{}",
                    matchbook_client::units::format_atoms(u128::from(c.token_balance(&ata)?), dec)
                );
            }
        }
        Ok(())
    }

    fn decimals(&self, mint: &Pubkey) -> Result<u8> {
        let a = self.chain.rpc.get_account(mint)?;
        a.data.get(44).copied().context("not a mint")
    }

    fn market(&self, cmd: MarketCmd) -> Result<()> {
        let c = &self.chain;
        match cmd {
            MarketCmd::Create {
                base_mint,
                quote_mint,
                base_lot_size,
                quote_lot_size,
                min_base_lots,
                taker_fee_bps,
                maker_rebate_bps,
                referral_share_bps,
                initial_nodes,
                permissioned,
                authority,
            } => {
                let market = Keypair::new();
                let accounts = MarketAccounts::new(
                    market.pubkey(),
                    base_mint,
                    quote_mint,
                    c.token_program_of(&base_mint)?,
                    c.token_program_of(&quote_mint)?,
                );
                let args = CreateMarketArgs {
                    base_lot_size,
                    quote_lot_size,
                    min_base_lots,
                    taker_fee_bps,
                    maker_rebate_bps,
                    initial_nodes,
                    referral_share_bps,
                    flags: if permissioned {
                        MARKET_FLAG_PERMISSIONED as u16
                    } else {
                        0
                    },
                    authority: authority.unwrap_or_else(|| c.payer_key()).to_bytes(),
                };
                let ix = ix::create_market(&c.payer_key(), &accounts, &args);
                let sig = c.send(&[ix], &[&market])?;
                if self.json {
                    println!(
                        "{}",
                        serde_json::json!({"market": market.pubkey().to_string(), "signature": sig.to_string()})
                    );
                } else {
                    println!("market {}", market.pubkey());
                    println!("signature {sig}");
                }
            }
            MarketCmd::Show { market } => {
                let m = c.market(&market)?;
                let h = m.snap.header();
                let u = &m.units;
                let status = match h.status {
                    0 => "active",
                    1 => "cancel-only",
                    _ => "closed",
                };
                let tick = u.price_to_string(1);
                if self.json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "market": market.to_string(),
                            "baseMint": m.accounts.base_mint.to_string(),
                            "quoteMint": m.accounts.quote_mint.to_string(),
                            "authority": Pubkey::new_from_array(h.authority).to_string(),
                            "status": status,
                            "baseLotSize": h.base_lot_size,
                            "quoteLotSize": h.quote_lot_size,
                            "tick": tick,
                            "minSize": u.lots_to_string(h.min_base_lots),
                            "takerFeeBps": h.taker_fee_bps,
                            "makerRebateBps": h.maker_rebate_bps,
                            "orders": h.order_count,
                            "seats": h.trader_count,
                            "stops": h.stop_count,
                            "lastPrice": u.price_to_string(h.last_price),
                            "feesAccrued": u.quote_to_string(h.fees_accrued),
                        })
                    );
                    return Ok(());
                }
                println!("market        {market}");
                println!(
                    "base mint     {} ({} decimals)",
                    m.accounts.base_mint, u.base_decimals
                );
                println!(
                    "quote mint    {} ({} decimals)",
                    m.accounts.quote_mint, u.quote_decimals
                );
                println!("authority     {}", Pubkey::new_from_array(h.authority));
                println!(
                    "status        {status}{}",
                    if h.flags & MARKET_FLAG_PERMISSIONED != 0 {
                        ", permissioned"
                    } else {
                        ""
                    }
                );
                println!(
                    "lot           {} base, tick {} quote",
                    u.lots_to_string(1),
                    tick
                );
                println!("min size      {}", u.lots_to_string(h.min_base_lots));
                println!(
                    "fees          taker {} bps, maker rebate {} bps",
                    h.taker_fee_bps, h.maker_rebate_bps
                );
                println!(
                    "orders        {}  seats {}  stops {}  delegates {}",
                    h.order_count, h.trader_count, h.stop_count, h.delegate_count
                );
                println!(
                    "last price    {}",
                    if h.last_price == 0 {
                        "-".into()
                    } else {
                        u.price_to_string(h.last_price)
                    }
                );
                println!(
                    "volume        {} base, {} quote",
                    u.lots_to_string(h.total_base_lots_traded),
                    u.quote_to_string(h.total_quote_atoms_traded)
                );
                println!("fees accrued  {}", u.quote_to_string(h.fees_accrued));
            }
            MarketCmd::SetStatus { market, status } => {
                let s = match status {
                    StatusArg::Active => MarketStatus::Active,
                    StatusArg::CancelOnly => MarketStatus::CancelOnly,
                    StatusArg::Closed => MarketStatus::Closed,
                };
                println!(
                    "{}",
                    c.send(&[ix::set_status(&c.payer_key(), &market, s)], &[])?
                );
            }
            MarketCmd::CollectFees { market, to } => {
                let m = c.market(&market)?;
                let a = &m.accounts;
                let dest = to.unwrap_or_else(|| {
                    pda::associated_token_address(
                        &c.payer_key(),
                        &a.quote_mint,
                        &a.quote_token_program,
                    )
                });
                let fees = m.snap.header().fees_accrued;
                let ixs = [
                    c.ensure_ata_ix(&c.payer_key(), &a.quote_mint, &a.quote_token_program),
                    ix::collect_fees(&c.payer_key(), a, &dest),
                ];
                let sig = c.send(&ixs, &[])?;
                println!("collected {} quote: {sig}", m.units.quote_to_string(fees));
            }
            MarketCmd::TriggerStops { market, limit } => {
                let m = c.market(&market)?;
                let e = c.execute(&[ix::trigger_stops(&market, limit)], &[])?;
                print_executed(&e, &m.units);
            }
            MarketCmd::Prune { market, limit } => {
                let m = c.market(&market)?;
                let e = c.execute(&[ix::prune_expired(&market, limit)], &[])?;
                print_executed(&e, &m.units);
            }
        }
        Ok(())
    }

    fn book(&self, market: &Pubkey, depth: usize) -> Result<()> {
        let m = self.chain.market(market)?;
        let u = &m.units;
        let t = now();
        let asks = m.snap.levels(Side::Ask, t, depth)?;
        let bids = m.snap.levels(Side::Bid, t, depth)?;
        if self.json {
            let lv = |l: &[matchbook_sdk::engine::BookLevel]| {
                l.iter()
                    .map(|l| serde_json::json!({"price": u.price_to_string(l.price), "size": u.lots_to_string(l.base_lots), "orders": l.orders}))
                    .collect::<Vec<_>>()
            };
            println!(
                "{}",
                serde_json::json!({"bids": lv(&bids), "asks": lv(&asks)})
            );
            return Ok(());
        }
        println!("{:>16} {:>16} {:>7}", "price", "size", "orders");
        for l in asks.iter().rev() {
            println!(
                "{:>16} {:>16} {:>7}  ask",
                u.price_to_string(l.price),
                u.lots_to_string(l.base_lots),
                l.orders
            );
        }
        match (bids.first(), asks.first()) {
            (Some(b), Some(a)) => {
                let mid = (u.price_to_f64(a.price) + u.price_to_f64(b.price)) / 2.0;
                let spread = (u.price_to_f64(a.price) - u.price_to_f64(b.price)) / mid * 10_000.0;
                println!("{:-^42}", format!(" mid {mid:.6}  spread {spread:.1} bps "));
            }
            _ => println!("{:-^42}", ""),
        }
        for l in &bids {
            println!(
                "{:>16} {:>16} {:>7}  bid",
                u.price_to_string(l.price),
                u.lots_to_string(l.base_lots),
                l.orders
            );
        }
        Ok(())
    }

    fn balance(&self, market: &Pubkey, owner: Option<Pubkey>) -> Result<()> {
        let c = &self.chain;
        let owner = owner.unwrap_or_else(|| c.payer_key());
        let m = c.market(market)?;
        let u = &m.units;
        let (base_ata, quote_ata) = self.token_accounts(&m, &owner);
        let wallet = (c.token_balance(&base_ata)?, c.token_balance(&quote_ata)?);
        let seat = m.snap.seat(&owner)?;
        let (free, locked) = match seat {
            Some((i, t)) => ((t.base_free, t.quote_free), m.snap.locked(i)?),
            None => ((0, 0), (0, 0)),
        };
        if self.json {
            println!(
                "{}",
                serde_json::json!({
                    "owner": owner.to_string(),
                    "seat": seat.map(|(i, _)| i),
                    "base": {"free": u.base_to_string(free.0), "locked": u.base_to_string(locked.0), "wallet": u.base_to_string(wallet.0)},
                    "quote": {"free": u.quote_to_string(free.1), "locked": u.quote_to_string(locked.1), "wallet": u.quote_to_string(wallet.1)},
                })
            );
            return Ok(());
        }
        match seat {
            Some((i, _)) => println!("seat {i} of {owner}"),
            None => println!("{owner} has no seat on this market"),
        }
        println!("{:>8} {:>18} {:>18} {:>18}", "", "free", "locked", "wallet");
        println!(
            "{:>8} {:>18} {:>18} {:>18}",
            "base",
            u.base_to_string(free.0),
            u.base_to_string(locked.0),
            u.base_to_string(wallet.0)
        );
        println!(
            "{:>8} {:>18} {:>18} {:>18}",
            "quote",
            u.quote_to_string(free.1),
            u.quote_to_string(locked.1),
            u.quote_to_string(wallet.1)
        );
        Ok(())
    }

    fn move_funds(
        &self,
        market: &Pubkey,
        base: Option<String>,
        quote: Option<String>,
        deposit: bool,
    ) -> Result<()> {
        let c = &self.chain;
        let me = c.payer_key();
        let m = c.market(market)?;
        let a = &m.accounts;
        let seat = m.require_seat(&me)?;
        let (base_ata, quote_ata) = self.token_accounts(&m, &me);
        let mut ixs = Vec::new();
        for (amount, mint, tp, ata, dec) in [
            (
                base,
                a.base_mint,
                a.base_token_program,
                base_ata,
                m.units.base_decimals,
            ),
            (
                quote,
                a.quote_mint,
                a.quote_token_program,
                quote_ata,
                m.units.quote_decimals,
            ),
        ] {
            let Some(amount) = amount else { continue };
            let atoms = Units::amount_to_atoms(&amount, dec)?;
            if atoms == 0 {
                continue;
            }
            if deposit {
                ixs.push(ix::deposit(&me, a, seat, &mint, &ata, atoms));
            } else {
                ixs.push(c.ensure_ata_ix(&me, &mint, &tp));
                ixs.push(ix::withdraw(&me, a, seat, &mint, &ata, atoms));
            }
        }
        ensure!(
            !ixs.is_empty(),
            "nothing to move: pass --base and/or --quote"
        );
        let sig = c.send(&ixs, &[])?;
        println!("{} {sig}", if deposit { "deposited" } else { "withdrew" });
        Ok(())
    }

    fn place(&self, p: PlaceArgs) -> Result<()> {
        let c = &self.chain;
        let a = self.acting(&p.trader)?;
        let m = c.market(&p.market)?;
        let u = &m.units;
        let seat = m.require_seat(&a.owner)?;
        let side: Side = p.side.into();
        let price = u.price_to_lots(&p.price)?;
        let lots = u.size_to_lots(&p.size)?;
        let reverse_spread = match p.reverse_spread_bps {
            Some(bps) => {
                ensure!(
                    bps > 0.0 && bps < 10_000.0,
                    "reverse spread must be in (0, 10000) bps"
                );
                (bps * f64::from(REVERSE_SPREAD_SCALE) / 10_000.0).round() as u32
            }
            None => 0,
        };
        let quote_budget = match &p.quote_budget {
            Some(q) => Units::amount_to_atoms(q, u.quote_decimals)?,
            None => 0,
        };
        let order = OrderArgs {
            order_type: OrderType::from(p.order_type) as u8,
            self_trade: SelfTradeBehavior::from(p.self_trade) as u8,
            client_order_id: p.client_id,
            expires_at: match &p.expires_in {
                Some(d) => now() + parse_duration(d)?,
                None => 0,
            },
            quote_budget,
            reverse_spread,
            ..OrderArgs::limit(side as u8, price, lots)
        };
        let ix = ix::place_order(&a.key(c), &p.market, seat, &order, None);
        let e = self.send_as(&a, &[ix])?;
        match e.returned_u64() {
            Some(id) if id != 0 => println!(
                "order {id}: {} {} @ {}",
                side_name(side),
                u.lots_to_string(lots),
                u.price_to_string(price)
            ),
            _ => println!(
                "{} {} @ {}: nothing rests",
                side_name(side),
                u.lots_to_string(lots),
                u.price_to_string(price)
            ),
        }
        print_executed(&e, u);
        Ok(())
    }

    fn orders(&self, market: &Pubkey, owner: Option<Pubkey>) -> Result<()> {
        let c = &self.chain;
        let owner = owner.unwrap_or_else(|| c.payer_key());
        let m = c.market(market)?;
        let u = &m.units;
        let seat = m.require_seat(&owner)?;
        let orders = m.snap.seat_orders(seat)?;
        let stops = m.snap.seat_stops(seat)?;
        if self.json {
            let o: Vec<_> = orders
                .iter()
                .map(|o| serde_json::json!({"id": o.order_id, "side": side_name(o.side), "price": u.price_to_string(o.price), "size": u.lots_to_string(o.base_lots), "clientId": o.client_order_id, "expiresAt": o.expires_at}))
                .collect();
            let s: Vec<_> = stops
                .iter()
                .map(|s| serde_json::json!({"id": s.stop_id, "side": side_name(s.side), "trigger": u.price_to_string(s.trigger_price), "when": if s.direction == TriggerDirection::Above {"above"} else {"below"}, "price": u.price_to_string(s.price), "size": u.lots_to_string(s.base_lots), "ocoWith": s.oco_with}))
                .collect();
            println!("{}", serde_json::json!({"orders": o, "stops": s}));
            return Ok(());
        }
        println!("orders ({})", orders.len());
        for o in &orders {
            let exp = if o.expires_at == 0 {
                String::new()
            } else {
                format!("  expires {}", o.expires_at)
            };
            println!(
                "  {:>22} {:<4} {:>14} @ {:<14} cid {}{exp}",
                o.order_id,
                side_name(o.side),
                u.lots_to_string(o.base_lots),
                u.price_to_string(o.price),
                o.client_order_id
            );
        }
        println!("stops ({})", stops.len());
        for s in &stops {
            let when = if s.direction == TriggerDirection::Above {
                ">="
            } else {
                "<="
            };
            let oco = s
                .oco_with
                .map(|id| format!("  oco {id}"))
                .unwrap_or_default();
            println!(
                "  {:>22} when {when} {:<12} {:<4} {:>14} @ {:<14} {:?}{oco}",
                s.stop_id,
                u.price_to_string(s.trigger_price),
                side_name(s.side),
                u.lots_to_string(s.base_lots),
                u.price_to_string(s.price),
                s.order_type
            );
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn stop_args(
        side: Side,
        lots: u64,
        trigger: u64,
        limit: Option<u64>,
        slippage_bps: u64,
        order_type: Option<OrderType>,
        oco_with: u64,
        client_id: u64,
    ) -> StopArgs {
        let limit_price = limit.unwrap_or_else(|| {
            let t = u128::from(trigger);
            let slip = u128::from(slippage_bps);
            let p = match side {
                Side::Ask => t * (10_000 - slip.min(9_999)) / 10_000,
                Side::Bid => t * (10_000 + slip) / 10_000,
            };
            u64::try_from(p).unwrap_or(u64::MAX).max(1)
        });
        let ot = order_type.unwrap_or(if limit.is_some() {
            OrderType::Limit
        } else {
            OrderType::ImmediateOrCancel
        });
        StopArgs {
            order: OrderArgs {
                order_type: ot as u8,
                client_order_id: client_id,
                ..OrderArgs::limit(side as u8, limit_price, lots)
            },
            trigger_price: trigger,
            oco_with,
        }
    }

    fn stop(&self, s: StopCmdArgs) -> Result<()> {
        let c = &self.chain;
        let a = self.acting(&s.trader)?;
        let m = c.market(&s.market)?;
        let u = &m.units;
        let seat = m.require_seat(&a.owner)?;
        let side: Side = s.side.into();
        let trigger = u.price_to_lots(&s.trigger)?;
        let limit = s.limit.as_deref().map(|l| u.price_to_lots(l)).transpose()?;
        let args = Self::stop_args(
            side,
            u.size_to_lots(&s.size)?,
            trigger,
            limit,
            s.slippage_bps,
            s.order_type.map(OrderType::from),
            s.oco_with.unwrap_or(0),
            s.client_id,
        );
        let dir = match s.when {
            WhenArg::Above => TriggerDirection::Above,
            WhenArg::Below => TriggerDirection::Below,
        };
        let e = self.send_as(
            &a,
            &[ix::place_stop(&a.key(c), &s.market, seat, dir, &args)],
        )?;
        let id = e
            .returned_u64()
            .context("the program returned no stop id")?;
        println!(
            "stop {id}: {} {} @ {} when a trade is {} {}",
            side_name(side),
            u.lots_to_string(args.order.base_lots),
            u.price_to_string(args.order.price),
            if dir == TriggerDirection::Above {
                "at or above"
            } else {
                "at or below"
            },
            u.price_to_string(trigger)
        );
        print_executed(&e, u);
        Ok(())
    }

    fn bracket(&self, b: BracketArgs) -> Result<()> {
        let c = &self.chain;
        let a = self.acting(&b.trader)?;
        let m = c.market(&b.market)?;
        let u = &m.units;
        let seat = m.require_seat(&a.owner)?;
        let side: Side = b.side.into();
        let lots = u.size_to_lots(&b.size)?;
        let sl = u.price_to_lots(&b.stop_loss)?;
        let tp = u.price_to_lots(&b.take_profit)?;
        // Closing a long (sell): the loss is below, the profit above.
        let (sl_dir, tp_dir) = match side {
            Side::Ask => {
                ensure!(
                    sl < tp,
                    "selling: the stop-loss must be below the take-profit"
                );
                (TriggerDirection::Below, TriggerDirection::Above)
            }
            Side::Bid => {
                ensure!(
                    sl > tp,
                    "buying: the stop-loss must be above the take-profit"
                );
                (TriggerDirection::Above, TriggerDirection::Below)
            }
        };
        let key = a.key(c);
        let sl_args = Self::stop_args(side, lots, sl, None, b.slippage_bps, None, 0, b.client_id);
        let e = self.send_as(
            &a,
            &[ix::place_stop(&key, &b.market, seat, sl_dir, &sl_args)],
        )?;
        let sl_id = e.returned_u64().context("no stop id")?;
        // The take-profit rests as a limit at its trigger once reached.
        let tp_args = Self::stop_args(
            side,
            lots,
            tp,
            Some(tp),
            0,
            Some(OrderType::Limit),
            sl_id,
            b.client_id,
        );
        let e2 = self.send_as(
            &a,
            &[ix::place_stop(&key, &b.market, seat, tp_dir, &tp_args)],
        )?;
        let tp_id = e2.returned_u64().context("no stop id")?;
        println!("stop-loss   {sl_id} at {}", u.price_to_string(sl));
        println!(
            "take-profit {tp_id} at {} (one cancels the other)",
            u.price_to_string(tp)
        );
        Ok(())
    }

    fn swap(&self, market: &Pubkey, side: Side, amount: &str, slippage_bps: u64) -> Result<()> {
        let c = &self.chain;
        let me = c.payer_key();
        let m = c.market(market)?;
        let u = &m.units;
        let a = &m.accounts;
        let atoms = match side {
            Side::Bid => Units::amount_to_atoms(amount, u.quote_decimals)?,
            Side::Ask => Units::amount_to_atoms(amount, u.base_decimals)?,
        };
        let params = m.snap.exact_in_params(side, atoms);
        let q = m.snap.quote_swap(&params, now())?;
        ensure!(q.base_lots > 0, "nothing to trade against");
        let out = match side {
            Side::Bid => q.base_atoms,
            Side::Ask => q.quote_net(side),
        };
        let min_out = u64::try_from(
            u128::from(out) * u128::from(10_000 - slippage_bps.min(10_000)) / 10_000,
        )?;
        let (base_ata, quote_ata) = self.token_accounts(&m, &me);
        let args = SwapArgs {
            base_lots: params.base_lots,
            quote_budget: params.quote_budget,
            limit_price: params.limit_price,
            min_out,
            match_limit: params.match_limit,
            referrer_seat: NO_HINT,
        };
        let ixs = [
            c.ensure_ata_ix(&me, &a.base_mint, &a.base_token_program),
            c.ensure_ata_ix(&me, &a.quote_mint, &a.quote_token_program),
            ix::swap(&me, a, &base_ata, &quote_ata, side, &args, None),
        ];
        let e = c.execute(&ixs, &[])?;
        let avg = if q.base_lots > 0 {
            q.quote_atoms as f64 / u.quote_to_f64(1).recip() / u.lots_to_f64(q.base_lots)
        } else {
            0.0
        };
        println!(
            "{} {} base for {} quote (fee {}), average {avg:.6}",
            if side == Side::Bid { "bought" } else { "sold" },
            u.base_to_string(q.base_atoms),
            u.quote_to_string(q.quote_net(side)),
            u.quote_to_string(q.fee_atoms)
        );
        print_executed(&e, u);
        Ok(())
    }

    fn session(&self, s: SessionCmd) -> Result<()> {
        let c = &self.chain;
        let me = c.payer_key();
        match s {
            SessionCmd::Create {
                market,
                out,
                expires_in,
                cancel_only,
                fund,
            } => {
                ensure!(
                    !out.exists(),
                    "{} exists; pick another --out",
                    out.display()
                );
                let m = c.market(&market)?;
                let seat = m.require_seat(&me)?;
                let session = Keypair::new();
                let secs = parse_duration(&expires_in)?;
                let args = DelegateArgs {
                    delegate: session.pubkey().to_bytes(),
                    expires_at: if secs == 0 { 0 } else { now() + secs },
                    permissions: if cancel_only {
                        permission::CANCEL
                    } else {
                        permission::TRADE
                    },
                };
                let mut ixs = vec![ix::set_delegate(&me, &market, seat, &args)];
                if fund > 0.0 {
                    ixs.push(solana_system_interface::instruction::transfer(
                        &me,
                        &session.pubkey(),
                        (fund * 1e9) as u64,
                    ));
                }
                write_keypair(&session, &out)?;
                let sig = c.send(&ixs, &[])?;
                println!(
                    "session key {} written to {}",
                    session.pubkey(),
                    out.display()
                );
                println!(
                    "{} for seat {seat}, {}",
                    if cancel_only {
                        "cancel-only"
                    } else {
                        "trading"
                    },
                    if secs == 0 {
                        "no expiry".to_string()
                    } else {
                        format!("expires in {expires_in}")
                    }
                );
                println!("signature {sig}");
            }
            SessionCmd::List { market, owner } => {
                let owner = owner.unwrap_or(me);
                let m = c.market(&market)?;
                let seat = m.require_seat(&owner)?;
                let t = now();
                for d in m.snap.delegates().into_iter().filter(|d| d.seat == seat) {
                    let perms = match d.permissions {
                        p if p & permission::TRADE != 0 => "trade",
                        p if p & permission::CANCEL != 0 => "cancel",
                        _ => "none",
                    };
                    let exp = match d.expires_at {
                        0 => "never".to_string(),
                        e if e <= t => "expired".to_string(),
                        e => format!("in {}s", e - t),
                    };
                    println!("{}  {perms}  expires {exp}", d.delegate);
                }
            }
            SessionCmd::Revoke { market, delegate } => {
                let m = c.market(&market)?;
                let seat = m.require_seat(&me)?;
                let sig = c.send(&[ix::remove_delegate(&me, &market, seat, &delegate)], &[])?;
                println!("revoked {delegate}: {sig}");
            }
        }
        Ok(())
    }
}

fn main() {
    let cli = Cli::parse();
    if let Cmd::Keygen { out } = &cli.cmd {
        let result = (|| -> Result<()> {
            ensure!(!out.exists(), "{} exists", out.display());
            let kp = Keypair::new();
            write_keypair(&kp, out)?;
            println!("{}", kp.pubkey());
            Ok(())
        })();
        if let Err(e) = result {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
        return;
    }
    let result = cli.connect.connect().and_then(|chain| {
        App {
            chain,
            json: cli.json,
        }
        .run(cli.cmd)
    });
    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
