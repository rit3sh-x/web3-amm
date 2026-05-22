import { describe, expect, it } from "vitest";
import {
    MINT_AMOUNT,
    assertTokenConservation,
    freshPool,
    sendTransaction,
    tokenBalance,
} from "./utils";
import { Transaction } from "@solana/web3.js";

describe("swap", () => {
    it("swap a to b preserves reserve k and accrues fee_growth_a", async () => {
        const amm = await freshPool(300);

        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const trader = await amm.newUser(100_000_000, 0);

        const l = 300_000_000;

        await sendTransaction(
            lp.kp,
            new Transaction().add(await amm.depositIx(lp, l, l, l))
        );

        const kBefore = await amm.k();

        await sendTransaction(
            trader.kp,
            new Transaction().add(
                await amm.swapIx(trader, { atoB: {} }, 10_000_000, 1)
            )
        );

        const cfg = await amm.ammState();
        expect(await amm.k()).toBeGreaterThanOrEqual(kBefore);
        expect(BigInt(cfg.feeGrowthA.toString())).toBeGreaterThan(0n);
        expect(cfg.feeGrowthB.toString()).toBe("0");

        await assertTokenConservation(amm, [lp, trader]);
    });

    it("swap b to a advances fee_growth_b only", async () => {
        const amm = await freshPool(300);

        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const trader = await amm.newUser(0, 100_000_000);

        await sendTransaction(
            lp.kp,
            new Transaction().add(
                await amm.depositIx(lp, 300_000_000, 300_000_000, 300_000_000)
            )
        );

        await sendTransaction(
            trader.kp,
            new Transaction().add(
                await amm.swapIx(trader, { btoA: {} }, 10_000_000, 1)
            )
        );

        const cfg = await amm.ammState();
        expect(BigInt(cfg.feeGrowthB.toString())).toBeGreaterThan(0n);
        expect(cfg.feeGrowthA.toString()).toBe("0");
        expect(await tokenBalance(trader.ataA)).toBeGreaterThan(0);
    });

    it("swap rejects zero amount", async () => {
        const amm = await freshPool(300);

        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const trader = await amm.newUser(100_000_000, 0);

        await sendTransaction(
            lp.kp,
            new Transaction().add(
                await amm.depositIx(lp, 300_000_000, 300_000_000, 300_000_000)
            )
        );

        await expect(
            sendTransaction(
                trader.kp,
                new Transaction().add(
                    await amm.swapIx(trader, { atoB: {} }, 0, 0)
                )
            )
        ).rejects.toThrow();
    });

    it("swap respects min out", async () => {
        const amm = await freshPool(300);

        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const trader = await amm.newUser(100_000_000, 0);

        await sendTransaction(
            lp.kp,
            new Transaction().add(
                await amm.depositIx(lp, 300_000_000, 300_000_000, 300_000_000)
            )
        );

        await expect(
            sendTransaction(
                trader.kp,
                new Transaction().add(
                    await amm.swapIx(
                        trader,
                        { atoB: {} },
                        10_000_000,
                        Number.MAX_SAFE_INTEGER
                    )
                )
            )
        ).rejects.toThrow();
    });

    it("swap rejects without liquidity", async () => {
        const amm = await freshPool(300);
        const trader = await amm.newUser(100_000_000, 100_000_000);

        await expect(
            sendTransaction(
                trader.kp,
                new Transaction().add(
                    await amm.swapIx(trader, { atoB: {} }, 1000, 1)
                )
            )
        ).rejects.toThrow();
    });

    it("swap btoa must not decrease k", async () => {
        const amm = await freshPool(300);
        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const trader = await amm.newUser(0, 100_000_000);

        const l = 400_000_000;
        await sendTransaction(
            lp.kp,
            new Transaction().add(await amm.depositIx(lp, l, l, l))
        );

        const kBefore = await amm.k();
        await sendTransaction(
            trader.kp,
            new Transaction().add(
                await amm.swapIx(trader, { btoA: {} }, 8_000_000, 1)
            )
        );
        const kAfter = await amm.k();

        expect(kAfter).toBeGreaterThanOrEqual(kBefore);
    });
});
