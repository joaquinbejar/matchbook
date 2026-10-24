//! Program errors.
//!
//! Engine errors map to custom codes `0..100` (their discriminant); program
//! errors start at 100.

use matchbook_engine::EngineError;
use pinocchio::error::ProgramError;

/// Errors raised by the program itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MatchbookError {
    /// Unknown instruction discriminator.
    InvalidInstruction = 100,
    /// Instruction data too short or malformed.
    InvalidInstructionData,
    /// An account is not the one the market expects.
    InvalidAccount,
    /// A required signature is missing.
    MissingSignature,
    /// Market account is not owned by this program.
    InvalidMarketOwner,
    /// Base and quote mints are equal.
    SameMints,
    /// Mint is not owned by the given token program.
    InvalidMint,
    /// Swap output below the minimum.
    SlippageExceeded,
    /// The vault received less than the amount owed (transfer fees are not
    /// supported on the input side of a swap).
    TransferAmountMismatch,
    /// Only the market authority may do this.
    NotAuthority,
    /// Too many accounts or orders in one instruction.
    TooManyItems,
}

impl From<MatchbookError> for ProgramError {
    fn from(e: MatchbookError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

/// Converts an engine error into a program error.
pub fn engine(e: EngineError) -> ProgramError {
    ProgramError::Custom(e as u32)
}
