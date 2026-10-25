//! Reference prices for the market maker.
//!
//! - `fixed:150.5` quotes around a constant.
//! - `random:150:20` walks randomly from 150, with 20 bps steps (for demos
//!   and load tests on a local validator).
//! - `book` follows the market itself: the last trade, else the mid.
//! - `http:<url>#<path>` reads a JSON number or numeric string at a dotted
//!   path, e.g.
//!   `http:https://api.coinbase.com/v2/prices/SOL-USD/spot#data.amount`.

use std::time::{Duration, Instant};

use anyhow::{bail, ensure, Context, Result};

use crate::chain::MarketCtx;

/// Where the reference price comes from.
#[derive(Debug, Clone)]
pub enum PriceSource {
    /// A constant.
    Fixed(f64),
    /// A random walk.
    Random {
        /// Current price.
        price: f64,
        /// Step size in bps.
        step_bps: f64,
        /// RNG state.
        state: u64,
    },
    /// The market's own last trade or mid.
    Book,
    /// A JSON HTTP endpoint.
    Http {
        /// URL to GET.
        url: String,
        /// Dotted path to the number.
        path: Vec<String>,
        /// Reused client.
        client: reqwest::blocking::Client,
    },
}

impl PriceSource {
    /// Parses a source spec (see the module docs).
    pub fn parse(spec: &str) -> Result<Self> {
        if let Some(v) = spec.strip_prefix("fixed:") {
            let p: f64 = v.parse().context("fixed:<price>")?;
            ensure!(p > 0.0, "price must be positive");
            return Ok(Self::Fixed(p));
        }
        if let Some(v) = spec.strip_prefix("random:") {
            let (start, step) = v.split_once(':').unwrap_or((v, "10"));
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(1, |d| d.as_nanos() as u64)
                | 1;
            return Ok(Self::Random {
                price: start.parse().context("random:<start>[:<step bps>]")?,
                step_bps: step.parse().context("random:<start>[:<step bps>]")?,
                state: seed,
            });
        }
        if spec == "book" {
            return Ok(Self::Book);
        }
        if let Some(v) = spec.strip_prefix("http:") {
            let (url, path) = v.split_once('#').context("http:<url>#<json.path>")?;
            return Ok(Self::Http {
                url: url.to_string(),
                path: path.split('.').map(str::to_string).collect(),
                client: reqwest::blocking::Client::builder()
                    .timeout(Duration::from_secs(5))
                    .build()?,
            });
        }
        bail!("unknown price source `{spec}` (fixed:, random:, book or http:)")
    }

    /// The next reference price in quote tokens per base token.
    pub fn next(&mut self, market: &MarketCtx) -> Result<f64> {
        match self {
            Self::Fixed(p) => Ok(*p),
            Self::Random {
                price,
                step_bps,
                state,
            } => {
                // xorshift64*: uniform in [-1, 1).
                *state ^= *state >> 12;
                *state ^= *state << 25;
                *state ^= *state >> 27;
                let r =
                    (state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64;
                *price *= 1.0 + (r * 2.0 - 1.0) * *step_bps / 10_000.0;
                Ok(*price)
            }
            Self::Book => {
                let h = market.snap.header();
                let u = &market.units;
                if h.last_price > 0 {
                    return Ok(u.price_to_f64(h.last_price));
                }
                ensure!(
                    h.best_bid_price > 0 && h.best_ask_price > 0 && h.best_ask_price != u64::MAX,
                    "the book has no trades and no two-sided market to follow"
                );
                Ok(u.price_to_f64(h.best_bid_price.saturating_add(h.best_ask_price) / 2))
            }
            Self::Http { url, path, client } => {
                let v: serde_json::Value = client
                    .get(url.as_str())
                    .send()?
                    .error_for_status()?
                    .json()?;
                let mut cur = &v;
                for k in path.iter() {
                    cur = match k.parse::<usize>() {
                        Ok(i) if cur.is_array() => &cur[i],
                        _ => &cur[k.as_str()],
                    };
                }
                let p = match cur {
                    serde_json::Value::Number(n) => n.as_f64(),
                    serde_json::Value::String(s) => s.parse().ok(),
                    _ => None,
                }
                .with_context(|| format!("no number at {} in {url}", path.join(".")))?;
                ensure!(p > 0.0, "price from {url} is not positive");
                Ok(p)
            }
        }
    }
}

/// A price with the time it was read.
#[derive(Debug, Clone, Copy)]
pub struct Timed {
    /// Quote tokens per base token.
    pub price: f64,
    /// When it was read.
    pub at: Instant,
}
