pub mod utils;

use amm::{error::AmmError, SwapDirection};
use solana_keypair::Keypair;
use solana_signer::Signer;
use utils::*;

#[test]
fn lock_blocks_ops_and_unlock_restores() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);

    let l = 300_000_000u64;
    let deposit_ix = amm.deposit_ix(&lp, l, l, l);
    let result = send_instruction(&mut svm, &lp.kp, deposit_ix);
    assert_ok(result);

    let set_locked_ix = amm.set_locked_ix(amm.initializer.pubkey(), true);
    let result = send_instruction(&mut svm, &amm.initializer, set_locked_ix);
    assert_ok(result);
    assert!(amm_state(&svm, &amm.config).locked);

    let deposit_ix = amm.deposit_ix(&lp, 1, 1, 1);
    let result = send_instruction(&mut svm, &lp.kp, deposit_ix);
    assert_error(result, AmmError::PoolLocked);

    let swap_ix = amm.swap_ix(&lp, SwapDirection::AtoB, 1_000, 1);
    let result = send_instruction(&mut svm, &lp.kp, swap_ix);
    assert_error(result, AmmError::PoolLocked);

    let withdraw_ix = amm.withdraw_ix(&lp, 1, 0, 0);
    let result = send_instruction(&mut svm, &lp.kp, withdraw_ix);
    assert_error(result, AmmError::PoolLocked);

    let set_locked_ix = amm.set_locked_ix(amm.initializer.pubkey(), false);
    let result = send_instruction(&mut svm, &amm.initializer, set_locked_ix);
    assert_ok(result);
    assert!(!amm_state(&svm, &amm.config).locked);

    let swap_ix = amm.swap_ix(&lp, SwapDirection::AtoB, 1_000, 1);
    let result = send_instruction(&mut svm, &lp.kp, swap_ix);
    assert_ok(result);
}

#[test]
fn set_locked_rejects_non_authority() {
    let (mut svm, amm) = fresh_pool(300);
    let attacker = Keypair::new();
    airdrop_to_user(&mut svm, &attacker.pubkey());

    let set_locked_ix = amm.set_locked_ix(attacker.pubkey(), true);
    let result = send_instruction(&mut svm, &attacker, set_locked_ix);
    assert_error(result, AmmError::Unauthorized);
    assert!(!amm_state(&svm, &amm.config).locked);
}

#[test]
fn set_locked_on_authorityless_pool_fails() {
    let mut svm = init_svm();
    let amm = AmmAccounts::new(&mut svm, generate_seed());

    let init_ix = amm.init_ix(300, None);
    let result = send_instruction(&mut svm, &amm.initializer, init_ix);
    assert_ok(result);

    let set_locked_ix = amm.set_locked_ix(amm.initializer.pubkey(), true);
    let result = send_instruction(&mut svm, &amm.initializer, set_locked_ix);
    assert_error(result, AmmError::NoAuthority);
}

#[test]
fn set_fee_updates_fee() {
    let (mut svm, amm) = fresh_pool(300);
    assert_eq!(amm_state(&svm, &amm.config).fee, 300);

    let set_fee_ix = amm.set_fee_ix(amm.initializer.pubkey(), 50);
    assert_ok(send_instruction(&mut svm, &amm.initializer, set_fee_ix));
    assert_eq!(amm_state(&svm, &amm.config).fee, 50);
}

#[test]
fn set_fee_rejects_non_authority() {
    let (mut svm, amm) = fresh_pool(300);
    let attacker = Keypair::new();
    airdrop_to_user(&mut svm, &attacker.pubkey());

    let set_fee_ix = amm.set_fee_ix(attacker.pubkey(), 100);
    let result = send_instruction(&mut svm, &attacker, set_fee_ix);
    assert_error(result, AmmError::Unauthorized);
    assert_eq!(amm_state(&svm, &amm.config).fee, 300);
}

#[test]
fn set_fee_rejects_at_or_above_max() {
    let (mut svm, amm) = fresh_pool(300);
    let set_fee_ix = amm.set_fee_ix(amm.initializer.pubkey(), 10_000);
    let result = send_instruction(&mut svm, &amm.initializer, set_fee_ix);
    assert_error(result, AmmError::InvalidFee);
    assert_eq!(amm_state(&svm, &amm.config).fee, 300);
}

#[test]
fn set_fee_rejects_authorityless_pool() {
    let mut svm = init_svm();
    let amm = AmmAccounts::new(&mut svm, generate_seed());
    assert_ok(send_instruction(
        &mut svm,
        &amm.initializer,
        amm.init_ix(300, None),
    ));

    let set_fee_ix = amm.set_fee_ix(amm.initializer.pubkey(), 100);
    let result = send_instruction(&mut svm, &amm.initializer, set_fee_ix);
    assert_error(result, AmmError::NoAuthority);
}

#[test]
fn set_fee_applies_to_next_swap() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let trader = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);

    let l = 500_000_000u64;
    assert_ok(send_instruction(
        &mut svm,
        &lp.kp,
        amm.deposit_ix(&lp, l, l, l),
    ));

    let amount_in = 10_000_000u64;
    assert_ok(send_instruction(
        &mut svm,
        &trader.kp,
        amm.swap_ix(&trader, SwapDirection::AtoB, amount_in, 1),
    ));

    assert_ok(send_instruction(
        &mut svm,
        &amm.initializer,
        amm.set_fee_ix(amm.initializer.pubkey(), 900),
    ));
    assert_eq!(amm_state(&svm, &amm.config).fee, 900);

    assert_ok(send_instruction(
        &mut svm,
        &trader.kp,
        amm.swap_ix(&trader, SwapDirection::AtoB, amount_in, 1),
    ));
}

#[test]
fn set_authority_transfers_to_new_keypair() {
    let (mut svm, amm) = fresh_pool(300);
    let new_authority = Keypair::new();
    airdrop_to_user(&mut svm, &new_authority.pubkey());

    let ix = amm.set_authority_ix(amm.initializer.pubkey(), Some(new_authority.pubkey()));
    assert_ok(send_instruction(&mut svm, &amm.initializer, ix));
    assert_eq!(
        amm_state(&svm, &amm.config).authority,
        Some(new_authority.pubkey())
    );

    let ix = amm.set_fee_ix(new_authority.pubkey(), 50);
    assert_ok(send_instruction(&mut svm, &new_authority, ix));
    assert_eq!(amm_state(&svm, &amm.config).fee, 50);

    let ix = amm.set_locked_ix(amm.initializer.pubkey(), true);
    let result = send_instruction(&mut svm, &amm.initializer, ix);
    assert_error(result, AmmError::Unauthorized);
}

#[test]
fn set_authority_rejects_non_authority() {
    let (mut svm, amm) = fresh_pool(300);
    let attacker = Keypair::new();
    airdrop_to_user(&mut svm, &attacker.pubkey());

    let ix = amm.set_authority_ix(attacker.pubkey(), Some(attacker.pubkey()));
    let result = send_instruction(&mut svm, &attacker, ix);
    assert_error(result, AmmError::Unauthorized);
    assert_eq!(
        amm_state(&svm, &amm.config).authority,
        Some(amm.initializer.pubkey())
    );
}

#[test]
fn set_authority_renounces_when_none() {
    let (mut svm, amm) = fresh_pool(300);

    let ix = amm.set_authority_ix(amm.initializer.pubkey(), None);
    assert_ok(send_instruction(&mut svm, &amm.initializer, ix));
    assert_eq!(amm_state(&svm, &amm.config).authority, None);

    let ix = amm.set_fee_ix(amm.initializer.pubkey(), 50);
    let result = send_instruction(&mut svm, &amm.initializer, ix);
    assert_error(result, AmmError::NoAuthority);

    let ix = amm.set_locked_ix(amm.initializer.pubkey(), true);
    let result = send_instruction(&mut svm, &amm.initializer, ix);
    assert_error(result, AmmError::NoAuthority);
}

#[test]
fn set_authority_rejects_authorityless_pool() {
    let mut svm = init_svm();
    let amm = AmmAccounts::new(&mut svm, generate_seed());
    assert_ok(send_instruction(
        &mut svm,
        &amm.initializer,
        amm.init_ix(300, None),
    ));

    let ix = amm.set_authority_ix(amm.initializer.pubkey(), Some(amm.initializer.pubkey()));
    let result = send_instruction(&mut svm, &amm.initializer, ix);
    assert_error(result, AmmError::NoAuthority);
}
