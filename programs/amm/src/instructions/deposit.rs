use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::{
    constants::{CONFIG_SEED, POSITION_SEED, PRECISION},
    error::AmmError,
    state::{Config, Position, Side},
    utils::cpmm::CMM,
};

#[derive(Accounts)]
pub struct Deposit<'info> {
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
        mut,
        has_one = mint_a,
        has_one = mint_b,
        seeds = [CONFIG_SEED, config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Box<Account<'info, Config>>,

    #[account(
        init_if_needed,
        payer = user,
        space = Position::DISCRIMINATOR.len() + Position::INIT_SPACE,
        seeds = [POSITION_SEED, config.key().as_ref(), user.key().as_ref()],
        bump,
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

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    pub fn deposit(
        &mut self,
        amount: u64,
        max_a: u64,
        max_b: u64,
        bumps: &DepositBumps,
    ) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);
        require!(amount > 0, AmmError::InvalidAmount);

        if self.position.liquidity > 0 {
            require_keys_eq!(self.position.owner, self.user.key(), AmmError::Unauthorized);
            self.position
                .settle(self.config.fee_growth_a, self.config.fee_growth_b)?;
        } else {
            self.position.owner = self.user.key();
            self.position.config = self.config.key();
            self.position.bump = bumps.position;
            self.position.fee_growth_snapshot_a = self.config.fee_growth_a;
            self.position.fee_growth_snapshot_b = self.config.fee_growth_b;
        }

        let (x, y) = if self.config.total_liquidity == 0 {
            require!(max_a > 0 && max_b > 0, AmmError::InvalidAmount);
            (max_a, max_b)
        } else {
            let amounts = CMM::deposit(
                self.config.reserve_a,
                self.config.reserve_b,
                self.config.total_liquidity,
                amount,
                PRECISION,
            )
            .map_err(AmmError::from)?;

            require!(
                amounts.x <= max_a && amounts.y <= max_b,
                AmmError::SlippageExceeded
            );

            (amounts.x, amounts.y)
        };

        self.deposit_tokens(Side::A, x)?;
        self.deposit_tokens(Side::B, y)?;

        self.config.reserve_a = self
            .config
            .reserve_a
            .checked_add(x)
            .ok_or(error!(AmmError::MathOverflow))?;
        self.config.reserve_b = self
            .config
            .reserve_b
            .checked_add(y)
            .ok_or(error!(AmmError::MathOverflow))?;
        self.config.total_liquidity = self
            .config
            .total_liquidity
            .checked_add(amount)
            .ok_or(error!(AmmError::MathOverflow))?;
        self.position.liquidity = self
            .position
            .liquidity
            .checked_add(amount)
            .ok_or(error!(AmmError::MathOverflow))?;

        Ok(())
    }

    fn deposit_tokens(&self, side: Side, amount: u64) -> Result<()> {
        let (from, to, mint, decimals) = match side {
            Side::A => (
                self.user_a.to_account_info(),
                self.vault_a.to_account_info(),
                self.mint_a.to_account_info(),
                self.mint_a.decimals,
            ),
            Side::B => (
                self.user_b.to_account_info(),
                self.vault_b.to_account_info(),
                self.mint_b.to_account_info(),
                self.mint_b.decimals,
            ),
        };

        let cpi_accounts = TransferChecked {
            from,
            to,
            authority: self.user.to_account_info(),
            mint,
        };

        let cpi_ctx = CpiContext::new(self.token_program.key(), cpi_accounts);

        transfer_checked(cpi_ctx, amount, decimals)
    }
}
