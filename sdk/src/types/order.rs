//! Order types for the Matchbook SDK.
//!
//! Provides order-related types including order type, status, and parameters.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::primitives::{Price, Quantity, Side};
use crate::error::SdkError;

/// Order type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    /// Standard limit order.
    Limit,
    /// Post-only order (maker only, rejected if would take).
    PostOnly,
    /// Immediate-or-cancel (fill what you can, cancel rest).
    ImmediateOrCancel,
    /// Fill-or-kill (fill entirely or cancel entirely).
    FillOrKill,
}

impl OrderType {
    /// Returns true if this order type can rest on the book.
    #[must_use]
    pub const fn can_rest(&self) -> bool {
        matches!(self, Self::Limit | Self::PostOnly)
    }

    /// Returns true if this order type requires immediate execution.
    #[must_use]
    pub const fn is_immediate(&self) -> bool {
        matches!(self, Self::ImmediateOrCancel | Self::FillOrKill)
    }
}

impl fmt::Display for OrderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Limit => write!(f, "limit"),
            Self::PostOnly => write!(f, "post_only"),
            Self::ImmediateOrCancel => write!(f, "ioc"),
            Self::FillOrKill => write!(f, "fok"),
        }
    }
}

impl From<OrderType> for u8 {
    fn from(order_type: OrderType) -> Self {
        match order_type {
            OrderType::Limit => 0,
            OrderType::PostOnly => 1,
            OrderType::ImmediateOrCancel => 2,
            OrderType::FillOrKill => 3,
        }
    }
}

impl TryFrom<u8> for OrderType {
    type Error = SdkError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Limit),
            1 => Ok(Self::PostOnly),
            2 => Ok(Self::ImmediateOrCancel),
            3 => Ok(Self::FillOrKill),
            _ => Err(SdkError::Deserialization(format!(
                "invalid order type: {}",
                value
            ))),
        }
    }
}

/// Time in force for an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeInForce {
    /// Good til cancelled.
    GoodTilCancelled,
    /// Immediate or cancel.
    ImmediateOrCancel,
    /// Fill or kill.
    FillOrKill,
    /// Post only.
    PostOnly,
}

impl TimeInForce {
    /// Converts to order type.
    #[must_use]
    pub const fn to_order_type(&self) -> OrderType {
        match self {
            Self::GoodTilCancelled => OrderType::Limit,
            Self::ImmediateOrCancel => OrderType::ImmediateOrCancel,
            Self::FillOrKill => OrderType::FillOrKill,
            Self::PostOnly => OrderType::PostOnly,
        }
    }
}

impl fmt::Display for TimeInForce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GoodTilCancelled => write!(f, "gtc"),
            Self::ImmediateOrCancel => write!(f, "ioc"),
            Self::FillOrKill => write!(f, "fok"),
            Self::PostOnly => write!(f, "post_only"),
        }
    }
}

impl From<TimeInForce> for u8 {
    fn from(tif: TimeInForce) -> Self {
        match tif {
            TimeInForce::GoodTilCancelled => 0,
            TimeInForce::ImmediateOrCancel => 1,
            TimeInForce::FillOrKill => 2,
            TimeInForce::PostOnly => 3,
        }
    }
}

impl TryFrom<u8> for TimeInForce {
    type Error = SdkError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::GoodTilCancelled),
            1 => Ok(Self::ImmediateOrCancel),
            2 => Ok(Self::FillOrKill),
            3 => Ok(Self::PostOnly),
            _ => Err(SdkError::Deserialization(format!(
                "invalid time in force: {}",
                value
            ))),
        }
    }
}

/// Order status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    /// Order is open and resting on the book.
    Open,
    /// Order is partially filled.
    PartiallyFilled,
    /// Order is completely filled.
    Filled,
    /// Order was cancelled.
    Cancelled,
    /// Order expired.
    Expired,
}

impl OrderStatus {
    /// Returns true if the order is still active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Open | Self::PartiallyFilled)
    }

    /// Returns true if the order is terminal (no longer active).
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Filled | Self::Cancelled | Self::Expired)
    }
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::PartiallyFilled => write!(f, "partially_filled"),
            Self::Filled => write!(f, "filled"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Expired => write!(f, "expired"),
        }
    }
}

impl From<OrderStatus> for u8 {
    fn from(status: OrderStatus) -> Self {
        match status {
            OrderStatus::Open => 0,
            OrderStatus::PartiallyFilled => 1,
            OrderStatus::Filled => 2,
            OrderStatus::Cancelled => 3,
            OrderStatus::Expired => 4,
        }
    }
}

impl TryFrom<u8> for OrderStatus {
    type Error = SdkError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Open),
            1 => Ok(Self::PartiallyFilled),
            2 => Ok(Self::Filled),
            3 => Ok(Self::Cancelled),
            4 => Ok(Self::Expired),
            _ => Err(SdkError::Deserialization(format!(
                "invalid order status: {}",
                value
            ))),
        }
    }
}

/// Self-trade behavior.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelfTradeBehavior {
    /// Cancel the taker order.
    #[default]
    CancelTaker,
    /// Cancel the maker order.
    CancelMaker,
    /// Decrement the taker order quantity.
    DecrementTake,
    /// Allow self-trade.
    Allow,
}

impl fmt::Display for SelfTradeBehavior {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CancelTaker => write!(f, "cancel_taker"),
            Self::CancelMaker => write!(f, "cancel_maker"),
            Self::DecrementTake => write!(f, "decrement_take"),
            Self::Allow => write!(f, "allow"),
        }
    }
}

impl From<SelfTradeBehavior> for u8 {
    fn from(stb: SelfTradeBehavior) -> Self {
        match stb {
            SelfTradeBehavior::CancelTaker => 0,
            SelfTradeBehavior::CancelMaker => 1,
            SelfTradeBehavior::DecrementTake => 2,
            SelfTradeBehavior::Allow => 3,
        }
    }
}

impl TryFrom<u8> for SelfTradeBehavior {
    type Error = SdkError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::CancelTaker),
            1 => Ok(Self::CancelMaker),
            2 => Ok(Self::DecrementTake),
            3 => Ok(Self::Allow),
            _ => Err(SdkError::Deserialization(format!(
                "invalid self trade behavior: {}",
                value
            ))),
        }
    }
}

/// Parameters for placing an order.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceOrderParams {
    /// Order side.
    pub side: Side,

    /// Price in ticks.
    pub price: Price,

    /// Quantity in lots.
    pub quantity: Quantity,

    /// Order type.
    pub order_type: OrderType,

    /// Client-provided order ID.
    pub client_order_id: u64,

    /// Self-trade behavior.
    #[serde(default)]
    pub self_trade_behavior: SelfTradeBehavior,

    /// Optional expiry slot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_slot: Option<u64>,
}

impl PlaceOrderParams {
    /// Creates new order parameters.
    #[must_use]
    pub fn new(side: Side, price: Price, quantity: Quantity, order_type: OrderType) -> Self {
        Self {
            side,
            price,
            quantity,
            order_type,
            client_order_id: 0,
            self_trade_behavior: SelfTradeBehavior::default(),
            expiry_slot: None,
        }
    }

    /// Sets the client order ID.
    #[must_use]
    pub fn with_client_order_id(mut self, id: u64) -> Self {
        self.client_order_id = id;
        self
    }

    /// Sets the self-trade behavior.
    #[must_use]
    pub fn with_self_trade_behavior(mut self, stb: SelfTradeBehavior) -> Self {
        self.self_trade_behavior = stb;
        self
    }

    /// Sets the expiry slot.
    #[must_use]
    pub fn with_expiry(mut self, slot: u64) -> Self {
        self.expiry_slot = Some(slot);
        self
    }
}

/// An order.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    /// Order ID (u128 encoded as string for JSON compatibility).
    pub order_id: String,

    /// Owner address (base58 encoded).
    pub owner: String,

    /// Market address (base58 encoded).
    pub market: String,

    /// Order side.
    pub side: Side,

    /// Price in ticks.
    pub price: Price,

    /// Original quantity in lots.
    pub original_quantity: Quantity,

    /// Remaining quantity in lots.
    pub remaining_quantity: Quantity,

    /// Order type.
    pub order_type: OrderType,

    /// Order status.
    pub status: OrderStatus,

    /// Client-provided order ID.
    pub client_order_id: u64,

    /// Slot when the order was placed.
    pub slot_placed: u64,

    /// Timestamp when the order was placed (milliseconds since epoch).
    pub timestamp: u64,
}

impl Order {
    /// Returns the filled quantity.
    #[must_use]
    pub fn filled_quantity(&self) -> Quantity {
        self.original_quantity - self.remaining_quantity
    }

    /// Returns the fill ratio (0.0 to 1.0).
    #[must_use]
    pub fn fill_ratio(&self) -> f64 {
        if self.original_quantity.is_zero() {
            return 0.0;
        }
        self.filled_quantity().value() as f64 / self.original_quantity.value() as f64
    }

    /// Returns true if the order is fully filled.
    #[must_use]
    pub fn is_filled(&self) -> bool {
        self.remaining_quantity.is_zero()
    }

    /// Returns true if the order is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_type_can_rest() {
        assert!(OrderType::Limit.can_rest());
        assert!(OrderType::PostOnly.can_rest());
        assert!(!OrderType::ImmediateOrCancel.can_rest());
        assert!(!OrderType::FillOrKill.can_rest());
    }

    #[test]
    fn test_order_type_is_immediate() {
        assert!(!OrderType::Limit.is_immediate());
        assert!(!OrderType::PostOnly.is_immediate());
        assert!(OrderType::ImmediateOrCancel.is_immediate());
        assert!(OrderType::FillOrKill.is_immediate());
    }

    #[test]
    fn test_order_type_display() {
        assert_eq!(OrderType::Limit.to_string(), "limit");
        assert_eq!(OrderType::PostOnly.to_string(), "post_only");
        assert_eq!(OrderType::ImmediateOrCancel.to_string(), "ioc");
        assert_eq!(OrderType::FillOrKill.to_string(), "fok");
    }

    #[test]
    fn test_order_type_from_u8() {
        assert_eq!(OrderType::try_from(0u8), Ok(OrderType::Limit));
        assert_eq!(OrderType::try_from(1u8), Ok(OrderType::PostOnly));
        assert_eq!(OrderType::try_from(2u8), Ok(OrderType::ImmediateOrCancel));
        assert_eq!(OrderType::try_from(3u8), Ok(OrderType::FillOrKill));
        assert!(OrderType::try_from(4u8).is_err());
    }

    #[test]
    fn test_time_in_force_to_order_type() {
        assert_eq!(
            TimeInForce::GoodTilCancelled.to_order_type(),
            OrderType::Limit
        );
        assert_eq!(
            TimeInForce::ImmediateOrCancel.to_order_type(),
            OrderType::ImmediateOrCancel
        );
        assert_eq!(
            TimeInForce::FillOrKill.to_order_type(),
            OrderType::FillOrKill
        );
        assert_eq!(TimeInForce::PostOnly.to_order_type(), OrderType::PostOnly);
    }

    #[test]
    fn test_time_in_force_display() {
        assert_eq!(TimeInForce::GoodTilCancelled.to_string(), "gtc");
        assert_eq!(TimeInForce::ImmediateOrCancel.to_string(), "ioc");
    }

    #[test]
    fn test_order_status_is_active() {
        assert!(OrderStatus::Open.is_active());
        assert!(OrderStatus::PartiallyFilled.is_active());
        assert!(!OrderStatus::Filled.is_active());
        assert!(!OrderStatus::Cancelled.is_active());
        assert!(!OrderStatus::Expired.is_active());
    }

    #[test]
    fn test_order_status_is_terminal() {
        assert!(!OrderStatus::Open.is_terminal());
        assert!(!OrderStatus::PartiallyFilled.is_terminal());
        assert!(OrderStatus::Filled.is_terminal());
        assert!(OrderStatus::Cancelled.is_terminal());
        assert!(OrderStatus::Expired.is_terminal());
    }

    #[test]
    fn test_order_status_display() {
        assert_eq!(OrderStatus::Open.to_string(), "open");
        assert_eq!(OrderStatus::PartiallyFilled.to_string(), "partially_filled");
    }

    #[test]
    fn test_self_trade_behavior_default() {
        assert_eq!(SelfTradeBehavior::default(), SelfTradeBehavior::CancelTaker);
    }

    #[test]
    fn test_place_order_params_new() {
        let params = PlaceOrderParams::new(
            Side::Bid,
            Price::new(1000),
            Quantity::new(100),
            OrderType::Limit,
        );

        assert_eq!(params.side, Side::Bid);
        assert_eq!(params.price.value(), 1000);
        assert_eq!(params.quantity.value(), 100);
        assert_eq!(params.order_type, OrderType::Limit);
        assert_eq!(params.client_order_id, 0);
    }

    #[test]
    fn test_place_order_params_builder() {
        let params = PlaceOrderParams::new(
            Side::Ask,
            Price::new(2000),
            Quantity::new(50),
            OrderType::PostOnly,
        )
        .with_client_order_id(12345)
        .with_self_trade_behavior(SelfTradeBehavior::CancelMaker)
        .with_expiry(100_000);

        assert_eq!(params.client_order_id, 12345);
        assert_eq!(params.self_trade_behavior, SelfTradeBehavior::CancelMaker);
        assert_eq!(params.expiry_slot, Some(100_000));
    }

    #[test]
    fn test_order_filled_quantity() {
        let order = Order {
            order_id: "123".to_string(),
            owner: "owner".to_string(),
            market: "market".to_string(),
            side: Side::Bid,
            price: Price::new(1000),
            original_quantity: Quantity::new(100),
            remaining_quantity: Quantity::new(40),
            order_type: OrderType::Limit,
            status: OrderStatus::PartiallyFilled,
            client_order_id: 0,
            slot_placed: 0,
            timestamp: 0,
        };

        assert_eq!(order.filled_quantity().value(), 60);
    }

    #[test]
    fn test_order_fill_ratio() {
        let order = Order {
            order_id: "123".to_string(),
            owner: "owner".to_string(),
            market: "market".to_string(),
            side: Side::Bid,
            price: Price::new(1000),
            original_quantity: Quantity::new(100),
            remaining_quantity: Quantity::new(25),
            order_type: OrderType::Limit,
            status: OrderStatus::PartiallyFilled,
            client_order_id: 0,
            slot_placed: 0,
            timestamp: 0,
        };

        assert!((order.fill_ratio() - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_order_is_filled() {
        let mut order = Order {
            order_id: "123".to_string(),
            owner: "owner".to_string(),
            market: "market".to_string(),
            side: Side::Bid,
            price: Price::new(1000),
            original_quantity: Quantity::new(100),
            remaining_quantity: Quantity::new(0),
            order_type: OrderType::Limit,
            status: OrderStatus::Filled,
            client_order_id: 0,
            slot_placed: 0,
            timestamp: 0,
        };

        assert!(order.is_filled());

        order.remaining_quantity = Quantity::new(10);
        assert!(!order.is_filled());
    }

    #[test]
    fn test_order_type_serde() {
        let ot = OrderType::PostOnly;
        let json = serde_json::to_string(&ot).expect("serialize");
        assert_eq!(json, "\"post_only\"");

        let parsed: OrderType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, ot);
    }

    #[test]
    fn test_order_status_serde() {
        let status = OrderStatus::PartiallyFilled;
        let json = serde_json::to_string(&status).expect("serialize");
        assert_eq!(json, "\"partially_filled\"");

        let parsed: OrderStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, status);
    }
}
