use anchor_lang::prelude::*;
use anchor_spl::token_2022::spl_token_2022::extension::transfer_fee::MAX_FEE_BASIS_POINTS;

use crate::{constants::CONFIG_SEED, error::AmmError, state::Config};

#[derive(Accounts)]
pub struct Admin<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [CONFIG_SEED, config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,
}

impl<'info> Admin<'info> {
    pub fn set_locked(&mut self, locked: bool) -> Result<()> {
        self.config.require_authority(&self.authority.key())?;

        self.config.locked = locked;

        Ok(())
    }

    pub fn set_fee(&mut self, fee: u16) -> Result<()> {
        self.config.require_authority(&self.authority.key())?;

        require!(fee < MAX_FEE_BASIS_POINTS, AmmError::InvalidFee);

        self.config.fee = fee;

        Ok(())
    }

    pub fn set_authority(&mut self, authority: Option<Pubkey>) -> Result<()> {
        self.config.require_authority(&self.authority.key())?;

        self.config.authority = authority;

        Ok(())
    }
}
