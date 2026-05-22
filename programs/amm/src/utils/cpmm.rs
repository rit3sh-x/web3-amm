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

const FEE_GROWTH_SCALE: u128 = 1u128 << 64;

pub fn narrow_u64(n: u128) -> Result<u64, CurveError> {
    n.try_into().map_err(|_| CurveError::Overflow)
}

pub fn fee_growth_delta(fee: u64, total_liquidity: u64) -> Result<u128, CurveError> {
    if total_liquidity == 0 {
        return Err(CurveError::ZeroBalance);
    }
    (fee as u128)
        .checked_mul(FEE_GROWTH_SCALE)
        .ok_or(CurveError::Overflow)?
        .checked_div(total_liquidity as u128)
        .ok_or(CurveError::Overflow)
}

pub fn fee_owed(liquidity: u64, growth: u128, snapshot: u128) -> Result<u64, CurveError> {
    let diff = growth.wrapping_sub(snapshot);
    let product = (liquidity as u128)
        .checked_mul(diff)
        .ok_or(CurveError::Overflow)?;
    narrow_u64(product.checked_div(FEE_GROWTH_SCALE).unwrap())
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
        let deposit_x = narrow_u64(
            (a as u128)
                .checked_mul(ratio)
                .ok_or(CurveError::Overflow)?
                .checked_div(precision as u128)
                .ok_or(CurveError::Overflow)?
                .checked_sub(a as u128)
                .ok_or(CurveError::Overflow)?,
        )?;
        let deposit_y = narrow_u64(
            (b as u128)
                .checked_mul(ratio)
                .ok_or(CurveError::Overflow)?
                .checked_div(precision as u128)
                .ok_or(CurveError::Overflow)?
                .checked_sub(b as u128)
                .ok_or(CurveError::Overflow)?,
        )?;
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

        let withdraw_x = narrow_u64(
            (a as u128)
                .checked_sub(
                    (a as u128)
                        .checked_mul(ratio)
                        .ok_or(CurveError::Overflow)?
                        .checked_div(precision as u128)
                        .ok_or(CurveError::Overflow)?,
                )
                .ok_or(CurveError::Overflow)?,
        )?;

        let withdraw_y = narrow_u64(
            (b as u128)
                .checked_sub(
                    (b as u128)
                        .checked_mul(ratio)
                        .ok_or(CurveError::Overflow)?
                        .checked_div(precision as u128)
                        .ok_or(CurveError::Overflow)?,
                )
                .ok_or(CurveError::Overflow)?,
        )?;

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
        let after_fee = narrow_u64(
            (amount as u128)
                .checked_mul(fee_factor)
                .ok_or(CurveError::Overflow)?
                .checked_div(MAX_FEE_BASIS_POINTS as u128)
                .ok_or(CurveError::Overflow)?,
        )?;

        let (new_a, new_b, withdraw) = match pair {
            LiquidityPair::X => {
                let new_a = self.a.checked_add(after_fee).ok_or(CurveError::Overflow)?;
                let delta_b = Self::amount_out(self.a, self.b, after_fee)?;
                let new_b = self.b.checked_sub(delta_b).ok_or(CurveError::Underflow)?;
                (new_a, new_b, delta_b)
            }
            LiquidityPair::Y => {
                let new_b = self.b.checked_add(after_fee).ok_or(CurveError::Overflow)?;
                let delta_a = Self::amount_out(self.b, self.a, after_fee)?;
                let new_a = self.a.checked_sub(delta_a).ok_or(CurveError::Underflow)?;
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

    fn amount_out(in_side: u64, out_side: u64, amount_in: u64) -> Result<u64, CurveError> {
        assert_non_zero!([in_side, out_side]);
        let new_in = (in_side as u128)
            .checked_add(amount_in as u128)
            .ok_or(CurveError::Overflow)?;
        let numerator = (amount_in as u128)
            .checked_mul(out_side as u128)
            .ok_or(CurveError::Overflow)?;
        narrow_u64(numerator.checked_div(new_in).ok_or(CurveError::Overflow)?)
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
    fn swap_never_decreases_k_under_truncation() {
        let mut c = CMM::initialize_cmm(100, 100, 0).unwrap();

        let k_before = (c.a as u128).checked_mul(c.b as u128).unwrap();

        let res = c.swap(LiquidityPair::X, 3, 1).unwrap();

        let k_after = (c.a as u128).checked_mul(c.b as u128).unwrap();

        assert!(
            k_after >= k_before,
            "k must never decrease: before={k_before} after={k_after}"
        );

        assert_eq!(res.withdraw, 2, "amount_out must round down (V2 formula)");
    }

    #[test]
    fn swap_y_never_decreases_k_under_truncation() {
        let mut c = CMM::initialize_cmm(100, 100, 0).unwrap();

        let k_before = (c.a as u128).checked_mul(c.b as u128).unwrap();

        c.swap(LiquidityPair::Y, 7, 1).unwrap();

        let k_after = (c.a as u128).checked_mul(c.b as u128).unwrap();

        assert!(k_after >= k_before);
    }

    #[test]
    fn deposit_rejects_u64_truncation() {
        let err = CMM::deposit(u64::MAX, u64::MAX, 1, u64::MAX, 1_000_000).unwrap_err();
        assert!(matches!(err, CurveError::Overflow));
    }

    #[test]
    fn init_rejects_excessive_fee() {
        assert!(matches!(
            CMM::initialize_cmm(10, 10, MAX_FEE_BASIS_POINTS),
            Err(CurveError::InvalidFeeAmount)
        ));
    }
}
