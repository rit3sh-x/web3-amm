import { describe, expect, it } from "vitest";
import {
    MINT_AMOUNT,
    Trade,
    assertTokenConservation,
    depositHelper,
    freshPool,
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
        const cfg = await amm.ammState();
        expect(cfg.totalLiquidity.toString()).toBe((l0 + l1).toString());

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

        const pos1 = await amm.positionState(lp1.pubkey());
        const pos2 = await amm.positionState(lp2.pubkey());

        await sendTransaction(
            lp1.kp,
            new Transaction().add(
                await amm.withdrawIx(
                    lp1,
                    Number(pos1!.liquidity.toString()),
                    0,
                    0
                )
            )
        );
        await sendTransaction(
            lp2.kp,
            new Transaction().add(
                await amm.withdrawIx(
                    lp2,
                    Number(pos2!.liquidity.toString()),
                    0,
                    0
                )
            )
        );

        const cfgFinal = await amm.ammState();
        expect(cfgFinal.totalLiquidity.toString()).toBe("0");
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

        const cfg = await amm.ammState();
        expect(BigInt(cfg.feeGrowthA.toString())).toBeGreaterThan(0n);

        const all = [lp1, lp2, alice, bob];
        await assertTokenConservation(amm, all);

        const pos1 = await amm.positionState(lp1.pubkey());
        const pos2 = await amm.positionState(lp2.pubkey());

        await sendTransaction(
            lp1.kp,
            new Transaction().add(
                await amm.withdrawIx(
                    lp1,
                    Number(pos1!.liquidity.toString()),
                    0,
                    0
                )
            )
        );
        await sendTransaction(
            lp2.kp,
            new Transaction().add(
                await amm.withdrawIx(
                    lp2,
                    Number(pos2!.liquidity.toString()),
                    0,
                    0
                )
            )
        );

        const cfgFinal = await amm.ammState();
        expect(cfgFinal.totalLiquidity.toString()).toBe("0");
        await assertTokenConservation(amm, all);

        const lp_total_now =
            (await tokenBalance(lp1.ataA)) +
            (await tokenBalance(lp1.ataB)) +
            (await tokenBalance(lp2.ataA)) +
            (await tokenBalance(lp2.ataB));
        const lp_principal = 4 * MINT_AMOUNT;
        expect(lp_total_now).toBeGreaterThan(lp_principal);
    });

    it("fee_share_is_proportional_to_liquidity", async () => {
        const amm = await freshPool(300);

        const lpBig = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const lpSmall = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const trader = await amm.newUser(200_000_000, 200_000_000);

        const bigL = 300_000_000;
        const smallL = 100_000_000;

        await depositHelper(amm, lpBig, bigL);
        await depositHelper(amm, lpSmall, smallL);

        for (let i = 0; i < 6; i++) {
            await sendTransaction(
                trader.kp,
                new Transaction().add(
                    await amm.swapIx(trader, { atoB: {} }, 5_000_000, 1)
                )
            );
            await sendTransaction(
                trader.kp,
                new Transaction().add(
                    await amm.swapIx(trader, { btoA: {} }, 5_000_000, 1)
                )
            );
        }

        await sendTransaction(
            lpBig.kp,
            new Transaction().add(await amm.collectFeesIx(lpBig))
        );
        await sendTransaction(
            lpSmall.kp,
            new Transaction().add(await amm.collectFeesIx(lpSmall))
        );

        const bigA = (await tokenBalance(lpBig.ataA)) - (MINT_AMOUNT - bigL);
        const smallA =
            (await tokenBalance(lpSmall.ataA)) - (MINT_AMOUNT - smallL);
        const bigB = (await tokenBalance(lpBig.ataB)) - (MINT_AMOUNT - bigL);
        const smallB =
            (await tokenBalance(lpSmall.ataB)) - (MINT_AMOUNT - smallL);

        expect(bigA).toBeGreaterThan(0);
        expect(smallA).toBeGreaterThan(0);
        expect(bigB).toBeGreaterThan(0);
        expect(smallB).toBeGreaterThan(0);

        const ratioA = bigA / smallA;
        const ratioB = bigB / smallB;
        const expected = bigL / smallL;

        expect(Math.abs(ratioA - expected)).toBeLessThan(0.05);
        expect(Math.abs(ratioB - expected)).toBeLessThan(0.05);
    });
});
