pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub mod utils;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("5jsjvYQBSxMiFQ75yxgCvhkmzWpjjkz6XBS3do2dKrwp");

#[program]
pub mod amm {
    use super::*;

    pub fn init(ctx: Context<Init>, seed: u64, fee: u16, authority: Option<Pubkey>) -> Result<()> {
        ctx.accounts.init(seed, fee, authority, &ctx.bumps)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64, max_x: u64, max_y: u64) -> Result<()> {
        ctx.accounts.deposit(amount, max_x, max_y, &ctx.bumps)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64, min_x: u64, min_y: u64) -> Result<()> {
        ctx.accounts.withdraw(amount, min_x, min_y)
    }

    pub fn swap(ctx: Context<Swap>, direction: SwapDirection, amount: u64, min: u64) -> Result<()> {
        ctx.accounts.swap(direction, amount, min)
    }

    pub fn collect_fees(ctx: Context<CollectFees>) -> Result<()> {
        ctx.accounts.collect_fees()
    }

    pub fn set_locked(ctx: Context<Admin>, locked: bool) -> Result<()> {
        ctx.accounts.set_locked(locked)
    }

    pub fn set_fee(ctx: Context<Admin>, fee: u16) -> Result<()> {
        ctx.accounts.set_fee(fee)
    }

    pub fn set_authority(ctx: Context<Admin>, authority: Option<Pubkey>) -> Result<()> {
        ctx.accounts.set_authority(authority)
    }
}
