import { describe, expect, it } from "vitest";
import {
    MINT_AMOUNT,
    assertTokenConservation,
    freshPool,
    sendTransaction,
    tokenBalance,
} from "./utils";
import { Transaction } from "@solana/web3.js";

describe("deposit", () => {
    it("first deposit sets reserves and opens position", async () => {
        const amm = await freshPool(300);

        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);

        const l = 300_000_000;

        await sendTransaction(
            lp.kp,
            new Transaction().add(await amm.depositIx(lp, l, l, l))
        );

        expect(await tokenBalance(amm.vaultA)).toBe(l);
        expect(await tokenBalance(amm.vaultB)).toBe(l);

        const cfg = await amm.ammState();
        expect(cfg.reserveA.toString()).toBe(l.toString());
        expect(cfg.reserveB.toString()).toBe(l.toString());
        expect(cfg.totalLiquidity.toString()).toBe(l.toString());

        const pos = await amm.positionState(lp.pubkey());
        expect(pos).not.toBeNull();
        expect(pos!.owner.toBase58()).toBe(lp.pubkey().toBase58());
        expect(pos!.liquidity.toString()).toBe(l.toString());
        expect(pos!.feeOwedA.toString()).toBe("0");
        expect(pos!.feeOwedB.toString()).toBe("0");

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

        const cfg = await amm.ammState();
        expect(cfg.totalLiquidity.toString()).toBe((l0 + l1).toString());

        const pos2 = await amm.positionState(lp2.pubkey());
        expect(pos2!.liquidity.toString()).toBe(l1.toString());

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

    it("repeat deposit grows the same position", async () => {
        const amm = await freshPool(300);

        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);

        const l0 = 200_000_000;
        await sendTransaction(
            lp.kp,
            new Transaction().add(await amm.depositIx(lp, l0, l0, l0))
        );

        const l1 = 100_000_000;
        await sendTransaction(
            lp.kp,
            new Transaction().add(await amm.depositIx(lp, l1, l1 * 2, l1 * 2))
        );

        const pos = await amm.positionState(lp.pubkey());
        expect(pos!.liquidity.toString()).toBe((l0 + l1).toString());

        const cfg = await amm.ammState();
        expect(cfg.totalLiquidity.toString()).toBe((l0 + l1).toString());
    });
});
