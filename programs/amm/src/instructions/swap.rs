use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{
    constants::{CONFIG_SEED, LP_SEED},
    error::AmmError,
    state::Config, utils::cmm::{CMM, LiquidityPair},
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SwapDirection {
    AtoB,
    BtoA,
}

impl From<SwapDirection> for LiquidityPair {
    fn from(direction: SwapDirection) -> Self {
        match direction {
            SwapDirection::AtoB => LiquidityPair::X,
            SwapDirection::BtoA => LiquidityPair::Y,
        }
    }
}

#[derive(Accounts)]
pub struct Swap<'info> {
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

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Swap<'info> {
    pub fn swap(&mut self, direction: SwapDirection, amount: u64, min: u64) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);
        require!(amount > 0, AmmError::InvalidAmount);
        require!(
            self.vault_a.amount > 0 && self.vault_b.amount > 0,
            AmmError::NoLiquidity
        );

        let mut curve = CMM::initialize_cmm(
            self.vault_a.amount,
            self.vault_b.amount,
            self.config.fee,
        )
        .map_err(AmmError::from)?;

        let swap_result = curve
            .swap(direction.into(), amount, min)
            .map_err(AmmError::from)?;

        self.transfer_in(direction, swap_result.deposit)?;
        self.transfer_out(direction, swap_result.withdraw)
    }

    fn input_leg(
        &self,
        direction: SwapDirection,
    ) -> (
        AccountInfo<'info>,
        AccountInfo<'info>,
        AccountInfo<'info>,
        u8,
    ) {
        match direction {
            SwapDirection::AtoB => (
                self.user_a.to_account_info(),
                self.vault_a.to_account_info(),
                self.mint_a.to_account_info(),
                self.mint_a.decimals,
            ),
            SwapDirection::BtoA => (
                self.user_b.to_account_info(),
                self.vault_b.to_account_info(),
                self.mint_b.to_account_info(),
                self.mint_b.decimals,
            ),
        }
    }

    fn output_leg(
        &self,
        direction: SwapDirection,
    ) -> (
        AccountInfo<'info>,
        AccountInfo<'info>,
        AccountInfo<'info>,
        u8,
    ) {
        match direction {
            SwapDirection::AtoB => (
                self.vault_b.to_account_info(),
                self.user_b.to_account_info(),
                self.mint_b.to_account_info(),
                self.mint_b.decimals,
            ),
            SwapDirection::BtoA => (
                self.vault_a.to_account_info(),
                self.user_a.to_account_info(),
                self.mint_a.to_account_info(),
                self.mint_a.decimals,
            ),
        }
    }

    fn transfer_in(&self, direction: SwapDirection, amount: u64) -> Result<()> {
        let (from, to, mint, decimals) = self.input_leg(direction);

        let cpi_accounts = TransferChecked {
            from,
            to,
            authority: self.user.to_account_info(),
            mint,
        };

        let cpi_ctx = CpiContext::new(self.token_program.key(), cpi_accounts);

        transfer_checked(cpi_ctx, amount, decimals)
    }

    fn transfer_out(&self, direction: SwapDirection, amount: u64) -> Result<()> {
        let (from, to, mint, decimals) = self.output_leg(direction);

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
