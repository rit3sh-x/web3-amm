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
    it("swap a to b grows invariant", async () => {
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

        expect(await amm.k()).toBeGreaterThan(kBefore);

        await assertTokenConservation(amm, [lp, trader]);
    });

    it("swap b to a works", async () => {
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

    it("swap_btoa_must_not_decrease_k", async () => {
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
