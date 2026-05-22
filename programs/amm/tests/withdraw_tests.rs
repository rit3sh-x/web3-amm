pub mod utils;

use amm::error::AmmError;
use utils::*;

#[test]
fn withdraw_returns_reserves_and_closes_position() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);

    let l = 300_000_000u64;
    let deposit_ix = amm.deposit_ix(&lp, l, l, l);
    let result = send_instruction(&mut svm, &lp.kp, deposit_ix);
    assert_ok(result);

    let withdraw_ix = amm.withdraw_ix(&lp, l, 0, 0);
    let result = send_instruction(&mut svm, &lp.kp, withdraw_ix);
    assert_ok(result);

    let cfg = amm_state(&svm, &amm.config);
    assert_eq!(cfg.total_liquidity, 0);
    assert_eq!(cfg.reserve_a, 0);
    assert_eq!(cfg.reserve_b, 0);

    let pos = position_state(&svm, &lp.position).expect("position still exists");
    assert_eq!(pos.liquidity, 0);

    assert_eq!(token_balance(&svm, &lp.ata_a), MINT_AMOUNT);
    assert_eq!(token_balance(&svm, &lp.ata_b), MINT_AMOUNT);
    assert_token_conservation(&svm, &amm, &[&lp]);
}

#[test]
fn withdraw_respects_min_out() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);

    let l = 300_000_000u64;
    let deposit_ix = amm.deposit_ix(&lp, l, l, l);
    let result = send_instruction(&mut svm, &lp.kp, deposit_ix);
    assert_ok(result);

    let withdraw_ix = amm.withdraw_ix(&lp, l, u64::MAX, u64::MAX);
    let result = send_instruction(&mut svm, &lp.kp, withdraw_ix);
    assert_error(result, AmmError::SlippageExceeded);
}

#[test]
fn withdraw_rejects_more_than_position() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);

    let l = 200_000_000u64;
    assert_ok(send_instruction(
        &mut svm,
        &lp.kp,
        amm.deposit_ix(&lp, l, l, l),
    ));

    let result = send_instruction(
        &mut svm,
        &lp.kp,
        amm.withdraw_ix(&lp, l.checked_add(1).unwrap(), 0, 0),
    );
    assert_error(result, AmmError::InsufficientBalance);
}

#[test]
fn partial_withdraw_keeps_position_open() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);

    let l = 400_000_000u64;
    assert_ok(send_instruction(
        &mut svm,
        &lp.kp,
        amm.deposit_ix(&lp, l, l, l),
    ));

    let half = l.checked_div(2).unwrap();
    assert_ok(send_instruction(
        &mut svm,
        &lp.kp,
        amm.withdraw_ix(&lp, half, 0, 0),
    ));

    let pos = position_state(&svm, &lp.position).expect("position still exists");
    assert_eq!(pos.liquidity, l - half);
    assert_eq!(
        amm_state(&svm, &amm.config).total_liquidity,
        l.checked_sub(half).unwrap()
    );
}
