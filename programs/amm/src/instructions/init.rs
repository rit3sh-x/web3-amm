use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::spl_token_2022::{
        extension::{BaseStateWithExtensions, ExtensionType, StateWithExtensions},
        state::Mint as MintState,
    },
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::constants::{CONFIG_SEED, LP_SEED, MAX_FEE_BPS};
use crate::error::AmmError;
use crate::state::Config;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Init<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,

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
        init,
        payer = initializer,
        seeds = [LP_SEED, config.key().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = config,
        mint::token_program = token_program,
    )]
    pub mint_lp: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_a,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub vault_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_b,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = initializer,
        seeds = [CONFIG_SEED, seed.to_le_bytes().as_ref()],
        bump,
        space = Config::DISCRIMINATOR.len() + Config::INIT_SPACE,
    )]
    pub config: Box<Account<'info, Config>>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub system_program: Program<'info, System>,
}

impl<'info> Init<'info> {
    pub fn init(
        &mut self,
        seed: u64,
        fee: u16,
        authority: Option<Pubkey>,
        bumps: &InitBumps,
    ) -> Result<()> {
        require!(fee < MAX_FEE_BPS, AmmError::InvalidFee);

        Self::assert_supported_mint(&self.mint_a.to_account_info())?;
        Self::assert_supported_mint(&self.mint_b.to_account_info())?;

        self.config.set_inner(Config {
            seed,
            authority,
            mint_a: self.mint_a.key(),
            mint_b: self.mint_b.key(),
            fee,
            locked: false,
            config_bump: bumps.config,
            lp_bump: bumps.mint_lp,
        });

        Ok(())
    }

    fn assert_supported_mint(mint_ai: &AccountInfo) -> Result<()> {
        if mint_ai.owner == &anchor_spl::token::ID {
            return Ok(());
        }

        let data = mint_ai.try_borrow_data()?;
        let mint = StateWithExtensions::<MintState>::unpack(&data)
            .map_err(|_| error!(AmmError::UnsupportedMintExtension))?;

        for ext in mint
            .get_extension_types()
            .map_err(|_| error!(AmmError::UnsupportedMintExtension))?
        {
            match ext {
                ExtensionType::TransferFeeConfig
                | ExtensionType::TransferHook
                | ExtensionType::NonTransferable
                | ExtensionType::DefaultAccountState
                | ExtensionType::PermanentDelegate
                | ExtensionType::ConfidentialTransferMint
                | ExtensionType::ConfidentialTransferFeeConfig => {
                    return err!(AmmError::UnsupportedMintExtension);
                }
                _ => {}
            }
        }

        Ok(())
    }
}
