pub mod utils;

use amm::{error::AmmError, SwapDirection};
use utils::*;

#[test]
fn swap_a_to_b_preserves_reserve_k_and_accrues_fee_growth() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let trader = amm.new_user(&mut svm, 100_000_000, 0);

    let l = 300_000_000u64;

    let deposit_ix = amm.deposit_ix(&lp, l, l, l);
    assert_ok(send_instruction(&mut svm, &lp.kp, deposit_ix));

    let k_before = amm.k(&svm).unwrap();
    let amount_in = 10_000_000u64;
    let swap_ix = amm.swap_ix(&trader, SwapDirection::AtoB, amount_in, 1);
    assert_ok(send_instruction(&mut svm, &trader.kp, swap_ix));

    assert_eq!(token_balance(&svm, &trader.ata_a), 100_000_000 - amount_in);
    let got_b = token_balance(&svm, &trader.ata_b);
    assert!(got_b > 0, "trader should receive token B");

    let cfg = amm_state(&svm, &amm.config);
    assert!(
        amm.k(&svm).unwrap() >= k_before,
        "reserve k must never decrease"
    );
    assert!(
        cfg.fee_growth_a > 0,
        "fee_growth_a should advance on an AtoB swap"
    );
    assert_eq!(cfg.fee_growth_b, 0, "no fee_growth_b on AtoB direction");

    assert_eq!(token_balance(&svm, &amm.vault_a), l + amount_in);
    assert_eq!(token_balance(&svm, &amm.vault_b), l - got_b);
    assert_eq!(
        token_balance(&svm, &amm.vault_a) - cfg.reserve_a,
        amount_in - (cfg.reserve_a - l),
        "vault A excess over reserve equals the swap fee (token A)"
    );
    assert_token_conservation(&svm, &amm, &[&lp, &trader]);
}

#[test]
fn swap_b_to_a_advances_fee_growth_b_only() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let trader = amm.new_user(&mut svm, 0, 100_000_000);

    let l = 300_000_000u64;
    assert_ok(send_instruction(
        &mut svm,
        &lp.kp,
        amm.deposit_ix(&lp, l, l, l),
    ));

    let amount_in = 10_000_000u64;
    assert_ok(send_instruction(
        &mut svm,
        &trader.kp,
        amm.swap_ix(&trader, SwapDirection::BtoA, amount_in, 1),
    ));

    let cfg = amm_state(&svm, &amm.config);
    assert!(cfg.fee_growth_b > 0);
    assert_eq!(cfg.fee_growth_a, 0);

    assert!(token_balance(&svm, &trader.ata_a) > 0, "should receive A");
    assert_eq!(token_balance(&svm, &trader.ata_b), 90_000_000);
    assert_token_conservation(&svm, &amm, &[&lp, &trader]);
}

#[test]
fn swap_respects_min_out() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let trader = amm.new_user(&mut svm, 100_000_000, 0);

    let l = 300_000_000u64;
    assert_ok(send_instruction(
        &mut svm,
        &lp.kp,
        amm.deposit_ix(&lp, l, l, l),
    ));

    let amount_in = 10_000_000u64;
    let swap_ix = amm.swap_ix(&trader, SwapDirection::BtoA, amount_in, u64::MAX);
    let result = send_instruction(&mut svm, &trader.kp, swap_ix);
    assert_error(result, AmmError::SlippageExceeded);
}

#[test]
fn swap_rejects_zero_amount() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let trader = amm.new_user(&mut svm, 100_000_000, 0);

    let l = 300_000_000u64;
    assert_ok(send_instruction(
        &mut svm,
        &lp.kp,
        amm.deposit_ix(&lp, l, l, l),
    ));

    let swap_ix = amm.swap_ix(&trader, SwapDirection::BtoA, 0, 0);
    let result = send_instruction(&mut svm, &trader.kp, swap_ix);
    assert_error(result, AmmError::InvalidAmount);
}

#[test]
fn swap_rejects_without_liquidity() {
    let (mut svm, mut amm) = fresh_pool(300);
    let trader = amm.new_user(&mut svm, 100_000_000, 100_000_000);

    let swap_ix = amm.swap_ix(&trader, SwapDirection::AtoB, 1_000, 1);
    let result = send_instruction(&mut svm, &trader.kp, swap_ix);
    assert_error(result, AmmError::NoLiquidity);
}

#[test]
fn swap_btoa_must_not_decrease_k() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let trader = amm.new_user(&mut svm, 0, 100_000_000);

    let l = 400_000_000u64;
    assert_ok(send_instruction(
        &mut svm,
        &lp.kp,
        amm.deposit_ix(&lp, l, l, l),
    ));

    let k_before = amm.k(&svm).unwrap();
    let swap_ix = amm.swap_ix(&trader, SwapDirection::BtoA, 8_000_000, 1);
    assert_ok(send_instruction(&mut svm, &trader.kp, swap_ix));

    let k_after = amm.k(&svm).unwrap();
    assert!(
        k_after >= k_before,
        "BtoA swap leaked value: k_before={k_before} k_after={k_after}"
    );
}
