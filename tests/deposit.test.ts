import { describe, expect, it } from "vitest";
import {
    MINT_AMOUNT,
    assertTokenConservation,
    freshPool,
    mintSupply,
    sendTransaction,
    tokenBalance,
} from "./utils";
import { Transaction } from "@solana/web3.js";

describe("deposit", () => {
    it("first deposit sets reserves and mints lp", async () => {
        const amm = await freshPool(300);

        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);

        const l = 300_000_000;

        await sendTransaction(
            lp.kp,
            new Transaction().add(await amm.depositIx(lp, l, l, l))
        );

        expect(await tokenBalance(amm.vaultA)).toBe(l);

        expect(await tokenBalance(amm.vaultB)).toBe(l);

        expect(await mintSupply(amm.mintLp)).toBe(l);

        expect(await tokenBalance(lp.ataLp)).toBe(l);

        await assertTokenConservation(amm, [lp]);
    });

    it("deposit rejects zero amount", async () => {
        const amm = await freshPool(300);

        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);

        await expect(
            sendTransaction(
                lp.kp,
                new Transaction().add(await amm.depositIx(lp, 0, 1, 1))
            )
        ).rejects.toThrow();
    });

    it("second deposit is proportional", async () => {
        const amm = await freshPool(300);

        const lp1 = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);

        const lp2 = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);

        const l0 = 300_000_000;

        await sendTransaction(
            lp1.kp,
            new Transaction().add(await amm.depositIx(lp1, l0, l0, l0))
        );

        const l1 = 100_000_000;

        await sendTransaction(
            lp2.kp,
            new Transaction().add(await amm.depositIx(lp2, l1, l1 * 2, l1 * 2))
        );

        expect(await mintSupply(amm.mintLp)).toBe(l0 + l1);

        expect(await tokenBalance(lp2.ataLp)).toBe(l1);

        await assertTokenConservation(amm, [lp1, lp2]);
    });

    it("deposit respects slippage", async () => {
        const amm = await freshPool(300);

        const lp1 = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);

        const lp2 = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);

        const l0 = 300_000_000;

        await sendTransaction(
            lp1.kp,
            new Transaction().add(await amm.depositIx(lp1, l0, l0, l0))
        );

        await expect(
            sendTransaction(
                lp2.kp,
                new Transaction().add(
                    await amm.depositIx(lp2, 100_000_000, 1, 1)
                )
            )
        ).rejects.toThrow();
    });
});
