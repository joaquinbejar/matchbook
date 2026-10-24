//! Conversions between human amounts and the market's lots and atoms.
//!
//! Prices on the book are quote lots per base lot. People think in quote
//! tokens per base token, so every command takes and prints those, and this
//! module converts exactly (decimal strings, integer math).

use anyhow::{bail, ensure, Context, Result};
use matchbook_sdk::engine::MarketHeader;

/// A non-negative decimal number held exactly: `mantissa / 10^scale`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decimal {
    /// Digits without the point.
    pub mantissa: u128,
    /// Digits after the point.
    pub scale: u32,
}

impl Decimal {
    /// Parses `"150"`, `"150.25"` or `".5"`.
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim().replace('_', "");
        let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
        ensure!(
            !(int.is_empty() && frac.is_empty())
                && int.chars().chain(frac.chars()).all(|c| c.is_ascii_digit()),
            "not a decimal number: {s}"
        );
        ensure!(frac.len() <= 30, "too many decimals: {s}");
        let digits = format!("{int}{frac}");
        let mantissa = if digits.is_empty() {
            0
        } else {
            digits.parse::<u128>().context("number too large")?
        };
        Ok(Self {
            mantissa,
            scale: u32::try_from(frac.len())?,
        })
    }

    /// `self * num / den`, rounded down, as an exact integer.
    fn scaled(self, num: u128, den: u128) -> Result<u128> {
        let den = den
            .checked_mul(pow10(self.scale)?)
            .context("number too precise")?;
        let n = self.mantissa.checked_mul(num).context("number too large")?;
        Ok(n / den)
    }

    /// `self * num / den`, rounded to nearest.
    fn scaled_round(self, num: u128, den: u128) -> Result<u128> {
        let den = den
            .checked_mul(pow10(self.scale)?)
            .context("number too precise")?;
        let n = self.mantissa.checked_mul(num).context("number too large")?;
        Ok((n + den / 2) / den)
    }
}

fn pow10(e: u32) -> Result<u128> {
    10u128.checked_pow(e).context("exponent too large")
}

fn to_u64(v: u128) -> Result<u64> {
    u64::try_from(v).context("value does not fit in u64")
}

/// Formats `v / 10^decimals` without trailing zeros.
#[must_use]
pub fn format_atoms(v: u128, decimals: u8) -> String {
    let d = u32::from(decimals);
    let p = 10u128.pow(d);
    let (int, frac) = (v / p, v % p);
    if frac == 0 {
        return int.to_string();
    }
    let f = format!("{frac:0width$}", width = d as usize);
    format!("{int}.{}", f.trim_end_matches('0'))
}

/// Lot sizes and decimals of a market.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Units {
    /// Base atoms per base lot.
    pub base_lot_size: u64,
    /// Quote atoms per quote lot.
    pub quote_lot_size: u64,
    /// Base mint decimals.
    pub base_decimals: u8,
    /// Quote mint decimals.
    pub quote_decimals: u8,
}

impl Units {
    /// Units of a market header with the mints' decimals.
    #[must_use]
    pub fn new(h: &MarketHeader, base_decimals: u8, quote_decimals: u8) -> Self {
        Self {
            base_lot_size: h.base_lot_size.max(1),
            quote_lot_size: h.quote_lot_size.max(1),
            base_decimals,
            quote_decimals,
        }
    }

    fn base_one(&self) -> u128 {
        10u128.pow(u32::from(self.base_decimals))
    }

    fn quote_one(&self) -> u128 {
        10u128.pow(u32::from(self.quote_decimals))
    }

    /// Book price (quote lots per base lot) of `price` quote tokens per base
    /// token, rounded to the nearest tick.
    pub fn price_to_lots(&self, price: &str) -> Result<u64> {
        let p = Decimal::parse(price)?;
        let num = self.quote_one() * u128::from(self.base_lot_size);
        let den = self.base_one() * u128::from(self.quote_lot_size);
        let lots = to_u64(p.scaled_round(num, den)?)?;
        if lots == 0 {
            bail!(
                "price {price} is below one tick ({})",
                self.price_to_string(1)
            );
        }
        Ok(lots)
    }

    /// Book price of a float price (for the bot), rounded to nearest.
    #[must_use]
    pub fn price_f64_to_lots(&self, price: f64) -> u64 {
        let v = price * self.quote_one() as f64 * self.base_lot_size as f64
            / (self.base_one() as f64 * self.quote_lot_size as f64);
        if v.is_finite() && v > 0.0 {
            v.round() as u64
        } else {
            0
        }
    }

    /// Quote tokens per base token of a book price.
    #[must_use]
    pub fn price_to_f64(&self, lots: u64) -> f64 {
        lots as f64 * self.quote_lot_size as f64 * self.base_one() as f64
            / (self.base_lot_size as f64 * self.quote_one() as f64)
    }

    /// Prints a book price in quote tokens per base token.
    #[must_use]
    pub fn price_to_string(&self, lots: u64) -> String {
        // price = lots * qls * 10^bd / (bls * 10^qd); print with enough digits.
        let num = u128::from(lots) * u128::from(self.quote_lot_size) * self.base_one();
        let den = u128::from(self.base_lot_size);
        let extra = 6u8;
        let scaled = num * 10u128.pow(u32::from(extra)) / den;
        format_atoms(scaled, self.quote_decimals + extra)
    }

    /// Base lots in `size` base tokens, rounded down.
    pub fn size_to_lots(&self, size: &str) -> Result<u64> {
        let s = Decimal::parse(size)?;
        let lots = to_u64(s.scaled(self.base_one(), u128::from(self.base_lot_size))?)?;
        ensure!(
            lots > 0,
            "size {size} is below one lot ({})",
            self.lots_to_string(1)
        );
        Ok(lots)
    }

    /// Prints base lots as base tokens.
    #[must_use]
    pub fn lots_to_string(&self, lots: u64) -> String {
        format_atoms(
            u128::from(lots) * u128::from(self.base_lot_size),
            self.base_decimals,
        )
    }

    /// Base tokens in base lots.
    #[must_use]
    pub fn lots_to_f64(&self, lots: u64) -> f64 {
        lots as f64 * self.base_lot_size as f64 / self.base_one() as f64
    }

    /// Atoms of `amount` tokens with `decimals`.
    pub fn amount_to_atoms(amount: &str, decimals: u8) -> Result<u64> {
        let a = Decimal::parse(amount)?;
        to_u64(a.scaled(10u128.pow(u32::from(decimals)), 1)?)
    }

    /// Prints base atoms as base tokens.
    #[must_use]
    pub fn base_to_string(&self, atoms: u64) -> String {
        format_atoms(u128::from(atoms), self.base_decimals)
    }

    /// Prints quote atoms as quote tokens.
    #[must_use]
    pub fn quote_to_string(&self, atoms: u64) -> String {
        format_atoms(u128::from(atoms), self.quote_decimals)
    }

    /// Base atoms as tokens.
    #[must_use]
    pub fn base_to_f64(&self, atoms: u64) -> f64 {
        atoms as f64 / self.base_one() as f64
    }

    /// Quote atoms as tokens.
    #[must_use]
    pub fn quote_to_f64(&self, atoms: u64) -> f64 {
        atoms as f64 / self.quote_one() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The demo market: 9-decimal base, 6-decimal quote, 0.001 base per lot,
    // 1 quote atom per lot. A price of 150 is 150,000 lots.
    fn demo() -> Units {
        Units {
            base_lot_size: 1_000_000,
            quote_lot_size: 1,
            base_decimals: 9,
            quote_decimals: 6,
        }
    }

    #[test]
    fn prices_round_trip() -> Result<()> {
        let u = demo();
        assert_eq!(u.price_to_lots("150")?, 150_000);
        assert_eq!(u.price_to_lots("150.125")?, 150_125);
        assert_eq!(u.price_to_lots("150.1254")?, 150_125);
        assert_eq!(u.price_to_string(150_125), "150.125");
        assert_eq!(u.price_f64_to_lots(150.1), 150_100);
        assert!(u.price_to_lots("0.0001").is_err());
        Ok(())
    }

    #[test]
    fn sizes_and_amounts() -> Result<()> {
        let u = demo();
        assert_eq!(u.size_to_lots("1.5")?, 1_500);
        assert_eq!(u.lots_to_string(1_500), "1.5");
        assert_eq!(Units::amount_to_atoms("2.25", 6)?, 2_250_000);
        assert_eq!(u.quote_to_string(2_250_000), "2.25");
        assert!(u.size_to_lots("0.0001").is_err());
        assert!(Decimal::parse("1.2.3").is_err());
        assert!(Decimal::parse("-1").is_err());
        Ok(())
    }
}
