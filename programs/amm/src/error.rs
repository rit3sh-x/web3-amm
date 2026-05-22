use crate::utils::cpmm::CurveError;
use anchor_lang::prelude::*;

#[error_code]
#[derive(Eq, PartialEq)]
pub enum AmmError {
    #[msg("Pool is locked: deposits, withdrawals, and swaps are disabled")]
    PoolLocked,
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Fee must be lower than 10000 basis points (100%)")]
    InvalidFee,
    #[msg("mint_a must be strictly less than mint_b (and the two must differ)")]
    InvalidMintOrder,
    #[msg("Signer is not the configured pool authority")]
    Unauthorized,
    #[msg("This pool has no authority and cannot be locked or unlocked")]
    NoAuthority,
    #[msg("Slippage tolerance exceeded: result is outside the requested min/max bounds")]
    SlippageExceeded,
    #[msg("The pool has no liquidity for this operation")]
    NoLiquidity,
    #[msg("Insufficient balance for the requested operation")]
    InsufficientBalance,
    #[msg("Curve precision is invalid")]
    InvalidPrecision,
    #[msg("Arithmetic overflow or underflow during curve calculation")]
    MathOverflow,
    #[msg("Mint carries a Token-2022 extension this AMM does not support (transfer fee, transfer hook, non-transferable, default-frozen, permanent delegate, or confidential transfer)")]
    UnsupportedMintExtension,
}

impl From<CurveError> for AmmError {
    fn from(err: CurveError) -> Self {
        match err {
            CurveError::InvalidPrecision => AmmError::InvalidPrecision,
            CurveError::Overflow => AmmError::MathOverflow,
            CurveError::Underflow => AmmError::MathOverflow,
            CurveError::InvalidFeeAmount => AmmError::InvalidFee,
            CurveError::InsufficientBalance => AmmError::InsufficientBalance,
            CurveError::ZeroBalance => AmmError::NoLiquidity,
            CurveError::SlippageLimitExceeded => AmmError::SlippageExceeded,
        }
    }
}
