pub mod utils;

use amm::{error::AmmError, SwapDirection};
use solana_signer::Signer;
use utils::*;

fn deposit(svm: &mut litesvm::LiteSVM, amm: &mut AmmAccounts, lp: &User, amount: u64) {
    let total = amm_state(svm, &amm.config).total_liquidity;
    let cap = if total == 0 {
        amount
    } else {
        amount.saturating_mul(4)
    };
    assert_ok(send_instruction(
        svm,
        &lp.kp,
        amm.deposit_ix(lp, amount, cap, cap),
    ));
}

fn trade(svm: &mut litesvm::LiteSVM, amm: &AmmAccounts, t: &User, dir: SwapDirection, amt: u64) {
    assert_ok(send_instruction(svm, &t.kp, amm.swap_ix(t, dir, amt, 1)));
}

#[test]
fn collect_fees_after_swap_pays_lp_fees() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let trader = amm.new_user(&mut svm, 100_000_000, 100_000_000);

    let l = 300_000_000u64;
    deposit(&mut svm, &mut amm, &lp, l);

    trade(&mut svm, &amm, &trader, SwapDirection::AtoB, 10_000_000);
    trade(&mut svm, &amm, &trader, SwapDirection::BtoA, 10_000_000);

    let a_before = token_balance(&svm, &lp.ata_a);
    let b_before = token_balance(&svm, &lp.ata_b);

    assert_ok(send_instruction(&mut svm, &lp.kp, amm.collect_fees_ix(&lp)));

    let a_after = token_balance(&svm, &lp.ata_a);
    let b_after = token_balance(&svm, &lp.ata_b);

    assert!(a_after > a_before, "LP should receive token A fees");
    assert!(b_after > b_before, "LP should receive token B fees");

    let pos = position_state(&svm, &lp.position).unwrap();
    assert_eq!(pos.fee_owed_a, 0, "owed should be zeroed after claim");
    assert_eq!(pos.fee_owed_b, 0);
    assert_eq!(
        pos.liquidity, l,
        "principal liquidity must not change on collect_fees"
    );
}

#[test]
fn collect_fees_with_nothing_owed_fails() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);

    let l = 200_000_000u64;
    deposit(&mut svm, &mut amm, &lp, l);

    let result = send_instruction(&mut svm, &lp.kp, amm.collect_fees_ix(&lp));
    assert_error(result, AmmError::InvalidAmount);
}

#[test]
fn late_lp_does_not_inherit_prior_fees() {
    let (mut svm, mut amm) = fresh_pool(300);
    let early = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let late = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let trader = amm.new_user(&mut svm, 200_000_000, 200_000_000);

    let l = 300_000_000u64;
    deposit(&mut svm, &mut amm, &early, l);

    for _ in 0..4 {
        trade(&mut svm, &amm, &trader, SwapDirection::AtoB, 8_000_000);
        trade(&mut svm, &amm, &trader, SwapDirection::BtoA, 8_000_000);
    }

    let fg_a_at_join = amm_state(&svm, &amm.config).fee_growth_a;
    let fg_b_at_join = amm_state(&svm, &amm.config).fee_growth_b;
    assert!(fg_a_at_join > 0 && fg_b_at_join > 0);

    deposit(&mut svm, &mut amm, &late, l);

    let pos = position_state(&svm, &late.position).unwrap();
    assert_eq!(pos.fee_growth_snapshot_a, fg_a_at_join);
    assert_eq!(pos.fee_growth_snapshot_b, fg_b_at_join);

    let result = send_instruction(&mut svm, &late.kp, amm.collect_fees_ix(&late));
    assert_error(result, AmmError::InvalidAmount);
}

#[test]
fn late_lp_earns_only_from_post_join_swaps() {
    let (mut svm, mut amm) = fresh_pool(300);
    let early = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let late = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let trader = amm.new_user(&mut svm, 400_000_000, 400_000_000);

    let l = 300_000_000u64;
    deposit(&mut svm, &mut amm, &early, l);

    for _ in 0..4 {
        trade(&mut svm, &amm, &trader, SwapDirection::AtoB, 8_000_000);
        trade(&mut svm, &amm, &trader, SwapDirection::BtoA, 8_000_000);
    }

    deposit(&mut svm, &mut amm, &late, l);

    let early_a_before = token_balance(&svm, &early.ata_a);
    let early_b_before = token_balance(&svm, &early.ata_b);
    let late_a_before = token_balance(&svm, &late.ata_a);
    let late_b_before = token_balance(&svm, &late.ata_b);

    for _ in 0..4 {
        trade(&mut svm, &amm, &trader, SwapDirection::AtoB, 8_000_000);
        trade(&mut svm, &amm, &trader, SwapDirection::BtoA, 8_000_000);
    }

    assert_ok(send_instruction(
        &mut svm,
        &early.kp,
        amm.collect_fees_ix(&early),
    ));
    assert_ok(send_instruction(
        &mut svm,
        &late.kp,
        amm.collect_fees_ix(&late),
    ));

    let early_a_after = token_balance(&svm, &early.ata_a);
    let early_b_after = token_balance(&svm, &early.ata_b);
    let late_a_after = token_balance(&svm, &late.ata_a);
    let late_b_after = token_balance(&svm, &late.ata_b);

    let early_gain_a = early_a_after.checked_sub(early_a_before).unwrap();

    let late_gain_a = late_a_after.checked_sub(late_a_before).unwrap();

    let early_gain_b = early_b_after.checked_sub(early_b_before).unwrap();

    let late_gain_b = late_b_after.checked_sub(late_b_before).unwrap();

    assert!(late_gain_a > 0 && late_gain_b > 0);
    assert!(
        early_gain_a > late_gain_a,
        "early LP must outearn late LP on A (early earned from all swaps): early={early_gain_a} late={late_gain_a}"
    );
    assert!(
        early_gain_b > late_gain_b,
        "early LP must outearn late LP on B: early={early_gain_b} late={late_gain_b}"
    );
}

#[test]
fn fee_change_only_affects_post_change_growth() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let trader = amm.new_user(&mut svm, 300_000_000, 0);

    let l = 300_000_000u64;
    deposit(&mut svm, &mut amm, &lp, l);

    trade(&mut svm, &amm, &trader, SwapDirection::AtoB, 10_000_000);
    let fg_low = amm_state(&svm, &amm.config).fee_growth_a;

    assert_ok(send_instruction(
        &mut svm,
        &amm.initializer,
        amm.set_fee_ix(amm.initializer.pubkey(), 900),
    ));

    trade(&mut svm, &amm, &trader, SwapDirection::AtoB, 10_000_000);
    let fg_after_high = amm_state(&svm, &amm.config).fee_growth_a;

    let delta_low = fg_low;
    let delta_high = fg_after_high.checked_sub(fg_low).unwrap();
    assert!(
        delta_high > delta_low * 2,
        "raising fee 3x->9x should noticeably increase per-swap accumulator: delta_low={delta_low} delta_high={delta_high}"
    );
}

#[test]
fn withdraw_auto_claims_pending_fees() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let trader = amm.new_user(&mut svm, 100_000_000, 100_000_000);

    let l = 200_000_000u64;
    deposit(&mut svm, &mut amm, &lp, l);

    for _ in 0..3 {
        trade(&mut svm, &amm, &trader, SwapDirection::AtoB, 6_000_000);
        trade(&mut svm, &amm, &trader, SwapDirection::BtoA, 6_000_000);
    }

    let a_before = token_balance(&svm, &lp.ata_a);
    let b_before = token_balance(&svm, &lp.ata_b);

    assert_ok(send_instruction(
        &mut svm,
        &lp.kp,
        amm.withdraw_ix(&lp, l, 0, 0),
    ));

    let a_gained = token_balance(&svm, &lp.ata_a)
        .checked_sub(a_before)
        .unwrap();

    let b_gained = token_balance(&svm, &lp.ata_b)
        .checked_sub(b_before)
        .unwrap();

    assert!(a_gained > l, "got principal + fee for A");
    assert!(b_gained > l, "got principal + fee for B");

    let pos = position_state(&svm, &lp.position).unwrap();
    assert_eq!(pos.fee_owed_a, 0);
    assert_eq!(pos.fee_owed_b, 0);
    assert_eq!(pos.liquidity, 0);
}

#[test]
fn second_deposit_settles_before_changing_liquidity() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let trader = amm.new_user(&mut svm, 200_000_000, 200_000_000);

    let l0 = 100_000_000u64;
    deposit(&mut svm, &mut amm, &lp, l0);

    for _ in 0..3 {
        trade(&mut svm, &amm, &trader, SwapDirection::AtoB, 5_000_000);
        trade(&mut svm, &amm, &trader, SwapDirection::BtoA, 5_000_000);
    }

    let fg_before = amm_state(&svm, &amm.config);

    deposit(&mut svm, &mut amm, &lp, 50_000_000);

    let pos = position_state(&svm, &lp.position).unwrap();
    assert_eq!(pos.fee_growth_snapshot_a, fg_before.fee_growth_a);
    assert_eq!(pos.fee_growth_snapshot_b, fg_before.fee_growth_b);
    assert!(
        pos.fee_owed_a > 0,
        "pending A fees must be crystallized before liquidity grows"
    );
    assert!(pos.fee_owed_b > 0);
}

#[test]
fn collect_fees_rejected_when_locked() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let trader = amm.new_user(&mut svm, 100_000_000, 100_000_000);

    let l = 200_000_000u64;
    deposit(&mut svm, &mut amm, &lp, l);

    trade(&mut svm, &amm, &trader, SwapDirection::AtoB, 5_000_000);

    assert_ok(send_instruction(
        &mut svm,
        &amm.initializer,
        amm.set_locked_ix(amm.initializer.pubkey(), true),
    ));

    let result = send_instruction(&mut svm, &lp.kp, amm.collect_fees_ix(&lp));
    assert_error(result, AmmError::PoolLocked);
}

#[test]
fn collect_fees_rejects_non_owner() {
    let (mut svm, mut amm) = fresh_pool(300);
    let lp = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let attacker = amm.new_user(&mut svm, MINT_AMOUNT, MINT_AMOUNT);
    let trader = amm.new_user(&mut svm, 100_000_000, 100_000_000);

    let l = 200_000_000u64;
    deposit(&mut svm, &mut amm, &lp, l);

    trade(&mut svm, &amm, &trader, SwapDirection::AtoB, 5_000_000);

    let mut ix = amm.collect_fees_ix(&attacker);
    for meta in &mut ix.accounts {
        if meta.pubkey == attacker.position {
            meta.pubkey = lp.position;
        }
    }
    let result = send_instruction(&mut svm, &attacker.kp, ix);
    assert!(result.is_err(), "non-owner collect must fail");
}
