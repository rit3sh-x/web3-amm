use anchor_lang::prelude::*;

#[constant]
pub const CONFIG_SEED: &[u8] = b"config";

#[constant]
pub const POSITION_SEED: &[u8] = b"position";

#[constant]
pub const PRECISION: u32 = 1_000_000;
