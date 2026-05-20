pub mod utils;

use amm::error::AmmError;
use solana_signer::Signer;
use utils::*;

#[test]
fn init_succeeds_and_sets_config() {
    let (svm, amm) = fresh_pool(300);

    let cfg = amm_state(&svm, &amm.config);
    assert_eq!(cfg.seed, amm.seed);
    assert_eq!(cfg.fee, 300);
    assert_eq!(cfg.mint_a, amm.mint_a);
    assert_eq!(cfg.mint_b, amm.mint_b);
    assert_eq!(cfg.authority, Some(amm.initializer.pubkey()));
    assert!(!cfg.locked);
    assert_eq!(mint_supply(&svm, &amm.mint_lp), 0);
    assert_eq!(token_balance(&svm, &amm.vault_a), 0);
    assert_eq!(token_balance(&svm, &amm.vault_b), 0);
}

#[test]
fn init_rejects_fee_at_or_above_max() {
    let mut svm = init_svm();
    let amm = AmmAccounts::new(&mut svm, generate_seed());
    let init_ix = amm.init_ix(10_000, Some(amm.initializer.pubkey()));
    let result = send_instruction(&mut svm, &amm.initializer, init_ix);
    assert_error(result, AmmError::InvalidFee);
}

#[test]
fn init_accepts_zero_fee() {
    let mut svm = init_svm();
    let amm = AmmAccounts::new(&mut svm, generate_seed());
    let init_ix = amm.init_ix(0, Some(amm.initializer.pubkey()));
    let result = send_instruction(&mut svm, &amm.initializer, init_ix);
    assert!(result.is_ok());
    assert_eq!(amm_state(&svm, &amm.config).fee, 0);
}
