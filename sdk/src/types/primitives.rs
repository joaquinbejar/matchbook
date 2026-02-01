//! Primitive types for the Matchbook SDK.
//!
//! Provides type-safe wrappers for prices, quantities, and order sides.

use std::fmt;
use std::ops::{Add, Div, Mul, Sub};

use serde::{Deserialize, Serialize};

use crate::error::SdkError;

/// A price value in ticks.
///
/// Prices are represented as unsigned 64-bit integers in tick units.
/// The actual price depends on the market's tick size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Price(u64);

impl Price {
    /// Creates a new price.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw value.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }

    /// Returns zero price.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Returns true if the price is zero.
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Checked addition.
    ///
    /// # Errors
    ///
    /// Returns `SdkError::Overflow` if the result overflows.
    pub fn checked_add(self, other: Self) -> Result<Self, SdkError> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or(SdkError::Overflow)
    }

    /// Checked subtraction.
    ///
    /// # Errors
    ///
    /// Returns `SdkError::Underflow` if the result underflows.
    pub fn checked_sub(self, other: Self) -> Result<Self, SdkError> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .ok_or(SdkError::Underflow)
    }

    /// Checked multiplication.
    ///
    /// # Errors
    ///
    /// Returns `SdkError::Overflow` if the result overflows.
    pub fn checked_mul(self, factor: u64) -> Result<Self, SdkError> {
        self.0
            .checked_mul(factor)
            .map(Self)
            .ok_or(SdkError::Overflow)
    }

    /// Checked division.
    ///
    /// # Errors
    ///
    /// Returns `SdkError::DivisionByZero` if divisor is zero.
    pub fn checked_div(self, divisor: u64) -> Result<Self, SdkError> {
        if divisor == 0 {
            return Err(SdkError::DivisionByZero);
        }
        Ok(Self(self.0 / divisor))
    }

    /// Saturating addition.
    #[must_use]
    pub fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// Saturating subtraction.
    #[must_use]
    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

impl Default for Price {
    fn default() -> Self {
        Self::zero()
    }
}

impl fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for Price {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<Price> for u64 {
    fn from(price: Price) -> Self {
        price.0
    }
}

impl Add for Price {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }
}

impl Sub for Price {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

impl Mul<u64> for Price {
    type Output = Self;

    fn mul(self, factor: u64) -> Self {
        Self(self.0.saturating_mul(factor))
    }
}

impl Div<u64> for Price {
    type Output = Self;

    fn div(self, divisor: u64) -> Self {
        Self(self.0 / divisor)
    }
}

/// A quantity value in lots.
///
/// Quantities are represented as unsigned 64-bit integers in lot units.
/// The actual quantity depends on the market's lot size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Quantity(u64);

impl Quantity {
    /// Creates a new quantity.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw value.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }

    /// Returns zero quantity.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Returns true if the quantity is zero.
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Checked addition.
    ///
    /// # Errors
    ///
    /// Returns `SdkError::Overflow` if the result overflows.
    pub fn checked_add(self, other: Self) -> Result<Self, SdkError> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or(SdkError::Overflow)
    }

    /// Checked subtraction.
    ///
    /// # Errors
    ///
    /// Returns `SdkError::Underflow` if the result underflows.
    pub fn checked_sub(self, other: Self) -> Result<Self, SdkError> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .ok_or(SdkError::Underflow)
    }

    /// Checked multiplication.
    ///
    /// # Errors
    ///
    /// Returns `SdkError::Overflow` if the result overflows.
    pub fn checked_mul(self, factor: u64) -> Result<Self, SdkError> {
        self.0
            .checked_mul(factor)
            .map(Self)
            .ok_or(SdkError::Overflow)
    }

    /// Checked division.
    ///
    /// # Errors
    ///
    /// Returns `SdkError::DivisionByZero` if divisor is zero.
    pub fn checked_div(self, divisor: u64) -> Result<Self, SdkError> {
        if divisor == 0 {
            return Err(SdkError::DivisionByZero);
        }
        Ok(Self(self.0 / divisor))
    }

    /// Saturating addition.
    #[must_use]
    pub fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// Saturating subtraction.
    #[must_use]
    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    /// Returns the minimum of two quantities.
    #[must_use]
    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }

    /// Returns the maximum of two quantities.
    #[must_use]
    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }
}

impl Default for Quantity {
    fn default() -> Self {
        Self::zero()
    }
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for Quantity {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<Quantity> for u64 {
    fn from(quantity: Quantity) -> Self {
        quantity.0
    }
}

impl Add for Quantity {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }
}

impl Sub for Quantity {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

impl Mul<u64> for Quantity {
    type Output = Self;

    fn mul(self, factor: u64) -> Self {
        Self(self.0.saturating_mul(factor))
    }
}

impl Div<u64> for Quantity {
    type Output = Self;

    fn div(self, divisor: u64) -> Self {
        Self(self.0 / divisor)
    }
}

/// Order side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    /// Buy order.
    Bid,
    /// Sell order.
    Ask,
}

impl Side {
    /// Returns true if this is a bid (buy) order.
    #[must_use]
    pub const fn is_bid(&self) -> bool {
        matches!(self, Self::Bid)
    }

    /// Returns true if this is an ask (sell) order.
    #[must_use]
    pub const fn is_ask(&self) -> bool {
        matches!(self, Self::Ask)
    }

    /// Returns the opposite side.
    #[must_use]
    pub const fn opposite(&self) -> Self {
        match self {
            Self::Bid => Self::Ask,
            Self::Ask => Self::Bid,
        }
    }
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bid => write!(f, "bid"),
            Self::Ask => write!(f, "ask"),
        }
    }
}

impl From<Side> for u8 {
    fn from(side: Side) -> Self {
        match side {
            Side::Bid => 0,
            Side::Ask => 1,
        }
    }
}

impl TryFrom<u8> for Side {
    type Error = SdkError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Bid),
            1 => Ok(Self::Ask),
            _ => Err(SdkError::Deserialization(format!(
                "invalid side value: {}",
                value
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_new() {
        let price = Price::new(1000);
        assert_eq!(price.value(), 1000);
    }

    #[test]
    fn test_price_zero() {
        let price = Price::zero();
        assert!(price.is_zero());
        assert_eq!(price.value(), 0);
    }

    #[test]
    fn test_price_checked_add() {
        let a = Price::new(100);
        let b = Price::new(50);
        assert_eq!(a.checked_add(b).map(|p| p.value()), Ok(150));
    }

    #[test]
    fn test_price_checked_add_overflow() {
        let a = Price::new(u64::MAX);
        let b = Price::new(1);
        assert!(a.checked_add(b).is_err());
    }

    #[test]
    fn test_price_checked_sub() {
        let a = Price::new(100);
        let b = Price::new(50);
        assert_eq!(a.checked_sub(b).map(|p| p.value()), Ok(50));
    }

    #[test]
    fn test_price_checked_sub_underflow() {
        let a = Price::new(50);
        let b = Price::new(100);
        assert!(a.checked_sub(b).is_err());
    }

    #[test]
    fn test_price_checked_mul() {
        let price = Price::new(100);
        assert_eq!(price.checked_mul(5).map(|p| p.value()), Ok(500));
    }

    #[test]
    fn test_price_checked_div() {
        let price = Price::new(100);
        assert_eq!(price.checked_div(5).map(|p| p.value()), Ok(20));
    }

    #[test]
    fn test_price_checked_div_by_zero() {
        let price = Price::new(100);
        assert!(price.checked_div(0).is_err());
    }

    #[test]
    fn test_price_saturating_ops() {
        let a = Price::new(u64::MAX);
        let b = Price::new(1);
        assert_eq!(a.saturating_add(b).value(), u64::MAX);

        let c = Price::new(0);
        let d = Price::new(1);
        assert_eq!(c.saturating_sub(d).value(), 0);
    }

    #[test]
    fn test_price_display() {
        let price = Price::new(1000);
        assert_eq!(price.to_string(), "1000");
    }

    #[test]
    fn test_price_from_u64() {
        let price: Price = 1000u64.into();
        assert_eq!(price.value(), 1000);
    }

    #[test]
    fn test_price_into_u64() {
        let price = Price::new(1000);
        let value: u64 = price.into();
        assert_eq!(value, 1000);
    }

    #[test]
    fn test_price_arithmetic() {
        let a = Price::new(100);
        let b = Price::new(50);

        assert_eq!((a + b).value(), 150);
        assert_eq!((a - b).value(), 50);
        assert_eq!((a * 2).value(), 200);
        assert_eq!((a / 2).value(), 50);
    }

    #[test]
    fn test_quantity_new() {
        let qty = Quantity::new(1000);
        assert_eq!(qty.value(), 1000);
    }

    #[test]
    fn test_quantity_zero() {
        let qty = Quantity::zero();
        assert!(qty.is_zero());
    }

    #[test]
    fn test_quantity_checked_ops() {
        let a = Quantity::new(100);
        let b = Quantity::new(50);

        assert_eq!(a.checked_add(b).map(|q| q.value()), Ok(150));
        assert_eq!(a.checked_sub(b).map(|q| q.value()), Ok(50));
        assert_eq!(a.checked_mul(5).map(|q| q.value()), Ok(500));
        assert_eq!(a.checked_div(5).map(|q| q.value()), Ok(20));
    }

    #[test]
    fn test_quantity_min_max() {
        let a = Quantity::new(100);
        let b = Quantity::new(50);

        assert_eq!(a.min(b).value(), 50);
        assert_eq!(a.max(b).value(), 100);
    }

    #[test]
    fn test_side_is_bid_ask() {
        assert!(Side::Bid.is_bid());
        assert!(!Side::Bid.is_ask());
        assert!(Side::Ask.is_ask());
        assert!(!Side::Ask.is_bid());
    }

    #[test]
    fn test_side_opposite() {
        assert_eq!(Side::Bid.opposite(), Side::Ask);
        assert_eq!(Side::Ask.opposite(), Side::Bid);
    }

    #[test]
    fn test_side_display() {
        assert_eq!(Side::Bid.to_string(), "bid");
        assert_eq!(Side::Ask.to_string(), "ask");
    }

    #[test]
    fn test_side_from_u8() {
        assert_eq!(Side::try_from(0u8), Ok(Side::Bid));
        assert_eq!(Side::try_from(1u8), Ok(Side::Ask));
        assert!(Side::try_from(2u8).is_err());
    }

    #[test]
    fn test_side_into_u8() {
        assert_eq!(u8::from(Side::Bid), 0);
        assert_eq!(u8::from(Side::Ask), 1);
    }

    #[test]
    fn test_price_serde() {
        let price = Price::new(1000);
        let json = serde_json::to_string(&price).expect("serialize");
        assert_eq!(json, "1000");

        let parsed: Price = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, price);
    }

    #[test]
    fn test_quantity_serde() {
        let qty = Quantity::new(500);
        let json = serde_json::to_string(&qty).expect("serialize");
        assert_eq!(json, "500");

        let parsed: Quantity = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, qty);
    }

    #[test]
    fn test_side_serde() {
        let bid = Side::Bid;
        let json = serde_json::to_string(&bid).expect("serialize");
        assert_eq!(json, "\"bid\"");

        let parsed: Side = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, bid);
    }
}
