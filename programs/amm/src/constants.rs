use anchor_lang::prelude::*;

#[constant]
pub const LP_SEED: &[u8] = b"lp";

#[constant]
pub const CONFIG_SEED: &[u8] = b"config";

#[constant]
pub const PRECISION: u32 = 1_000_000;

#[constant]
pub const MAX_FEE_BPS: u16 = 10_000;
