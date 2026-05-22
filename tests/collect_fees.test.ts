import { describe, expect, it } from "vitest";
import {
    MINT_AMOUNT,
    depositHelper,
    freshPool,
    sendTransaction,
    tokenBalance,
} from "./utils";
import { Keypair, Transaction } from "@solana/web3.js";

describe("collect_fees", () => {
    it("collect_fees after swap pays LP fees", async () => {
        const amm = await freshPool(300);
        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const trader = await amm.newUser(100_000_000, 100_000_000);

        const l = 300_000_000;
        await depositHelper(amm, lp, l);

        await sendTransaction(
            trader.kp,
            new Transaction().add(
                await amm.swapIx(trader, { atoB: {} }, 10_000_000, 1)
            )
        );
        await sendTransaction(
            trader.kp,
            new Transaction().add(
                await amm.swapIx(trader, { btoA: {} }, 10_000_000, 1)
            )
        );

        const aBefore = await tokenBalance(lp.ataA);
        const bBefore = await tokenBalance(lp.ataB);

        await sendTransaction(
            lp.kp,
            new Transaction().add(await amm.collectFeesIx(lp))
        );

        const aAfter = await tokenBalance(lp.ataA);
        const bAfter = await tokenBalance(lp.ataB);

        expect(aAfter).toBeGreaterThan(aBefore);
        expect(bAfter).toBeGreaterThan(bBefore);

        const pos = await amm.positionState(lp.pubkey());
        expect(pos!.feeOwedA.toString()).toBe("0");
        expect(pos!.feeOwedB.toString()).toBe("0");
        expect(pos!.liquidity.toString()).toBe(l.toString());
    });

    it("collect_fees with nothing owed fails", async () => {
        const amm = await freshPool(300);
        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);

        await depositHelper(amm, lp, 200_000_000);

        await expect(
            sendTransaction(
                lp.kp,
                new Transaction().add(await amm.collectFeesIx(lp))
            )
        ).rejects.toThrow();
    });

    it("late LP does not inherit prior fees", async () => {
        const amm = await freshPool(300);
        const early = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const late = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const trader = await amm.newUser(200_000_000, 200_000_000);

        const l = 300_000_000;
        await depositHelper(amm, early, l);

        for (let i = 0; i < 4; i++) {
            await sendTransaction(
                trader.kp,
                new Transaction().add(
                    await amm.swapIx(trader, { atoB: {} }, 8_000_000, 1)
                )
            );
            await sendTransaction(
                trader.kp,
                new Transaction().add(
                    await amm.swapIx(trader, { btoA: {} }, 8_000_000, 1)
                )
            );
        }

        const cfgAtJoin = await amm.ammState();
        expect(BigInt(cfgAtJoin.feeGrowthA.toString())).toBeGreaterThan(0n);
        expect(BigInt(cfgAtJoin.feeGrowthB.toString())).toBeGreaterThan(0n);

        await depositHelper(amm, late, l);

        const pos = await amm.positionState(late.pubkey());
        expect(pos!.feeGrowthSnapshotA.toString()).toBe(
            cfgAtJoin.feeGrowthA.toString()
        );
        expect(pos!.feeGrowthSnapshotB.toString()).toBe(
            cfgAtJoin.feeGrowthB.toString()
        );

        await expect(
            sendTransaction(
                late.kp,
                new Transaction().add(await amm.collectFeesIx(late))
            )
        ).rejects.toThrow();
    });

    it("late LP earns only from post-join swaps", async () => {
        const amm = await freshPool(300);
        const early = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const late = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const trader = await amm.newUser(400_000_000, 400_000_000);

        const l = 300_000_000;
        await depositHelper(amm, early, l);

        for (let i = 0; i < 4; i++) {
            await sendTransaction(
                trader.kp,
                new Transaction().add(
                    await amm.swapIx(trader, { atoB: {} }, 8_000_000, 1)
                )
            );
            await sendTransaction(
                trader.kp,
                new Transaction().add(
                    await amm.swapIx(trader, { btoA: {} }, 8_000_000, 1)
                )
            );
        }

        await depositHelper(amm, late, l);

        const earlyABefore = await tokenBalance(early.ataA);
        const earlyBBefore = await tokenBalance(early.ataB);
        const lateABefore = await tokenBalance(late.ataA);
        const lateBBefore = await tokenBalance(late.ataB);

        for (let i = 0; i < 4; i++) {
            await sendTransaction(
                trader.kp,
                new Transaction().add(
                    await amm.swapIx(trader, { atoB: {} }, 8_000_000, 1)
                )
            );
            await sendTransaction(
                trader.kp,
                new Transaction().add(
                    await amm.swapIx(trader, { btoA: {} }, 8_000_000, 1)
                )
            );
        }

        await sendTransaction(
            early.kp,
            new Transaction().add(await amm.collectFeesIx(early))
        );
        await sendTransaction(
            late.kp,
            new Transaction().add(await amm.collectFeesIx(late))
        );

        const earlyGainA = (await tokenBalance(early.ataA)) - earlyABefore;
        const lateGainA = (await tokenBalance(late.ataA)) - lateABefore;
        const earlyGainB = (await tokenBalance(early.ataB)) - earlyBBefore;
        const lateGainB = (await tokenBalance(late.ataB)) - lateBBefore;

        expect(lateGainA).toBeGreaterThan(0);
        expect(lateGainB).toBeGreaterThan(0);
        expect(earlyGainA).toBeGreaterThan(lateGainA);
        expect(earlyGainB).toBeGreaterThan(lateGainB);
    });

    it("fee change only affects post-change growth", async () => {
        const amm = await freshPool(300);
        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const trader = await amm.newUser(300_000_000, 0);

        const l = 300_000_000;
        await depositHelper(amm, lp, l);

        await sendTransaction(
            trader.kp,
            new Transaction().add(
                await amm.swapIx(trader, { atoB: {} }, 10_000_000, 1)
            )
        );
        const fgLow = BigInt((await amm.ammState()).feeGrowthA.toString());

        await sendTransaction(
            amm.initializer,
            new Transaction().add(
                await amm.setFeeIx(amm.initializer.publicKey, 900)
            )
        );

        await sendTransaction(
            trader.kp,
            new Transaction().add(
                await amm.swapIx(trader, { atoB: {} }, 10_000_000, 1)
            )
        );
        const fgAfterHigh = BigInt(
            (await amm.ammState()).feeGrowthA.toString()
        );

        const deltaLow = fgLow;
        const deltaHigh = fgAfterHigh - fgLow;
        expect(deltaHigh).toBeGreaterThan(deltaLow * 2n);
    });

    it("withdraw auto-claims pending fees", async () => {
        const amm = await freshPool(300);
        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const trader = await amm.newUser(100_000_000, 100_000_000);

        const l = 200_000_000;
        await depositHelper(amm, lp, l);

        for (let i = 0; i < 3; i++) {
            await sendTransaction(
                trader.kp,
                new Transaction().add(
                    await amm.swapIx(trader, { atoB: {} }, 6_000_000, 1)
                )
            );
            await sendTransaction(
                trader.kp,
                new Transaction().add(
                    await amm.swapIx(trader, { btoA: {} }, 6_000_000, 1)
                )
            );
        }

        const aBefore = await tokenBalance(lp.ataA);
        const bBefore = await tokenBalance(lp.ataB);

        await sendTransaction(
            lp.kp,
            new Transaction().add(await amm.withdrawIx(lp, l, 0, 0))
        );

        const aGained = (await tokenBalance(lp.ataA)) - aBefore;
        const bGained = (await tokenBalance(lp.ataB)) - bBefore;

        expect(aGained).toBeGreaterThan(l);
        expect(bGained).toBeGreaterThan(l);

        const pos = await amm.positionState(lp.pubkey());
        expect(pos!.feeOwedA.toString()).toBe("0");
        expect(pos!.feeOwedB.toString()).toBe("0");
        expect(pos!.liquidity.toString()).toBe("0");
    });

    it("second deposit settles before changing liquidity", async () => {
        const amm = await freshPool(300);
        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const trader = await amm.newUser(200_000_000, 200_000_000);

        const l0 = 100_000_000;
        await depositHelper(amm, lp, l0);

        for (let i = 0; i < 3; i++) {
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

        const fgBefore = await amm.ammState();

        await depositHelper(amm, lp, 50_000_000);

        const pos = await amm.positionState(lp.pubkey());
        expect(pos!.feeGrowthSnapshotA.toString()).toBe(
            fgBefore.feeGrowthA.toString()
        );
        expect(pos!.feeGrowthSnapshotB.toString()).toBe(
            fgBefore.feeGrowthB.toString()
        );
        expect(BigInt(pos!.feeOwedA.toString())).toBeGreaterThan(0n);
        expect(BigInt(pos!.feeOwedB.toString())).toBeGreaterThan(0n);
    });

    it("collect_fees rejected when locked", async () => {
        const amm = await freshPool(300);
        const lp = await amm.newUser(MINT_AMOUNT, MINT_AMOUNT);
        const trader = await amm.newUser(100_000_000, 100_000_000);

        const l = 200_000_000;
        await depositHelper(amm, lp, l);

        await sendTransaction(
            trader.kp,
            new Transaction().add(
                await amm.swapIx(trader, { atoB: {} }, 5_000_000, 1)
            )
        );

        await sendTransaction(
            amm.initializer,
            new Transaction().add(
                await amm.setLockedIx(amm.initializer.publicKey, true)
            )
        );

        await expect(
            sendTransaction(
                lp.kp,
                new Transaction().add(await amm.collectFeesIx(lp))
            )
        ).rejects.toThrow();
    });
});
