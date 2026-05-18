use crate::utils::math::ConstantProduct;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        mint_to, transfer_checked, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked,
    },
};

use crate::{
    constants::{CONFIG_SEED, LP_SEED, PRECISION},
    error::AmmError,
    state::Config,
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
        seeds = [LP_SEED, config.key().as_ref()],
        bump = config.lp_bump,
        mint::token_program = token_program,
    )]
    pub mint_lp: Box<InterfaceAccount<'info, Mint>>,

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

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_lp: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    pub fn deposit(&mut self, amount: u64, max_a: u64, max_b: u64) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);
        require!(amount > 0, AmmError::InvalidAmount);

        let (x, y) =
            if self.mint_lp.supply == 0 && self.vault_a.amount == 0 && self.vault_b.amount == 0 {
                require!(max_a > 0 && max_b > 0, AmmError::InvalidAmount);
                (max_a, max_b)
            } else {
                let amounts = ConstantProduct::xy_deposit_amounts_from_l(
                    self.vault_a.amount,
                    self.vault_b.amount,
                    self.mint_lp.supply,
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

        self.deposit_tokens(true, x)?;
        self.deposit_tokens(false, y)?;
        self.mint_lp_tokens(amount)
    }

    fn deposit_tokens(&self, is_a: bool, amount: u64) -> Result<()> {
        let (from, to, mint, decimals) = match is_a {
            true => (
                self.user_a.to_account_info(),
                self.vault_a.to_account_info(),
                self.mint_a.to_account_info(),
                self.mint_a.decimals,
            ),
            false => (
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

    fn mint_lp_tokens(&self, amount: u64) -> Result<()> {
        let cpi_accounts = MintTo {
            mint: self.mint_lp.to_account_info(),
            authority: self.config.to_account_info(),
            to: self.user_lp.to_account_info(),
        };

        let signer_seeds: &[&[&[u8]]] = &[&[
            CONFIG_SEED,
            &self.config.seed.to_le_bytes(),
            &[self.config.config_bump],
        ]];

        let cpi_ctx =
            CpiContext::new_with_signer(self.token_program.key(), cpi_accounts, signer_seeds);

        mint_to(cpi_ctx, amount)
    }
}
