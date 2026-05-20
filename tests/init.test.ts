import { describe, expect, it } from "vitest";
import {
    AmmAccounts,
    freshPool,
    mintSupply,
    sendTransaction,
    tokenBalance,
} from "./utils";
import { Transaction } from "@solana/web3.js";
import BN from "bn.js";

describe("init", () => {
    it("init succeeds and sets config", async () => {
        const amm = await freshPool(300);

        const cfg = await amm.ammState();

        expect(cfg.seed.toString()).toBe(
            new BN(amm.seed.subarray(0, 8), "le").toString()
        );

        expect(cfg.fee).toBe(300);

        expect(cfg.mintA.toBase58()).toBe(amm.mintA.toBase58());

        expect(cfg.mintB.toBase58()).toBe(amm.mintB.toBase58());

        expect(cfg.authority?.toBase58()).toBe(
            amm.initializer.publicKey.toBase58()
        );

        expect(cfg.locked).toBe(false);

        expect(await mintSupply(amm.mintLp)).toBe(0);

        expect(await tokenBalance(amm.vaultA)).toBe(0);

        expect(await tokenBalance(amm.vaultB)).toBe(0);
    });

    it("init rejects fee >= 10000", async () => {
        const amm = await AmmAccounts.new();

        await expect(
            sendTransaction(
                amm.initializer,
                new Transaction().add(
                    await amm.initIx(10_000, amm.initializer.publicKey)
                )
            )
        ).rejects.toThrow();
    });

    it("init accepts zero fee", async () => {
        const amm = await AmmAccounts.new();

        await sendTransaction(
            amm.initializer,
            new Transaction().add(
                await amm.initIx(0, amm.initializer.publicKey)
            )
        );

        const cfg = await amm.ammState();

        expect(cfg.fee).toBe(0);
    });
});
