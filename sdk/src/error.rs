//! SDK error types.
//!
//! Provides error types for SDK operations.

/// SDK errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SdkError {
    /// Invalid price value.
    #[error("invalid price: {0}")]
    InvalidPrice(String),

    /// Invalid quantity value.
    #[error("invalid quantity: {0}")]
    InvalidQuantity(String),

    /// Invalid address.
    #[error("invalid address: {0}")]
    InvalidAddress(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Deserialization error.
    #[error("deserialization error: {0}")]
    Deserialization(String),

    /// Arithmetic overflow.
    #[error("arithmetic overflow")]
    Overflow,

    /// Arithmetic underflow.
    #[error("arithmetic underflow")]
    Underflow,

    /// Division by zero.
    #[error("division by zero")]
    DivisionByZero,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = SdkError::InvalidPrice("negative value".to_string());
        assert_eq!(err.to_string(), "invalid price: negative value");
    }

    #[test]
    fn test_error_overflow() {
        let err = SdkError::Overflow;
        assert_eq!(err.to_string(), "arithmetic overflow");
    }
}
