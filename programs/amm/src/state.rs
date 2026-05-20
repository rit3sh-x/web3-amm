use anchor_lang::prelude::*;

use crate::error::AmmError;

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
    pub lp_bump: u8,
}

impl Config {
    pub fn require_authority(&self, signer: &Pubkey) -> Result<()> {
        let authority: Pubkey = self.authority.ok_or(error!(AmmError::NoAuthority))?;
        require_keys_eq!(authority, *signer, AmmError::Unauthorized);
        Ok(())
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    A,
    B,
}
