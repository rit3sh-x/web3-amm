import { describe, expect, it } from "vitest";
import {
    AmmAccounts,
    MINT_AMOUNT,
    airdropToUser,
    freshPool,
    sendTransaction,
} from "./utils";
import { Keypair, Transaction } from "@solana/web3.js";

describe("admin", () => {
    it("lock blocks operations", async () => {
        const amm = await freshPool(300);

        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);

        await sendTransaction(
            lp.kp,
            new Transaction().add(
                await amm.depositIx(lp, 300_000_000, 300_000_000, 300_000_000)
            )
        );

        await sendTransaction(
            amm.initializer,
            new Transaction().add(
                await amm.setLockedIx(amm.initializer.publicKey, true)
            )
        );

        const cfg = await amm.ammState();

        expect(cfg.locked).toBe(true);

        await expect(
            sendTransaction(
                lp.kp,
                new Transaction().add(
                    await amm.swapIx(lp, { atoB: {} }, 1000, 1)
                )
            )
        ).rejects.toThrow();
    });

    it("non authority cannot lock", async () => {
        const amm = await freshPool(300);

        const attacker = Keypair.generate();

        await airdropToUser(attacker.publicKey);

        await expect(
            sendTransaction(
                attacker,
                new Transaction().add(
                    await amm.setLockedIx(attacker.publicKey, true)
                )
            )
        ).rejects.toThrow();
    });

    it("set_locked_on_authorityless_pool_fails", async () => {
        const amm = await AmmAccounts.new();

        await sendTransaction(
            amm.initializer,
            new Transaction().add(await amm.initIx(300, null))
        );

        await expect(
            sendTransaction(
                amm.initializer,
                new Transaction().add(
                    await amm.setLockedIx(amm.initializer.publicKey, true)
                )
            )
        ).rejects.toThrow();
    });

    it("set_fee updates fee", async () => {
        const amm = await freshPool(300);
        expect((await amm.ammState()).fee).toBe(300);

        await sendTransaction(
            amm.initializer,
            new Transaction().add(
                await amm.setFeeIx(amm.initializer.publicKey, 50)
            )
        );

        expect((await amm.ammState()).fee).toBe(50);
    });

    it("set_fee rejects non authority", async () => {
        const amm = await freshPool(300);
        const attacker = Keypair.generate();
        await airdropToUser(attacker.publicKey);

        await expect(
            sendTransaction(
                attacker,
                new Transaction().add(
                    await amm.setFeeIx(attacker.publicKey, 100)
                )
            )
        ).rejects.toThrow();

        expect((await amm.ammState()).fee).toBe(300);
    });

    it("set_fee rejects at or above max", async () => {
        const amm = await freshPool(300);

        await expect(
            sendTransaction(
                amm.initializer,
                new Transaction().add(
                    await amm.setFeeIx(amm.initializer.publicKey, 10_000)
                )
            )
        ).rejects.toThrow();

        expect((await amm.ammState()).fee).toBe(300);
    });

    it("set_fee rejects authorityless pool", async () => {
        const amm = await AmmAccounts.new();
        await sendTransaction(
            amm.initializer,
            new Transaction().add(await amm.initIx(300, null))
        );

        await expect(
            sendTransaction(
                amm.initializer,
                new Transaction().add(
                    await amm.setFeeIx(amm.initializer.publicKey, 100)
                )
            )
        ).rejects.toThrow();
    });

    it("set_fee applies to next swap", async () => {
        const amm = await freshPool(300);
        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const trader = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);

        const l = 500_000_000;
        await sendTransaction(
            lp.kp,
            new Transaction().add(await amm.depositIx(lp, l, l, l))
        );

        const amountIn = 10_000_000;
        await sendTransaction(
            trader.kp,
            new Transaction().add(
                await amm.swapIx(trader, { atoB: {} }, amountIn, 1)
            )
        );

        await sendTransaction(
            amm.initializer,
            new Transaction().add(
                await amm.setFeeIx(amm.initializer.publicKey, 900)
            )
        );
        expect((await amm.ammState()).fee).toBe(900);

        await sendTransaction(
            trader.kp,
            new Transaction().add(
                await amm.swapIx(trader, { atoB: {} }, amountIn, 1)
            )
        );
    });

    it("set_authority transfers to new keypair", async () => {
        const amm = await freshPool(300);
        const newAuthority = Keypair.generate();
        await airdropToUser(newAuthority.publicKey);

        await sendTransaction(
            amm.initializer,
            new Transaction().add(
                await amm.setAuthorityIx(
                    amm.initializer.publicKey,
                    newAuthority.publicKey
                )
            )
        );
        expect((await amm.ammState()).authority?.toBase58()).toBe(
            newAuthority.publicKey.toBase58()
        );

        await sendTransaction(
            newAuthority,
            new Transaction().add(
                await amm.setFeeIx(newAuthority.publicKey, 50)
            )
        );
        expect((await amm.ammState()).fee).toBe(50);

        await expect(
            sendTransaction(
                amm.initializer,
                new Transaction().add(
                    await amm.setLockedIx(amm.initializer.publicKey, true)
                )
            )
        ).rejects.toThrow();
    });

    it("set_authority rejects non authority", async () => {
        const amm = await freshPool(300);
        const attacker = Keypair.generate();
        await airdropToUser(attacker.publicKey);

        await expect(
            sendTransaction(
                attacker,
                new Transaction().add(
                    await amm.setAuthorityIx(
                        attacker.publicKey,
                        attacker.publicKey
                    )
                )
            )
        ).rejects.toThrow();

        expect((await amm.ammState()).authority?.toBase58()).toBe(
            amm.initializer.publicKey.toBase58()
        );
    });

    it("set_authority renounces when None", async () => {
        const amm = await freshPool(300);

        await sendTransaction(
            amm.initializer,
            new Transaction().add(
                await amm.setAuthorityIx(amm.initializer.publicKey, null)
            )
        );
        expect((await amm.ammState()).authority).toBeNull();

        await expect(
            sendTransaction(
                amm.initializer,
                new Transaction().add(
                    await amm.setFeeIx(amm.initializer.publicKey, 50)
                )
            )
        ).rejects.toThrow();

        await expect(
            sendTransaction(
                amm.initializer,
                new Transaction().add(
                    await amm.setLockedIx(amm.initializer.publicKey, true)
                )
            )
        ).rejects.toThrow();
    });

    it("set_authority rejects authorityless pool", async () => {
        const amm = await AmmAccounts.new();
        await sendTransaction(
            amm.initializer,
            new Transaction().add(await amm.initIx(300, null))
        );

        await expect(
            sendTransaction(
                amm.initializer,
                new Transaction().add(
                    await amm.setAuthorityIx(
                        amm.initializer.publicKey,
                        amm.initializer.publicKey
                    )
                )
            )
        ).rejects.toThrow();
    });
});
