//! Account discriminator constants.
//!
//! Anchor uses 8-byte discriminators derived from the account type name
//! using SHA256. These constants identify account types.

/// Account discriminator enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Discriminator {
    /// Market account discriminator.
    Market,
    /// OrderBookSide account discriminator.
    OrderBookSide,
    /// EventQueue account discriminator.
    EventQueue,
    /// OpenOrders account discriminator.
    OpenOrders,
}

/// Market account discriminator bytes.
///
/// Derived from: `sha256("account:Market")[0..8]`
pub const MARKET_DISCRIMINATOR: [u8; 8] = [219, 190, 213, 55, 0, 227, 198, 154];

/// OrderBookSide account discriminator bytes.
///
/// Derived from: `sha256("account:OrderBookSideHeader")[0..8]`
pub const ORDERBOOK_SIDE_DISCRIMINATOR: [u8; 8] = [139, 166, 126, 82, 145, 115, 116, 152];

/// EventQueue account discriminator bytes.
///
/// Derived from: `sha256("account:EventQueueHeader")[0..8]`
pub const EVENT_QUEUE_DISCRIMINATOR: [u8; 8] = [164, 207, 200, 51, 199, 113, 35, 109];

/// OpenOrders account discriminator bytes.
///
/// Derived from: `sha256("account:OpenOrders")[0..8]`
pub const OPEN_ORDERS_DISCRIMINATOR: [u8; 8] = [139, 82, 72, 83, 211, 18, 186, 37];

impl Discriminator {
    /// Returns the discriminator bytes for this type.
    #[must_use]
    pub const fn as_bytes(&self) -> &'static [u8; 8] {
        match self {
            Self::Market => &MARKET_DISCRIMINATOR,
            Self::OrderBookSide => &ORDERBOOK_SIDE_DISCRIMINATOR,
            Self::EventQueue => &EVENT_QUEUE_DISCRIMINATOR,
            Self::OpenOrders => &OPEN_ORDERS_DISCRIMINATOR,
        }
    }

    /// Attempts to identify a discriminator from bytes.
    #[must_use]
    pub fn from_bytes(bytes: &[u8; 8]) -> Option<Self> {
        if bytes == &MARKET_DISCRIMINATOR {
            Some(Self::Market)
        } else if bytes == &ORDERBOOK_SIDE_DISCRIMINATOR {
            Some(Self::OrderBookSide)
        } else if bytes == &EVENT_QUEUE_DISCRIMINATOR {
            Some(Self::EventQueue)
        } else if bytes == &OPEN_ORDERS_DISCRIMINATOR {
            Some(Self::OpenOrders)
        } else {
            None
        }
    }

    /// Returns a human-readable name for the discriminator.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Market => "Market",
            Self::OrderBookSide => "OrderBookSide",
            Self::EventQueue => "EventQueue",
            Self::OpenOrders => "OpenOrders",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discriminator_as_bytes() {
        assert_eq!(Discriminator::Market.as_bytes(), &MARKET_DISCRIMINATOR);
        assert_eq!(
            Discriminator::OrderBookSide.as_bytes(),
            &ORDERBOOK_SIDE_DISCRIMINATOR
        );
        assert_eq!(
            Discriminator::EventQueue.as_bytes(),
            &EVENT_QUEUE_DISCRIMINATOR
        );
        assert_eq!(
            Discriminator::OpenOrders.as_bytes(),
            &OPEN_ORDERS_DISCRIMINATOR
        );
    }

    #[test]
    fn test_discriminator_from_bytes() {
        assert_eq!(
            Discriminator::from_bytes(&MARKET_DISCRIMINATOR),
            Some(Discriminator::Market)
        );
        assert_eq!(
            Discriminator::from_bytes(&ORDERBOOK_SIDE_DISCRIMINATOR),
            Some(Discriminator::OrderBookSide)
        );
        assert_eq!(
            Discriminator::from_bytes(&EVENT_QUEUE_DISCRIMINATOR),
            Some(Discriminator::EventQueue)
        );
        assert_eq!(
            Discriminator::from_bytes(&OPEN_ORDERS_DISCRIMINATOR),
            Some(Discriminator::OpenOrders)
        );
    }

    #[test]
    fn test_discriminator_from_bytes_unknown() {
        let unknown = [0xFF; 8];
        assert_eq!(Discriminator::from_bytes(&unknown), None);
    }

    #[test]
    fn test_discriminator_name() {
        assert_eq!(Discriminator::Market.name(), "Market");
        assert_eq!(Discriminator::OrderBookSide.name(), "OrderBookSide");
        assert_eq!(Discriminator::EventQueue.name(), "EventQueue");
        assert_eq!(Discriminator::OpenOrders.name(), "OpenOrders");
    }

    #[test]
    fn test_discriminator_equality() {
        assert_eq!(Discriminator::Market, Discriminator::Market);
        assert_ne!(Discriminator::Market, Discriminator::OpenOrders);
    }

    #[test]
    fn test_discriminators_are_unique() {
        let discriminators = [
            MARKET_DISCRIMINATOR,
            ORDERBOOK_SIDE_DISCRIMINATOR,
            EVENT_QUEUE_DISCRIMINATOR,
            OPEN_ORDERS_DISCRIMINATOR,
        ];

        for i in 0..discriminators.len() {
            for j in (i + 1)..discriminators.len() {
                assert_ne!(
                    discriminators[i], discriminators[j],
                    "Discriminators must be unique"
                );
            }
        }
    }
}
