use anchor_lang::prelude::*;

use crate::error::AmmError;
use crate::utils::cpmm::fee_owed;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub seed: u64,
    pub authority: Option<Pubkey>,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub fee: u16,
    pub locked: bool,
    pub config_bump: u8,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub total_liquidity: u64,
    pub fee_growth_a: u128,
    pub fee_growth_b: u128,
}

impl Config {
    pub fn require_authority(&self, signer: &Pubkey) -> Result<()> {
        let authority: Pubkey = self.authority.ok_or(error!(AmmError::NoAuthority))?;
        require_keys_eq!(authority, *signer, AmmError::Unauthorized);
        Ok(())
    }
}

#[account]
#[derive(InitSpace)]
pub struct Position {
    pub owner: Pubkey,
    pub config: Pubkey,
    pub liquidity: u64,
    pub fee_growth_snapshot_a: u128,
    pub fee_growth_snapshot_b: u128,
    pub fee_owed_a: u64,
    pub fee_owed_b: u64,
    pub bump: u8,
}

impl Position {
    pub fn settle(&mut self, growth_a: u128, growth_b: u128) -> Result<()> {
        let pending_a = fee_owed(self.liquidity, growth_a, self.fee_growth_snapshot_a)
            .map_err(AmmError::from)?;
        let pending_b = fee_owed(self.liquidity, growth_b, self.fee_growth_snapshot_b)
            .map_err(AmmError::from)?;
        self.fee_owed_a = self
            .fee_owed_a
            .checked_add(pending_a)
            .ok_or(error!(AmmError::MathOverflow))?;
        self.fee_owed_b = self
            .fee_owed_b
            .checked_add(pending_b)
            .ok_or(error!(AmmError::MathOverflow))?;
        self.fee_growth_snapshot_a = growth_a;
        self.fee_growth_snapshot_b = growth_b;
        Ok(())
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    A,
    B,
}
