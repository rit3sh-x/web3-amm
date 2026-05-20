import { describe, expect, it } from "vitest";
import {
    MINT_AMOUNT,
    Trade,
    assertTokenConservation,
    freshPool,
    mintSupply,
    sendTransaction,
    tokenBalance,
} from "./utils";
import { Transaction } from "@solana/web3.js";

describe("integration", () => {
    it("multi_user_trading_conserves_tokens", async () => {
        const amm = await freshPool(300);

        const lp1 = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const lp2 = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const alice = await amm.newUser(200_000_000, 200_000_000);
        const bob = await amm.newUser(200_000_000, 200_000_000);

        const l0 = 400_000_000;
        await sendTransaction(
            lp1.kp,
            new Transaction().add(await amm.depositIx(lp1, l0, l0, l0))
        );

        const l1 = 150_000_000;
        await sendTransaction(
            lp2.kp,
            new Transaction().add(await amm.depositIx(lp2, l1, l1 * 2, l1 * 2))
        );
        expect(await mintSupply(amm.mintLp)).toBe(l0 + l1);

        const trades: Array<Trade> = [
            [alice, { atoB: {} }, 12_000_000],
            [bob, { btoA: {} }, 8_000_000],
            [alice, { btoA: {} }, 5_000_000],
            [bob, { atoB: {} }, 17_000_000],
            [alice, { atoB: {} }, 3_000_000],
            [bob, { btoA: {} }, 9_000_000],
        ];

        const all = [lp1, lp2, alice, bob];
        for (const t of trades) {
            const [trader, dir, amt] = t;
            await sendTransaction(
                trader.kp,
                new Transaction().add(await amm.swapIx(trader, dir, amt, 1))
            );
            await assertTokenConservation(amm, all);
        }

        const lp1_bal = await tokenBalance(lp1.ataLp);
        const lp2_bal = await tokenBalance(lp2.ataLp);

        await sendTransaction(
            lp1.kp,
            new Transaction().add(await amm.withdrawIx(lp1, lp1_bal, 0, 0))
        );
        await sendTransaction(
            lp2.kp,
            new Transaction().add(await amm.withdrawIx(lp2, lp2_bal, 0, 0))
        );

        expect(await mintSupply(amm.mintLp)).toBe(0);
        await assertTokenConservation(amm, all);
    });

    it("multi_trader_atob_flow_earns_lp_fees", async () => {
        const amm = await freshPool(300);

        const lp1 = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const lp2 = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const alice = await amm.newUser(300_000_000, 0);
        const bob = await amm.newUser(300_000_000, 0);

        const l0 = 400_000_000;
        await sendTransaction(
            lp1.kp,
            new Transaction().add(await amm.depositIx(lp1, l0, l0, l0))
        );

        const l1 = 150_000_000;
        await sendTransaction(
            lp2.kp,
            new Transaction().add(await amm.depositIx(lp2, l1, l1 * 2, l1 * 2))
        );

        const trades = [
            12_000_000, 17_000_000, 3_000_000, 9_000_000, 21_000_000,
        ];
        let k = await amm.k();
        for (const amt of trades) {
            await sendTransaction(
                alice.kp,
                new Transaction().add(
                    await amm.swapIx(alice, { atoB: {} }, amt, 1)
                )
            );
            const kNew = await amm.k();
            expect(kNew).toBeGreaterThanOrEqual(k);
            k = kNew;
        }

        const all = [lp1, lp2, alice, bob];
        await assertTokenConservation(amm, all);

        const lp1_bal = await tokenBalance(lp1.ataLp);
        const lp2_bal = await tokenBalance(lp2.ataLp);

        await sendTransaction(
            lp1.kp,
            new Transaction().add(await amm.withdrawIx(lp1, lp1_bal, 0, 0))
        );
        await sendTransaction(
            lp2.kp,
            new Transaction().add(await amm.withdrawIx(lp2, lp2_bal, 0, 0))
        );

        expect(await mintSupply(amm.mintLp)).toBe(0);
        await assertTokenConservation(amm, all);

        const lp_total_now =
            (await tokenBalance(lp1.ataA)) +
            (await tokenBalance(lp1.ataB)) +
            (await tokenBalance(lp2.ataA)) +
            (await tokenBalance(lp2.ataB));
        const lp_principal = 4 * MINT_AMOUNT;
        expect(lp_total_now).toBeGreaterThan(lp_principal);
    });
});
