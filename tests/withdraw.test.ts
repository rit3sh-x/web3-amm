import { describe, expect, it } from "vitest";
import {
    MINT_AMOUNT,
    assertTokenConservation,
    freshPool,
    sendTransaction,
    tokenBalance,
} from "./utils";
import { Transaction } from "@solana/web3.js";

describe("withdraw", () => {
    it("withdraw returns reserves and zeros position liquidity", async () => {
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

        const cfg = await amm.ammState();
        expect(cfg.totalLiquidity.toString()).toBe("0");
        expect(cfg.reserveA.toString()).toBe("0");
        expect(cfg.reserveB.toString()).toBe("0");

        const pos = await amm.positionState(lp.pubkey());
        expect(pos!.liquidity.toString()).toBe("0");

        expect(await tokenBalance(lp.ataA)).toBe(MINT_AMOUNT);
        expect(await tokenBalance(lp.ataB)).toBe(MINT_AMOUNT);
        await assertTokenConservation(amm, [lp]);
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

    it("withdraw rejects more than position", async () => {
        const amm = await freshPool(300);

        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);

        const l = 200_000_000;
        await sendTransaction(
            lp.kp,
            new Transaction().add(await amm.depositIx(lp, l, l, l))
        );

        await expect(
            sendTransaction(
                lp.kp,
                new Transaction().add(await amm.withdrawIx(lp, l + 1, 0, 0))
            )
        ).rejects.toThrow();
    });

    it("partial withdraw keeps position open", async () => {
        const amm = await freshPool(300);

        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);

        const l = 400_000_000;
        await sendTransaction(
            lp.kp,
            new Transaction().add(await amm.depositIx(lp, l, l, l))
        );

        const half = l / 2;
        await sendTransaction(
            lp.kp,
            new Transaction().add(await amm.withdrawIx(lp, half, 0, 0))
        );

        const pos = await amm.positionState(lp.pubkey());
        expect(pos!.liquidity.toString()).toBe((l - half).toString());

        const cfg = await amm.ammState();
        expect(cfg.totalLiquidity.toString()).toBe((l - half).toString());
    });
});
