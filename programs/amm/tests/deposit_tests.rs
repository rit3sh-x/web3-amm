pub mod utils;

use amm::error::AmmError;
use utils::*;

#[test]
fn first_deposit_sets_reserves_and_mints_lp() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);

    let l = 300_000_000u64;
    let deposit_ix = amm.deposit_ix(&lp, l, l, l);
    let result = send_instruction(&mut svm, &lp.kp, deposit_ix);
    assert_ok(result);

    assert_eq!(token_balance(&svm, &amm.vault_a), l);
    assert_eq!(token_balance(&svm, &amm.vault_b), l);
    assert_eq!(mint_supply(&svm, &amm.mint_lp), l);
    assert_eq!(token_balance(&svm, &lp.ata_lp), l);
    assert_eq!(token_balance(&svm, &lp.ata_a), MINT_AMOUNT - l);
    assert_eq!(token_balance(&svm, &lp.ata_b), MINT_AMOUNT - l);
    assert_token_conservation(&svm, &amm, &[&lp]);
}

#[test]
fn deposit_rejects_zero_amount() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let deposit_ix = amm.deposit_ix(&lp, 0, 1, 1);
    let result = send_instruction(&mut svm, &lp.kp, deposit_ix);
    assert_error(result, AmmError::InvalidAmount);
}

#[test]
fn second_deposit_is_proportional() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp1 = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let lp2 = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);

    let l0 = 300_000_000u64;
    let deposit_ix = amm.deposit_ix(&lp1, l0, l0, l0);
    let result = send_instruction(&mut svm, &lp1.kp, deposit_ix);
    assert_ok(result);

    let l1 = 100_000_000u64;
    let l1_twice = l1.checked_mul(2).expect("l1 * 2 overflow");
    let deposit_ix = amm.deposit_ix(&lp2, l1, l1_twice, l1_twice);
    let result = send_instruction(&mut svm, &lp2.kp, deposit_ix);
    assert_ok(result);

    assert_eq!(mint_supply(&svm, &amm.mint_lp), l0 + l1);
    assert_eq!(token_balance(&svm, &lp2.ata_lp), l1);

    let pulled_a = MINT_AMOUNT
        .checked_sub(token_balance(&svm, &lp2.ata_a))
        .expect("pulled_a underflow");
    let pulled_b = MINT_AMOUNT
        .checked_sub(token_balance(&svm, &lp2.ata_b))
        .expect("pulled_b underflow");

    assert!(pulled_a > 0 && pulled_a <= l1_twice);
    assert!(pulled_b > 0 && pulled_b <= l1_twice);
    assert_token_conservation(&svm, &amm, &[&lp1, &lp2]);
}

#[test]
fn deposit_respects_max_slippage() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp1 = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let lp2 = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);

    let l0 = 300_000_000u64;
    let deposit_ix = amm.deposit_ix(&lp1, l0, l0, l0);
    let result = send_instruction(&mut svm, &lp1.kp, deposit_ix);
    assert_ok(result);

    let deposit_ix = amm.deposit_ix(&lp2, 100_000_000, 1, 1);
    let result = send_instruction(&mut svm, &lp2.kp, deposit_ix);
    assert_error(result, AmmError::SlippageExceeded);
}
