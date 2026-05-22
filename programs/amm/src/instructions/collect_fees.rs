use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{
    constants::{CONFIG_SEED, POSITION_SEED},
    error::AmmError,
    state::{Config, Position, Side},
};

#[derive(Accounts)]
pub struct CollectFees<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        constraint = mint_a.key() < mint_b.key() @ AmmError::InvalidMintOrder,
        mint::token_program = token_program
    )]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mint::token_program = token_program
    )]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub vault_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        has_one = mint_a,
        has_one = mint_b,
        seeds = [CONFIG_SEED, config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Box<Account<'info, Config>>,

    #[account(
        mut,
        seeds = [POSITION_SEED, config.key().as_ref(), user.key().as_ref()],
        bump = position.bump,
        constraint = position.owner == user.key() @ AmmError::Unauthorized,
    )]
    pub position: Box<Account<'info, Position>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_b: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> CollectFees<'info> {
    pub fn collect_fees(&mut self) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);

        self.position
            .settle(self.config.fee_growth_a, self.config.fee_growth_b)?;

        let fee_a = self.position.fee_owed_a;
        let fee_b = self.position.fee_owed_b;

        require!(fee_a > 0 || fee_b > 0, AmmError::InvalidAmount);

        self.position.fee_owed_a = 0;
        self.position.fee_owed_b = 0;

        self.payout(Side::A, fee_a)?;
        self.payout(Side::B, fee_b)
    }

    fn payout(&self, side: Side, amount: u64) -> Result<()> {
        if amount == 0 {
            return Ok(());
        }

        let (from, to, mint, decimals) = match side {
            Side::A => (
                self.vault_a.to_account_info(),
                self.user_a.to_account_info(),
                self.mint_a.to_account_info(),
                self.mint_a.decimals,
            ),
            Side::B => (
                self.vault_b.to_account_info(),
                self.user_b.to_account_info(),
                self.mint_b.to_account_info(),
                self.mint_b.decimals,
            ),
        };

        let signer_seeds: &[&[&[u8]]] = &[&[
            CONFIG_SEED,
            &self.config.seed.to_le_bytes(),
            &[self.config.config_bump],
        ]];

        let cpi_accounts = TransferChecked {
            from,
            to,
            authority: self.config.to_account_info(),
            mint,
        };

        let cpi_ctx =
            CpiContext::new_with_signer(self.token_program.key(), cpi_accounts, signer_seeds);

        transfer_checked(cpi_ctx, amount, decimals)
    }
}
