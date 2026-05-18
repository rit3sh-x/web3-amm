use std::error::Error;
use std::fmt;

macro_rules! assert_non_zero {
    ($array:expr) => {
        if $array.contains(&0u64) {
            return Err(CurveError::ZeroBalance);
        }
    };
}

macro_rules! withdraw_slippage {
    ($x:expr, $y:expr, $x_min:expr, $y_min:expr) => {
        if $x < $x_min || $y < $y_min {
            return Err(CurveError::SlippageLimitExceeded);
        }
    };
}

macro_rules! deposit_slippage {
    ($x:expr, $y:expr, $x_max:expr, $y_max:expr) => {
        if $x > $x_max || $y > $y_max {
            return Err(CurveError::SlippageLimitExceeded);
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
pub struct SpotPrice {
    pub amount: u128,
    pub precision: u32,
}

#[derive(Debug)]
pub struct XYAmounts {
    pub x: u64,
    pub y: u64,
}

#[derive(Debug)]
pub struct DepositLiquidityResult {
    pub deposit_x: u64,
    pub deposit_y: u64,
    pub mint_l: u64,
}

#[derive(Debug)]
pub struct WithdrawLiquidityResult {
    pub withdraw_x: u64,
    pub withdraw_y: u64,
    pub burn_l: u64,
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
pub struct ConstantProduct {
    x: u64,
    y: u64,
    l: u64,
    fee: u16,
    precision: u32,
}

impl ConstantProduct {
    pub fn init(
        x: u64,
        y: u64,
        l: u64,
        fee: u16,
        precision: Option<u8>,
    ) -> Result<ConstantProduct, CurveError> {
        assert_non_zero!([x, y]);
        if fee >= 10_000 {
            return Err(CurveError::InvalidFeeAmount);
        }
        let precision = match precision {
            Some(p) => 10u32
                .checked_pow(p as u32)
                .ok_or(CurveError::InvalidPrecision)?,
            None => 1_000_000,
        };

        let l = match l > 0 {
            true => l,
            false => x.max(y),
        };

        Ok(ConstantProduct {
            x,
            y,
            l,
            fee,
            precision,
        })
    }

    pub fn k_from_xy(x: u64, y: u64) -> Result<u128, CurveError> {
        assert_non_zero!([x, y]);
        (x as u128)
            .checked_mul(y as u128)
            .ok_or(CurveError::Overflow)
    }

    pub fn spot_price_from_pair(x: u64, y: u64, precision: u32) -> Result<SpotPrice, CurveError> {
        assert_non_zero!([x, y]);
        Ok(SpotPrice {
            amount: (x as u128)
                .checked_mul(precision as u128)
                .ok_or(CurveError::Overflow)?
                .checked_div(y as u128)
                .ok_or(CurveError::Overflow)?,
            precision,
        })
    }

    pub fn xy_deposit_amounts_from_l(
        x: u64,
        y: u64,
        l: u64,
        a: u64,
        precision: u32,
    ) -> Result<XYAmounts, CurveError> {
        if l == 0 {
            return Err(CurveError::ZeroBalance);
        }
        let ratio = (l as u128)
            .checked_add(a as u128)
            .ok_or(CurveError::Overflow)?
            .checked_mul(precision as u128)
            .ok_or(CurveError::Overflow)?
            .checked_div(l as u128)
            .ok_or(CurveError::Overflow)?;
        let deposit_x = (x as u128)
            .checked_mul(ratio)
            .ok_or(CurveError::Overflow)?
            .checked_div(precision as u128)
            .ok_or(CurveError::Overflow)?
            .checked_sub(x as u128)
            .ok_or(CurveError::Overflow)? as u64;
        let deposit_y = (y as u128)
            .checked_mul(ratio)
            .ok_or(CurveError::Overflow)?
            .checked_div(precision as u128)
            .ok_or(CurveError::Overflow)?
            .checked_sub(y as u128)
            .ok_or(CurveError::Overflow)? as u64;
        Ok(XYAmounts {
            x: deposit_x,
            y: deposit_y,
        })
    }

    pub fn xy_withdraw_amounts_from_l(
        x: u64,
        y: u64,
        l: u64,
        a: u64,
        precision: u32,
    ) -> Result<XYAmounts, CurveError> {
        if l == 0 {
            return Err(CurveError::ZeroBalance);
        }
        let ratio = (l.checked_sub(a).ok_or(CurveError::InsufficientBalance)? as u128)
            .checked_mul(precision as u128)
            .ok_or(CurveError::Overflow)?
            .checked_div(l as u128)
            .ok_or(CurveError::Overflow)?;

        let withdraw_x = (x as u128)
            .checked_sub(
                (x as u128)
                    .checked_mul(ratio)
                    .ok_or(CurveError::Overflow)?
                    .checked_div(precision as u128)
                    .ok_or(CurveError::Overflow)?,
            )
            .ok_or(CurveError::Overflow)? as u64;

        let withdraw_y = (y as u128)
            .checked_sub(
                (y as u128)
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

    pub fn x2_from_y_swap_amount(x: u64, y: u64, a: u64) -> Result<u64, CurveError> {
        let k = Self::k_from_xy(x, y)?;
        let x_new = (y as u128)
            .checked_add(a as u128)
            .ok_or(CurveError::Overflow)?;
        Ok(k.checked_div(x_new).ok_or(CurveError::Overflow)? as u64)
    }

    pub fn y2_from_x_swap_amount(x: u64, y: u64, a: u64) -> Result<u64, CurveError> {
        Self::x2_from_y_swap_amount(y, x, a)
    }

    pub fn delta_x_from_y_swap_amount(x: u64, y: u64, a: u64) -> Result<u64, CurveError> {
        Ok(x.checked_sub(Self::x2_from_y_swap_amount(x, y, a)?)
            .ok_or(CurveError::Overflow)?)
    }

    pub fn delta_y_from_x_swap_amount(x: u64, y: u64, a: u64) -> Result<u64, CurveError> {
        Self::delta_x_from_y_swap_amount(y, x, a)
    }

    pub fn k(&self) -> Result<u128, CurveError> {
        Self::k_from_xy(self.x, self.y)
    }

    pub fn spot_price_x(&self) -> Result<SpotPrice, CurveError> {
        Self::spot_price_from_pair(self.x, self.y, self.precision)
    }

    pub fn spot_price_y(&self) -> Result<SpotPrice, CurveError> {
        Self::spot_price_from_pair(self.y, self.x, self.precision)
    }

    pub fn swap_unsafe(&mut self, p: LiquidityPair, a: u64) -> Result<SwapResult, CurveError> {
        let fee_multiplier = 10_000u64
            .checked_sub(self.fee.into())
            .ok_or(CurveError::Underflow)?;
        let a2 = (a as u128)
            .checked_mul(fee_multiplier as u128)
            .ok_or(CurveError::Overflow)?
            .checked_div(10_000)
            .ok_or(CurveError::Overflow)? as u64;
        let (new_x, new_y, withdraw) = match p {
            LiquidityPair::X => (
                self.x.checked_add(a2).ok_or(CurveError::Overflow)?,
                Self::y2_from_x_swap_amount(self.x, self.y, a2)?,
                Self::delta_y_from_x_swap_amount(self.x, self.y, a2)?,
            ),
            LiquidityPair::Y => (
                Self::x2_from_y_swap_amount(self.x, self.y, a2)?,
                self.y.checked_add(a2).ok_or(CurveError::Overflow)?,
                Self::delta_x_from_y_swap_amount(self.x, self.y, a2)?,
            ),
        };
        let fee = a.checked_sub(a2).ok_or(CurveError::Underflow)?;
        self.x = new_x;
        self.y = new_y;
        Ok(SwapResult {
            deposit: a,
            fee,
            withdraw,
        })
    }

    pub fn swap(&mut self, p: LiquidityPair, a: u64, min: u64) -> Result<SwapResult, CurveError> {
        let fee_factor = (10_000u128)
            .checked_sub(self.fee as u128)
            .ok_or(CurveError::Underflow)?;
        let a2 = (a as u128)
            .checked_mul(fee_factor)
            .ok_or(CurveError::Overflow)?
            .checked_div(10_000u128)
            .ok_or(CurveError::Overflow)? as u64;

        let (new_x, new_y, withdraw) = match p {
            LiquidityPair::X => {
                let x2 = self.x.checked_add(a2).ok_or(CurveError::Overflow)?;
                let y2 = Self::y2_from_x_swap_amount(self.x, self.y, a2)?;
                let delta_y = Self::delta_y_from_x_swap_amount(self.x, self.y, a2)?;
                (x2, y2, delta_y)
            }
            LiquidityPair::Y => {
                let x2 = Self::x2_from_y_swap_amount(self.x, self.y, a2)?;
                let y2 = self.y.checked_add(a2).ok_or(CurveError::Overflow)?;
                let delta_x = Self::delta_x_from_y_swap_amount(self.x, self.y, a2)?;
                (x2, y2, delta_x)
            }
        };

        swap_slippage!(withdraw, min);
        let fee = a.checked_sub(a2).ok_or(CurveError::Underflow)?;
        self.x = new_x;
        self.y = new_y;

        Ok(SwapResult {
            deposit: a,
            fee,
            withdraw,
        })
    }

    pub fn deposit_liquidity_unsafe(
        &mut self,
        x: u64,
        y: u64,
        a: u64,
    ) -> Result<DepositLiquidityResult, CurveError> {
        self.x = self.x.checked_add(x).ok_or(CurveError::Overflow)?;
        self.y = self.y.checked_add(y).ok_or(CurveError::Overflow)?;
        self.l = self.l.checked_add(a).ok_or(CurveError::Overflow)?;
        Ok(DepositLiquidityResult {
            deposit_x: x,
            deposit_y: y,
            mint_l: a,
        })
    }

    pub fn withdraw_liquidity_unsafe(
        &mut self,
        x: u64,
        y: u64,
        a: u64,
    ) -> Result<WithdrawLiquidityResult, CurveError> {
        self.x = self.x.checked_sub(x).ok_or(CurveError::Underflow)?;
        self.y = self.y.checked_sub(y).ok_or(CurveError::Underflow)?;
        self.l = self.l.checked_sub(a).ok_or(CurveError::Underflow)?;
        Ok(WithdrawLiquidityResult {
            withdraw_x: x,
            withdraw_y: y,
            burn_l: a,
        })
    }

    pub fn deposit_liquidity(
        &mut self,
        a: u64,
        max_x: u64,
        max_y: u64,
    ) -> Result<DepositLiquidityResult, CurveError> {
        let xy = Self::xy_deposit_amounts_from_l(self.x, self.y, self.l, a, self.precision)?;
        deposit_slippage!(xy.x, xy.y, max_x, max_y);
        self.deposit_liquidity_unsafe(xy.x, xy.y, a)
    }

    pub fn withdraw_liquidity(
        &mut self,
        a: u64,
        min_x: u64,
        min_y: u64,
    ) -> Result<WithdrawLiquidityResult, CurveError> {
        let xy = Self::xy_withdraw_amounts_from_l(self.x, self.y, self.l, a, self.precision)?;
        withdraw_slippage!(xy.x, xy.y, min_x, min_y);
        self.withdraw_liquidity_unsafe(xy.x, xy.y, a)
    }
}

#[cfg(test)]
mod tests {
    use crate::{ConstantProduct, LiquidityPair};

    #[test]
    fn swap_balance() {
        let mut c = ConstantProduct::init(20, 30, 0, 0, None).unwrap();
        assert_eq!(c.k().unwrap(), 600);
        assert_eq!(c.x, 20);
        assert_eq!(c.y, 30);
        let res = c.swap(LiquidityPair::X, 5, 6).unwrap();
        assert_eq!(c.k().unwrap(), 600);
        assert_eq!(res.deposit, 5);
        assert_eq!(res.withdraw, 6);
        assert_eq!(c.x, 25);
        assert_eq!(c.y, 24);
        let res = c.swap(LiquidityPair::X, 5, 4).unwrap();
        assert_eq!(c.k().unwrap(), 600);
        assert_eq!(res.deposit, 5);
        assert_eq!(res.withdraw, 4);
        assert_eq!(c.x, 30);
        assert_eq!(c.y, 20);
    }

    #[test]
    fn swap_balance_reverse() {
        let mut c = ConstantProduct::init(30, 20, 0, 0, None).unwrap();
        assert_eq!(c.k().unwrap(), 600);
        assert_eq!(c.x, 30);
        assert_eq!(c.y, 20);
        let res = c.swap(LiquidityPair::Y, 5, 6).unwrap();
        assert_eq!(c.k().unwrap(), 600);
        assert_eq!(res.deposit, 5);
        assert_eq!(res.withdraw, 6);
        assert_eq!(c.x, 24);
        assert_eq!(c.y, 25);
        let res = c.swap(LiquidityPair::Y, 5, 4).unwrap();
        assert_eq!(res.deposit, 5);
        assert_eq!(res.withdraw, 4);
        assert_eq!(c.x, 20);
        assert_eq!(c.y, 30);
    }

    #[test]
    fn swap_balance_with_fee() {
        let mut c = ConstantProduct::init(20, 30, 0, 100, None).unwrap();
        assert_eq!(c.k().unwrap(), 600);
        assert_eq!(c.x, 20);
        assert_eq!(c.y, 30);
        let res = c.swap(LiquidityPair::X, 5, 5).unwrap();
        assert_eq!(res.deposit, 5);
        assert_eq!(res.withdraw, 5);
        assert_eq!(res.fee, 1);
        assert_eq!(c.x, 24);
        assert_eq!(c.y, 25);
    }

    #[test]
    fn deposit_liquidity() {
        let mut c = ConstantProduct::init(30, 30, 0, 100, None).unwrap();
        assert_eq!(c.k().unwrap(), 900);
        assert_eq!(c.x, 30);
        assert_eq!(c.y, 30);
        let r = c.deposit_liquidity(30, 10000000, 10000000).unwrap();
        assert_eq!(r.deposit_x, 30);
        assert_eq!(r.deposit_y, 30);
        assert_eq!(r.mint_l, 30);
        let r = c.withdraw_liquidity(30, 0, 0).unwrap();
        assert_eq!(r.withdraw_x, 30);
        assert_eq!(r.withdraw_y, 30);
        assert_eq!(r.burn_l, 30);
    }

    #[test]
    fn spot_price() {
        let c = ConstantProduct::init(10, 10, 0, 100, Some(0)).unwrap();
        assert_eq!(
            c.spot_price_x().unwrap().amount,
            c.spot_price_y().unwrap().amount
        );
        assert_eq!(c.spot_price_x().unwrap().amount, 1)
    }
}
