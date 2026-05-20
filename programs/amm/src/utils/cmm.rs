use std::error::Error;
use std::fmt;

use anchor_spl::token_2022::spl_token_2022::extension::transfer_fee::MAX_FEE_BASIS_POINTS;

macro_rules! assert_non_zero {
    ($array:expr) => {
        if $array.contains(&0u64) {
            return Err(CurveError::ZeroBalance);
        }
    };
}

macro_rules! swap_slippage {
    ($x:expr, $x_min:expr) => {
        if $x < $x_min {
            return Err(CurveError::SlippageLimitExceeded);
        }
    };
}

#[derive(Debug)]
pub enum LiquidityPair {
    X,
    Y,
}

#[derive(Debug)]
pub struct XYAmounts {
    pub x: u64,
    pub y: u64,
}

#[derive(Debug)]
pub struct SwapResult {
    pub deposit: u64,
    pub withdraw: u64,
    pub fee: u64,
}

#[derive(Debug)]
pub enum CurveError {
    InvalidPrecision,
    Overflow,
    Underflow,
    InvalidFeeAmount,
    InsufficientBalance,
    ZeroBalance,
    SlippageLimitExceeded,
}

impl Error for CurveError {}

impl fmt::Display for CurveError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub struct CMM {
    pub a: u64,
    pub b: u64,
    pub fee: u16,
}

impl CMM {
    pub fn initialize_cmm(a: u64, b: u64, fee: u16) -> Result<CMM, CurveError> {
        assert_non_zero!([a, b]);
        if fee >= MAX_FEE_BASIS_POINTS {
            return Err(CurveError::InvalidFeeAmount);
        }
        Ok(CMM { a, b, fee })
    }

    pub fn deposit(
        a: u64,
        b: u64,
        lp: u64,
        amount: u64,
        precision: u32,
    ) -> Result<XYAmounts, CurveError> {
        if lp == 0 {
            return Err(CurveError::ZeroBalance);
        }
        let ratio = (lp as u128)
            .checked_add(amount as u128)
            .ok_or(CurveError::Overflow)?
            .checked_mul(precision as u128)
            .ok_or(CurveError::Overflow)?
            .checked_div(lp as u128)
            .ok_or(CurveError::Overflow)?;
        let deposit_x = (a as u128)
            .checked_mul(ratio)
            .ok_or(CurveError::Overflow)?
            .checked_div(precision as u128)
            .ok_or(CurveError::Overflow)?
            .checked_sub(a as u128)
            .ok_or(CurveError::Overflow)? as u64;
        let deposit_y = (b as u128)
            .checked_mul(ratio)
            .ok_or(CurveError::Overflow)?
            .checked_div(precision as u128)
            .ok_or(CurveError::Overflow)?
            .checked_sub(b as u128)
            .ok_or(CurveError::Overflow)? as u64;
        Ok(XYAmounts {
            x: deposit_x,
            y: deposit_y,
        })
    }

    pub fn withdraw(
        a: u64,
        b: u64,
        lp: u64,
        amount: u64,
        precision: u32,
    ) -> Result<XYAmounts, CurveError> {
        if lp == 0 {
            return Err(CurveError::ZeroBalance);
        }
        let ratio = (lp
            .checked_sub(amount)
            .ok_or(CurveError::InsufficientBalance)? as u128)
            .checked_mul(precision as u128)
            .ok_or(CurveError::Overflow)?
            .checked_div(lp as u128)
            .ok_or(CurveError::Overflow)?;

        let withdraw_x = (a as u128)
            .checked_sub(
                (a as u128)
                    .checked_mul(ratio)
                    .ok_or(CurveError::Overflow)?
                    .checked_div(precision as u128)
                    .ok_or(CurveError::Overflow)?,
            )
            .ok_or(CurveError::Overflow)? as u64;

        let withdraw_y = (b as u128)
            .checked_sub(
                (b as u128)
                    .checked_mul(ratio)
                    .ok_or(CurveError::Overflow)?
                    .checked_div(precision as u128)
                    .ok_or(CurveError::Overflow)?,
            )
            .ok_or(CurveError::Overflow)? as u64;

        Ok(XYAmounts {
            x: withdraw_x,
            y: withdraw_y,
        })
    }

    pub fn swap(
        &mut self,
        pair: LiquidityPair,
        amount: u64,
        min: u64,
    ) -> Result<SwapResult, CurveError> {
        let fee_factor = (MAX_FEE_BASIS_POINTS as u128)
            .checked_sub(self.fee as u128)
            .ok_or(CurveError::Underflow)?;
        let after_fee = (amount as u128)
            .checked_mul(fee_factor)
            .ok_or(CurveError::Overflow)?
            .checked_div(MAX_FEE_BASIS_POINTS as u128)
            .ok_or(CurveError::Overflow)? as u64;

        let (new_a, new_b, withdraw) = match pair {
            LiquidityPair::X => {
                let new_a = self.a.checked_add(after_fee).ok_or(CurveError::Overflow)?;
                let new_b = Self::other_side(self.a, self.b, after_fee)?;
                let delta_b = self.b.checked_sub(new_b).ok_or(CurveError::Overflow)?;
                (new_a, new_b, delta_b)
            }
            LiquidityPair::Y => {
                let new_b = self.b.checked_add(after_fee).ok_or(CurveError::Overflow)?;
                let new_a = Self::other_side(self.b, self.a, after_fee)?;
                let delta_a = self.a.checked_sub(new_a).ok_or(CurveError::Overflow)?;
                (new_a, new_b, delta_a)
            }
        };

        swap_slippage!(withdraw, min);
        let fee = amount.checked_sub(after_fee).ok_or(CurveError::Underflow)?;

        self.a = new_a;
        self.b = new_b;

        Ok(SwapResult {
            deposit: amount,
            fee,
            withdraw,
        })
    }

    fn other_side(in_side: u64, out_side: u64, amount: u64) -> Result<u64, CurveError> {
        let k = Self::k(in_side, out_side)?;
        let new_in = (in_side as u128)
            .checked_add(amount as u128)
            .ok_or(CurveError::Overflow)?;
        Ok(k.checked_div(new_in).ok_or(CurveError::Overflow)? as u64)
    }

    fn k(a: u64, b: u64) -> Result<u128, CurveError> {
        assert_non_zero!([a, b]);
        (a as u128)
            .checked_mul(b as u128)
            .ok_or(CurveError::Overflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_x_preserves_k_no_fee() {
        let mut c = CMM::initialize_cmm(20, 30, 0).unwrap();
        let res = c.swap(LiquidityPair::X, 5, 1).unwrap();
        assert_eq!(res.deposit, 5);
        assert_eq!(res.withdraw, 6);
        assert_eq!(res.fee, 0);
        assert_eq!(c.a, 25);
        assert_eq!(c.b, 24);
    }

    #[test]
    fn swap_y_preserves_k_no_fee() {
        let mut c = CMM::initialize_cmm(30, 20, 0).unwrap();
        let res = c.swap(LiquidityPair::Y, 5, 1).unwrap();
        assert_eq!(res.deposit, 5);
        assert_eq!(res.withdraw, 6);
        assert_eq!(c.a, 24);
        assert_eq!(c.b, 25);
    }

    #[test]
    fn swap_with_fee_charges_lp() {
        let mut c = CMM::initialize_cmm(20, 30, 100).unwrap();
        let res = c.swap(LiquidityPair::X, 5, 1).unwrap();
        assert_eq!(res.deposit, 5);
        assert_eq!(res.fee, 1);
        assert_eq!(res.withdraw, 5);
        assert_eq!(c.a, 24);
        assert_eq!(c.b, 25);
    }

    #[test]
    fn swap_min_out_enforced() {
        let mut c = CMM::initialize_cmm(20, 30, 0).unwrap();
        let err = c.swap(LiquidityPair::X, 5, 7).unwrap_err();
        assert!(matches!(err, CurveError::SlippageLimitExceeded));
    }

    #[test]
    fn deposit_amounts_proportional() {
        let r = CMM::deposit(30, 30, 30, 30, 1_000_000).unwrap();
        assert_eq!(r.x, 30);
        assert_eq!(r.y, 30);
    }

    #[test]
    fn withdraw_amounts_proportional() {
        let r = CMM::withdraw(60, 60, 30, 15, 1_000_000).unwrap();
        assert_eq!(r.x, 30);
        assert_eq!(r.y, 30);
    }

    #[test]
    fn init_rejects_zero_side() {
        assert!(matches!(
            CMM::initialize_cmm(0, 10, 0),
            Err(CurveError::ZeroBalance)
        ));
        assert!(matches!(
            CMM::initialize_cmm(10, 0, 0),
            Err(CurveError::ZeroBalance)
        ));
    }

    #[test]
    fn init_rejects_excessive_fee() {
        assert!(matches!(
            CMM::initialize_cmm(10, 10, MAX_FEE_BASIS_POINTS),
            Err(CurveError::InvalidFeeAmount)
        ));
    }
}
