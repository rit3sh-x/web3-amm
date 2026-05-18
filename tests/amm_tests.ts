import { describe, expect, it } from "vitest";
import {
    AmmAccounts,
    MINT_AMOUNT,
    Trade,
    airdropToUser,
    assertTokenConservation,
    freshPool,
    mintSupply,
    sendTransaction,
    tokenBalance,
} from "./utils";
import { Keypair, Transaction } from "@solana/web3.js";
import BN from "bn.js";

describe("amm", () => {
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
