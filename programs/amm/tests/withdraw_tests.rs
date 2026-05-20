pub mod utils;

use amm::error::AmmError;
use utils::*;

#[test]
fn withdraw_burns_lp_and_returns_reserves() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);

    let l = 300_000_000u64;
    let deposit_ix = amm.deposit_ix(&lp, l, l, l);
    let result = send_instruction(&mut svm, &lp.kp, deposit_ix);
    assert_ok(result);

    let swap_ix = amm.withdraw_ix(&lp, l, 0, 0);
    let result = send_instruction(&mut svm, &lp.kp, swap_ix);
    assert_ok(result);

    assert_eq!(token_balance(&svm, &lp.ata_lp), 0);
    assert_eq!(mint_supply(&svm, &amm.mint_lp), 0);

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
