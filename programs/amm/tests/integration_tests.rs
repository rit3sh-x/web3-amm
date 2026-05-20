pub mod utils;

use amm::SwapDirection;
use utils::*;

#[test]
fn multi_user_trading_conserves_tokens() {
    let (mut svm, mut amm) = fresh_pool(300);

    let lp1 = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let lp2 = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let alice = amm.new_user(&mut svm, 200_000_000, 200_000_000);
    let bob = amm.new_user(&mut svm, 200_000_000, 200_000_000);

    let l0 = 400_000_000u64;
    let deposit_ix = amm.deposit_ix(&lp1, l0, l0, l0);
    let result = send_instruction(&mut svm, &lp1.kp, deposit_ix);
    assert_ok(result);

    let l1 = 150_000_000u64;
    let two_l1 = l1.checked_mul(2).expect("l1 * 2 overflow");

    let deposit_ix = amm.deposit_ix(&lp2, l1, two_l1, two_l1);
    let result = send_instruction(&mut svm, &lp2.kp, deposit_ix);
    assert_ok(result);
    assert_eq!(mint_supply(&svm, &amm.mint_lp), l0 + l1);

    let trades = [
        (&alice, SwapDirection::AtoB, 12_000_000u64),
        (&bob, SwapDirection::BtoA, 8_000_000),
        (&alice, SwapDirection::BtoA, 5_000_000),
        (&bob, SwapDirection::AtoB, 17_000_000),
        (&alice, SwapDirection::AtoB, 3_000_000),
        (&bob, SwapDirection::BtoA, 9_000_000),
    ];
    let all = [&lp1, &lp2, &alice, &bob];
    for (trader, dir, amt) in trades {
        let swap_ix = amm.swap_ix(trader, dir, amt, 1);
        let result = send_instruction(&mut svm, &trader.kp, swap_ix);
        assert_ok(result);
        assert_token_conservation(&svm, &amm, &all);
    }

    let lp1_bal = token_balance(&svm, &lp1.ata_lp);
    let lp2_bal = token_balance(&svm, &lp2.ata_lp);

    let withdraw_ix = amm.withdraw_ix(&lp1, lp1_bal, 0, 0);
    let result = send_instruction(&mut svm, &lp1.kp, withdraw_ix);
    assert_ok(result);

    let withdraw_ix = amm.withdraw_ix(&lp2, lp2_bal, 0, 0);
    let result = send_instruction(&mut svm, &lp2.kp, withdraw_ix);
    assert_ok(result);

    assert_eq!(mint_supply(&svm, &amm.mint_lp), 0);
    assert_token_conservation(&svm, &amm, &all);
}

#[test]
fn multi_trader_atob_flow_earns_lp_fees() {
    let (mut svm, mut amm) = fresh_pool(300);

    let lp1 = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let lp2 = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let alice = amm.new_user(&mut svm, 300_000_000, 0);
    let bob = amm.new_user(&mut svm, 300_000_000, 0);

    let l0 = 400_000_000u64;
    let deposit_ix = amm.deposit_ix(&lp1, l0, l0, l0);
    let result = send_instruction(&mut svm, &lp1.kp, deposit_ix);
    assert_ok(result);

    let l1 = 150_000_000u64;
    let l1_x2 = l1.checked_mul(2).expect("l1 * 2 overflow");
    let deposit_ix = amm.deposit_ix(&lp2, l1, l1_x2, l1_x2);
    let result = send_instruction(&mut svm, &lp2.kp, deposit_ix);
    assert_ok(result);

    let trades = [
        (&alice, 12_000_000u64),
        (&bob, 17_000_000),
        (&alice, 3_000_000),
        (&bob, 9_000_000),
        (&alice, 21_000_000),
    ];
    let mut k = amm.k(&svm);
    for (trader, amt) in trades {
        let swap_ix = amm.swap_ix(trader, SwapDirection::AtoB, amt, 1);
        let result = send_instruction(&mut svm, &trader.kp, swap_ix);
        assert_ok(result);
        let k_new = amm.k(&svm);
        assert!(
            k_new >= k,
            "k should never decrease: before={k} after={k_new}"
        );
        k = k_new;
    }

    let all = [&lp1, &lp2, &alice, &bob];
    assert_token_conservation(&svm, &amm, &all);

    let lp1_bal = token_balance(&svm, &lp1.ata_lp);
    let lp2_bal = token_balance(&svm, &lp2.ata_lp);

    let withdraw_ix = amm.withdraw_ix(&lp1, lp1_bal, 0, 0);
    let result = send_instruction(&mut svm, &lp1.kp, withdraw_ix);
    assert_ok(result);

    let withdraw_ix = amm.withdraw_ix(&lp2, lp2_bal, 0, 0);
    let result = send_instruction(&mut svm, &lp2.kp, withdraw_ix);
    assert_ok(result);

    assert_eq!(mint_supply(&svm, &amm.mint_lp), 0);
    assert_token_conservation(&svm, &amm, &all);

    let lp_total_now: u128 = token_balance(&svm, &lp1.ata_a) as u128
        + token_balance(&svm, &lp1.ata_b) as u128
        + token_balance(&svm, &lp2.ata_a) as u128
        + token_balance(&svm, &lp2.ata_b) as u128;
    let lp_principal = 4u128 * MINT_AMOUNT as u128;
    assert!(
        lp_total_now > lp_principal,
        "LPs should have earned fees: now={lp_total_now}, principal={}",
        lp_principal
    );

    let trader_total_now: u128 = token_balance(&svm, &alice.ata_a) as u128
        + token_balance(&svm, &alice.ata_b) as u128
        + token_balance(&svm, &bob.ata_a) as u128
        + token_balance(&svm, &bob.ata_b) as u128;
    let trader_principal = 600_000_000u128;
    assert!(
        trader_total_now < trader_principal,
        "traders should have paid fees: now={trader_total_now}, principal={}",
        trader_principal
    );
}
