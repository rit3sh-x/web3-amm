#![allow(dead_code)]

use {
    amm::{SwapDirection, CONFIG_SEED, LP_SEED},
    anchor_lang::{
        prelude::*,
        solana_program::{instruction::Instruction, program_pack::Pack},
        system_program::ID as SYSTEM_PROGRAM_ID,
        InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::{self, ID as ASSOCIATED_TOKEN_PROGRAM_ID},
        token::ID as TOKEN_PROGRAM_ID,
    },
    litesvm::{types::TransactionResult, LiteSVM},
    litesvm_token::{spl_token, CreateAssociatedTokenAccount, CreateMint, MintTo},
    solana_keypair::Keypair,
    solana_message::Message,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::{InstructionError, Transaction, TransactionError},
    std::time::{SystemTime, UNIX_EPOCH},
};

pub const INITIAL_USER_LAMPORTS: u64 = 5_000_000_000;
pub const MINT_AMOUNT: u64 = 1_000_000_000;
pub const MINT_DECIMALS: u8 = 6;

pub fn init_svm() -> LiteSVM {
    let bytes = include_bytes!("../../../target/deploy/amm.so");
    let mut svm = LiteSVM::new();
    svm.add_program(amm::id(), bytes).unwrap();
    svm
}

pub fn generate_seed() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    (nanos % u64::MAX as u128) as u64
}

pub fn airdrop_to_user(svm: &mut LiteSVM, user: &Pubkey) {
    svm.airdrop(user, INITIAL_USER_LAMPORTS).unwrap();
}

pub fn create_mint(svm: &mut LiteSVM, authority: &Keypair) -> Pubkey {
    CreateMint::new(svm, authority)
        .decimals(MINT_DECIMALS)
        .authority(&authority.pubkey())
        .send()
        .unwrap()
}

pub fn create_ata(svm: &mut LiteSVM, payer: &Keypair, mint: &Pubkey, owner: &Pubkey) -> Pubkey {
    let ata = associated_token::get_associated_token_address(owner, mint);
    CreateAssociatedTokenAccount::new(svm, payer, mint)
        .owner(owner)
        .send()
        .unwrap();
    ata
}

pub fn mint_tokens_to_ata(
    svm: &mut LiteSVM,
    authority: &Keypair,
    mint: &Pubkey,
    ata: &Pubkey,
    amount: u64,
) {
    MintTo::new(svm, authority, mint, ata, amount)
        .send()
        .unwrap();
}

pub fn ordered_mints(mint_a: Pubkey, mint_b: Pubkey) -> (Pubkey, Pubkey) {
    if mint_a < mint_b {
        (mint_a, mint_b)
    } else {
        (mint_b, mint_a)
    }
}

fn mint_pair(svm: &mut LiteSVM, authority: &Keypair) -> (Pubkey, Pubkey) {
    let m1 = create_mint(svm, authority);
    let m2 = create_mint(svm, authority);
    ordered_mints(m1, m2)
}

pub fn send_instruction(
    svm: &mut LiteSVM,
    signer: &Keypair,
    instruction: Instruction,
) -> TransactionResult {
    let message = Message::new(&[instruction], Some(&signer.pubkey()));
    let recent_blockhash = svm.latest_blockhash();
    let transaction = Transaction::new(&[signer], message, recent_blockhash);
    let result = svm.send_transaction(transaction);
    svm.expire_blockhash();
    result
}

pub fn token_balance(svm: &LiteSVM, ata: &Pubkey) -> u64 {
    match svm.get_account(ata) {
        Some(acc) if !acc.data.is_empty() => {
            spl_token::state::Account::unpack(&acc.data).unwrap().amount
        }
        _ => 0,
    }
}

pub fn mint_supply(svm: &LiteSVM, mint: &Pubkey) -> u64 {
    let acc = svm.get_account(mint).unwrap();
    spl_token::state::Mint::unpack(&acc.data).unwrap().supply
}

pub fn amm_state(svm: &LiteSVM, config: &Pubkey) -> amm::state::Config {
    let account = svm.get_account(config).unwrap();
    amm::state::Config::try_deserialize(&mut account.data.as_ref()).unwrap()
}

pub fn assert_error<E>(result: TransactionResult, expected_error: E)
where
    E: Into<u32> + std::fmt::Debug + Copy,
{
    let expected_code: u32 = expected_error.into();

    match result {
        Ok(meta) => {
            panic!(
                "Expected transaction to fail with error `{:?}` (code: {}), but it succeeded.\nLogs:\n{:#?}",
                expected_error, expected_code, meta.logs
            );
        }
        Err(failed) => {
            if let TransactionError::InstructionError(_, InstructionError::Custom(actual_code)) =
                failed.err
            {
                assert_eq!(
                    actual_code, expected_code,
                    "Error Mismatch!\nExpected: `{:?}` (Code: {})\nGot: Code {}",
                    expected_error, expected_code, actual_code
                );
            } else {
                panic!(
                    "Expected custom program error `{:?}`, but transaction dropped with a structural runtime error: {:?}.\nLogs:\n{:#?}",
                    expected_error, failed.err, failed.meta.logs
                );
            }
        }
    }
}

pub fn assert_ok(result: TransactionResult) {
    if let Err(failed) = result {
        panic!("expected success, got error:\n{:#?}", failed.meta.logs);
    }
}

pub struct User {
    pub kp: Keypair,
    pub ata_a: Pubkey,
    pub ata_b: Pubkey,
    pub ata_lp: Pubkey,
}

impl User {
    pub fn pubkey(&self) -> Pubkey {
        self.kp.pubkey()
    }
}

pub struct AmmAccounts {
    pub initializer: Keypair,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub mint_lp: Pubkey,
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,
    pub seed: u64,
    pub config: Pubkey,
    pub minted_a: u64,
    pub minted_b: u64,
}

impl AmmAccounts {
    pub fn new(svm: &mut LiteSVM, seed: u64) -> Self {
        let initializer = Keypair::new();
        airdrop_to_user(svm, &initializer.pubkey());

        let (mint_a, mint_b) = mint_pair(svm, &initializer);

        let (config, _) =
            Pubkey::find_program_address(&[CONFIG_SEED, &seed.to_le_bytes()], &amm::id());
        let (mint_lp, _) = Pubkey::find_program_address(&[LP_SEED, config.as_ref()], &amm::id());
        let vault_a = associated_token::get_associated_token_address(&config, &mint_a);
        let vault_b = associated_token::get_associated_token_address(&config, &mint_b);

        Self {
            initializer,
            mint_a,
            mint_b,
            mint_lp,
            vault_a,
            vault_b,
            seed,
            config,
            minted_a: 0,
            minted_b: 0,
        }
    }

    pub fn new_user(&mut self, svm: &mut LiteSVM, fund_a: u64, fund_b: u64) -> User {
        let kp = Keypair::new();
        airdrop_to_user(svm, &kp.pubkey());

        let ata_a = create_ata(svm, &self.initializer, &self.mint_a, &kp.pubkey());
        let ata_b = create_ata(svm, &self.initializer, &self.mint_b, &kp.pubkey());
        let ata_lp = create_ata(svm, &self.initializer, &self.mint_lp, &kp.pubkey());

        if fund_a > 0 {
            mint_tokens_to_ata(svm, &self.initializer, &self.mint_a, &ata_a, fund_a);
            self.minted_a = self
                .minted_a
                .checked_add(fund_a)
                .expect("minted_a overflow");
        }
        if fund_b > 0 {
            mint_tokens_to_ata(svm, &self.initializer, &self.mint_b, &ata_b, fund_b);
            self.minted_b = self
                .minted_b
                .checked_add(fund_b)
                .expect("minted_b overflow");
        }

        User {
            kp,
            ata_a,
            ata_b,
            ata_lp,
        }
    }

    pub fn k(&self, svm: &LiteSVM) -> u128 {
        token_balance(svm, &self.vault_a) as u128 * token_balance(svm, &self.vault_b) as u128
    }

    pub fn init_ix(&self, fee: u16, authority: Option<Pubkey>) -> Instruction {
        Instruction {
            program_id: amm::id(),
            accounts: amm::accounts::Init {
                config: self.config,
                initializer: self.initializer.pubkey(),
                mint_lp: self.mint_lp,
                mint_a: self.mint_a,
                mint_b: self.mint_b,
                vault_a: self.vault_a,
                vault_b: self.vault_b,
                system_program: SYSTEM_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: amm::instruction::Init {
                seed: self.seed,
                authority,
                fee,
            }
            .data(),
        }
    }

    pub fn deposit_ix(&self, user: &User, amount: u64, max_x: u64, max_y: u64) -> Instruction {
        Instruction {
            program_id: amm::id(),
            accounts: amm::accounts::Deposit {
                config: self.config,
                mint_lp: self.mint_lp,
                user: user.pubkey(),
                user_a: user.ata_a,
                user_b: user.ata_b,
                user_lp: user.ata_lp,
                mint_a: self.mint_a,
                mint_b: self.mint_b,
                vault_a: self.vault_a,
                vault_b: self.vault_b,
                system_program: SYSTEM_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: amm::instruction::Deposit {
                amount,
                max_x,
                max_y,
            }
            .data(),
        }
    }

    pub fn swap_ix(
        &self,
        user: &User,
        direction: SwapDirection,
        amount: u64,
        min: u64,
    ) -> Instruction {
        Instruction {
            program_id: amm::id(),
            accounts: amm::accounts::Swap {
                config: self.config,
                mint_lp: self.mint_lp,
                user: user.pubkey(),
                user_a: user.ata_a,
                user_b: user.ata_b,
                mint_a: self.mint_a,
                mint_b: self.mint_b,
                vault_a: self.vault_a,
                vault_b: self.vault_b,
                token_program: TOKEN_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: amm::instruction::Swap {
                amount,
                direction,
                min,
            }
            .data(),
        }
    }

    pub fn withdraw_ix(&self, user: &User, amount: u64, min_x: u64, min_y: u64) -> Instruction {
        Instruction {
            program_id: amm::id(),
            accounts: amm::accounts::Withdraw {
                config: self.config,
                mint_lp: self.mint_lp,
                user: user.pubkey(),
                user_a: user.ata_a,
                user_b: user.ata_b,
                user_lp: user.ata_lp,
                mint_a: self.mint_a,
                mint_b: self.mint_b,
                vault_a: self.vault_a,
                vault_b: self.vault_b,
                system_program: SYSTEM_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: amm::instruction::Withdraw {
                amount,
                min_x,
                min_y,
            }
            .data(),
        }
    }

    pub fn set_locked_ix(&self, authority: Pubkey, locked: bool) -> Instruction {
        Instruction {
            program_id: amm::id(),
            accounts: amm::accounts::SetLocked {
                config: self.config,
                authority,
            }
            .to_account_metas(None),
            data: amm::instruction::SetLocked { locked }.data(),
        }
    }
}

pub fn fresh_pool(fee: u16) -> (litesvm::LiteSVM, AmmAccounts) {
    let mut svm = init_svm();
    let amm = AmmAccounts::new(&mut svm, generate_seed());
    let init_ix = amm.init_ix(fee, Some(amm.initializer.pubkey()));
    let result = send_instruction(&mut svm, &amm.initializer, init_ix);
    assert_ok(result);

    (svm, amm)
}

pub fn assert_token_conservation(svm: &litesvm::LiteSVM, amm: &AmmAccounts, users: &[&User]) {
    let users_a: u64 = users.iter().map(|u| token_balance(svm, &u.ata_a)).sum();
    let users_b: u64 = users.iter().map(|u| token_balance(svm, &u.ata_b)).sum();
    assert_eq!(
        users_a + token_balance(svm, &amm.vault_a),
        amm.minted_a,
        "token A was created or destroyed"
    );
    assert_eq!(
        users_b + token_balance(svm, &amm.vault_b),
        amm.minted_b,
        "token B was created or destroyed"
    );
}
