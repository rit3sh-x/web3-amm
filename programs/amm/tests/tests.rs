pub mod utils;

use amm::{error::AmmError, SwapDirection};
use solana_keypair::Keypair;
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

#[test]
fn swap_a_to_b_grows_invariant() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let trader = amm.new_user(&mut svm, 100_000_000, 0);

    let l = 300_000_000u64;

    let deposit_ix = amm.deposit_ix(&lp, l, l, l);
    let result = send_instruction(&mut svm, &lp.kp, deposit_ix);
    assert_ok(result);

    let k_before = amm.k(&svm);
    let amount_in = 10_000_000u64;
    let swap_ix = amm.swap_ix(&trader, SwapDirection::AtoB, amount_in, 1);
    let result = send_instruction(&mut svm, &trader.kp, swap_ix);
    assert_ok(result);

    assert_eq!(token_balance(&svm, &trader.ata_a), 100_000_000 - amount_in);
    let got_b = token_balance(&svm, &trader.ata_b);
    assert!(got_b > 0, "trader should receive token B");
    assert_eq!(token_balance(&svm, &amm.vault_a), l + amount_in);
    assert_eq!(token_balance(&svm, &amm.vault_b), l - got_b);

    assert!(
        amm.k(&svm) > k_before,
        "constant product must grow as fees accrue"
    );
    assert_token_conservation(&svm, &amm, &[&lp, &trader]);
}

#[test]
fn swap_b_to_a_direction_works() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let trader = amm.new_user(&mut svm, 0, 100_000_000);

    let l = 300_000_000u64;
    let deposit_ix = amm.deposit_ix(&lp, l, l, l);
    let result = send_instruction(&mut svm, &lp.kp, deposit_ix);
    assert_ok(result);

    let amount_in = 10_000_000u64;
    let swap_ix = amm.swap_ix(&trader, SwapDirection::BtoA, amount_in, 1);
    let result = send_instruction(&mut svm, &trader.kp, swap_ix);
    assert_ok(result);

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
    let deposit_ix = amm.deposit_ix(&lp, l, l, l);
    let result = send_instruction(&mut svm, &lp.kp, deposit_ix);
    assert_ok(result);

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
    let deposit_ix = amm.deposit_ix(&lp, l, l, l);
    let result = send_instruction(&mut svm, &lp.kp, deposit_ix);
    assert_ok(result);

    let swap_ix = amm.swap_ix(&trader, SwapDirection::BtoA, 0, 0);
    let result = send_instruction(&mut svm, &trader.kp, swap_ix);
    assert_error(result, AmmError::InvalidAmount);
}

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

#[test]
fn swap_btoa_must_not_decrease_k() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let trader = amm.new_user(&mut svm, 0, 100_000_000);

    let l = 400_000_000u64;
    let deposit_ix = amm.deposit_ix(&lp, l, l, l);
    let result = send_instruction(&mut svm, &lp.kp, deposit_ix);
    assert_ok(result);

    let k_before = amm.k(&svm);
    let swap_ix = amm.swap_ix(&trader, SwapDirection::BtoA, 8_000_000, 1);
    let result = send_instruction(&mut svm, &trader.kp, swap_ix);
    assert_ok(result);

    let k_after = amm.k(&svm);

    assert!(
        k_after >= k_before,
        "BtoA swap leaked value: k_before={k_before} k_after={k_after} delta={}",
        k_after as i128 - k_before as i128
    );
}
