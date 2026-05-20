import { describe, expect, it } from "vitest";
import {
    MINT_AMOUNT,
    freshPool,
    mintSupply,
    sendTransaction,
    tokenBalance,
} from "./utils";
import { Transaction } from "@solana/web3.js";

describe("withdraw", () => {
    it("withdraw burns lp and returns reserves", async () => {
        const amm = await freshPool(300);

        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);

        const l = 300_000_000;

        await sendTransaction(
            lp.kp,
            new Transaction().add(await amm.depositIx(lp, l, l, l))
        );

        await sendTransaction(
            lp.kp,
            new Transaction().add(await amm.withdrawIx(lp, l, 0, 0))
        );

        expect(await mintSupply(amm.mintLp)).toBe(0);

        expect(await tokenBalance(lp.ataLp)).toBe(0);
    });

    it("withdraw respects min out", async () => {
        const amm = await freshPool(300);

        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);

        const l = 300_000_000;

        await sendTransaction(
            lp.kp,
            new Transaction().add(await amm.depositIx(lp, l, l, l))
        );

        await expect(
            sendTransaction(
                lp.kp,
                new Transaction().add(
                    await amm.withdrawIx(
                        lp,
                        l,
                        Number.MAX_SAFE_INTEGER,
                        Number.MAX_SAFE_INTEGER
                    )
                )
            )
        ).rejects.toThrow();
    });
});
