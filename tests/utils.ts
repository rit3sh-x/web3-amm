import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import { Amm } from "@contracts/amm";
import {
    ASSOCIATED_TOKEN_PROGRAM_ID,
    createAssociatedTokenAccountInstruction,
    createInitializeMintInstruction,
    createMintToInstruction,
    getAccount,
    getAssociatedTokenAddressSync,
    getMint,
    MINT_SIZE,
    TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {
    Commitment,
    Keypair,
    LAMPORTS_PER_SOL,
    PublicKey,
    SystemProgram,
    Transaction,
} from "@solana/web3.js";
import { randomBytes } from "crypto";
import BN from "bn.js";

export const commitment: Commitment = "confirmed";

export const INITIAL_USER_LAMPORTS = 5 * LAMPORTS_PER_SOL;
export const MINT_AMOUNT = 1_000_000_000;
export const MINT_DECIMALS = 6;

export const CONFIG_SEED = Buffer.from("config");
export const LP_SEED = Buffer.from("lp");

export type Config = anchor.IdlAccounts<Amm>["config"];
export type SwapDirection = anchor.IdlTypes<Amm>["swapDirection"];

export type Trade = [User, SwapDirection, number];

export const provider = anchor.AnchorProvider.env();

anchor.setProvider(provider);

export const program = anchor.workspace.amm as Program<Amm>;

export const confirmTx = async (signature: string) => {
    const latestBlockhash = await provider.connection.getLatestBlockhash();

    await provider.connection.confirmTransaction(
        {
            signature,
            ...latestBlockhash,
        },
        commitment
    );
};

export const generateSeed = (): Buffer => {
    return randomBytes(32);
};

export const sendTransaction = async (signer: Keypair, tx: Transaction) => {
    tx.feePayer = signer.publicKey;

    const latestBlockhash = await provider.connection.getLatestBlockhash();

    tx.recentBlockhash = latestBlockhash.blockhash;

    tx.sign(signer);

    const signature = await provider.connection.sendRawTransaction(
        tx.serialize()
    );

    await confirmTx(signature);

    return signature;
};

export const airdropToUser = async (user: PublicKey) => {
    const sig = await provider.connection.requestAirdrop(
        user,
        INITIAL_USER_LAMPORTS
    );

    await confirmTx(sig);
};

export const createMint = async (authority: Keypair): Promise<PublicKey> => {
    const mint = Keypair.generate();

    const lamports =
        await provider.connection.getMinimumBalanceForRentExemption(MINT_SIZE);

    const tx = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: authority.publicKey,
            newAccountPubkey: mint.publicKey,
            space: MINT_SIZE,
            lamports,
            programId: TOKEN_PROGRAM_ID,
        }),

        createInitializeMintInstruction(
            mint.publicKey,
            MINT_DECIMALS,
            authority.publicKey,
            null
        )
    );

    tx.feePayer = authority.publicKey;

    const latestBlockhash = await provider.connection.getLatestBlockhash();

    tx.recentBlockhash = latestBlockhash.blockhash;

    tx.partialSign(mint);
    tx.partialSign(authority);

    const sig = await provider.connection.sendRawTransaction(tx.serialize());

    await confirmTx(sig);

    return mint.publicKey;
};

export const createAta = async (
    payer: Keypair,
    mint: PublicKey,
    owner: PublicKey
): Promise<PublicKey> => {
    const ata = getAssociatedTokenAddressSync(mint, owner);

    const tx = new Transaction().add(
        createAssociatedTokenAccountInstruction(
            payer.publicKey,
            ata,
            owner,
            mint
        )
    );

    await sendTransaction(payer, tx);

    return ata;
};

export const mintTokensToAta = async (
    authority: Keypair,
    mint: PublicKey,
    ata: PublicKey,
    amount: number
) => {
    const tx = new Transaction().add(
        createMintToInstruction(mint, ata, authority.publicKey, amount)
    );

    await sendTransaction(authority, tx);
};

export const orderedMints = (
    mintA: PublicKey,
    mintB: PublicKey
): [PublicKey, PublicKey] => {
    return mintA.toBuffer().compare(mintB.toBuffer()) < 0
        ? [mintA, mintB]
        : [mintB, mintA];
};

export const mintPair = async (
    authority: Keypair
): Promise<[PublicKey, PublicKey]> => {
    const m1 = await createMint(authority);
    const m2 = await createMint(authority);

    return orderedMints(m1, m2);
};

export const tokenBalance = async (ata: PublicKey): Promise<number> => {
    try {
        const acc = await getAccount(provider.connection, ata);

        return Number(acc.amount);
    } catch {
        return 0;
    }
};

export const mintSupply = async (mint: PublicKey): Promise<number> => {
    const acc = await getMint(provider.connection, mint);

    return Number(acc.supply);
};

export class User {
    constructor(
        public kp: Keypair,
        public ataA: PublicKey,
        public ataB: PublicKey,
        public ataLp: PublicKey
    ) {}

    pubkey() {
        return this.kp.publicKey;
    }
}

export class AmmAccounts {
    initializer: Keypair;
    mintA: PublicKey;
    mintB: PublicKey;
    mintLp: PublicKey;
    vaultA: PublicKey;
    vaultB: PublicKey;
    seed: Buffer;
    config: PublicKey;
    mintedA = 0;
    mintedB = 0;

    private constructor(
        initializer: Keypair,
        mintA: PublicKey,
        mintB: PublicKey,
        mintLp: PublicKey,
        vaultA: PublicKey,
        vaultB: PublicKey,
        seed: Buffer,
        config: PublicKey
    ) {
        this.initializer = initializer;

        this.mintA = mintA;
        this.mintB = mintB;

        this.mintLp = mintLp;

        this.vaultA = vaultA;
        this.vaultB = vaultB;

        this.seed = seed;

        this.config = config;
    }

    static async new() {
        const initializer = Keypair.generate();

        await airdropToUser(initializer.publicKey);

        const [mintA, mintB] = await mintPair(initializer);

        const seed = generateSeed();

        const [config] = PublicKey.findProgramAddressSync(
            [Buffer.from(CONFIG_SEED), seed.subarray(0, 8)],
            program.programId
        );

        const [mintLp] = PublicKey.findProgramAddressSync(
            [Buffer.from(LP_SEED), config.toBuffer()],
            program.programId
        );

        const vaultA = getAssociatedTokenAddressSync(mintA, config, true);

        const vaultB = getAssociatedTokenAddressSync(mintB, config, true);

        return new AmmAccounts(
            initializer,
            mintA,
            mintB,
            mintLp,
            vaultA,
            vaultB,
            seed,
            config
        );
    }

    async newUser(fundA: number, fundB: number) {
        const kp = Keypair.generate();

        await airdropToUser(kp.publicKey);

        const ataA = await createAta(
            this.initializer,
            this.mintA,
            kp.publicKey
        );

        const ataB = await createAta(
            this.initializer,
            this.mintB,
            kp.publicKey
        );

        const ataLp = await createAta(
            this.initializer,
            this.mintLp,
            kp.publicKey
        );

        if (fundA > 0) {
            await mintTokensToAta(this.initializer, this.mintA, ataA, fundA);

            this.mintedA += fundA;
        }

        if (fundB > 0) {
            await mintTokensToAta(this.initializer, this.mintB, ataB, fundB);

            this.mintedB += fundB;
        }

        return new User(kp, ataA, ataB, ataLp);
    }

    async k(): Promise<bigint> {
        const a = await tokenBalance(this.vaultA);

        const b = await tokenBalance(this.vaultB);

        return BigInt(a) * BigInt(b);
    }

    async ammState(): Promise<Config> {
        return await program.account.config.fetch(this.config);
    }

    initIx(fee: number, authority: PublicKey | null) {
        const seedBn = new BN(this.seed.subarray(0, 8), "le");

        return program.methods
            .init(seedBn, fee, authority)
            .accountsStrict({
                config: this.config,
                initializer: this.initializer.publicKey,
                mintLp: this.mintLp,
                mintA: this.mintA,
                mintB: this.mintB,
                vaultA: this.vaultA,
                vaultB: this.vaultB,
                systemProgram: SystemProgram.programId,
                tokenProgram: TOKEN_PROGRAM_ID,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            })
            .instruction();
    }

    depositIx(user: User, amount: number, maxX: number, maxY: number) {
        return program.methods
            .deposit(new BN(amount), new BN(maxX), new BN(maxY))
            .accountsStrict({
                config: this.config,
                mintLp: this.mintLp,
                user: user.pubkey(),
                userA: user.ataA,
                userB: user.ataB,
                userLp: user.ataLp,
                mintA: this.mintA,
                mintB: this.mintB,
                vaultA: this.vaultA,
                vaultB: this.vaultB,
                systemProgram: SystemProgram.programId,
                tokenProgram: TOKEN_PROGRAM_ID,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            })
            .instruction();
    }

    swapIx(user: User, direction: any, amount: number, min: number) {
        return program.methods
            .swap(direction, new BN(amount), new BN(min))
            .accountsStrict({
                config: this.config,
                mintLp: this.mintLp,
                user: user.pubkey(),
                userA: user.ataA,
                userB: user.ataB,
                mintA: this.mintA,
                mintB: this.mintB,
                vaultA: this.vaultA,
                vaultB: this.vaultB,
                tokenProgram: TOKEN_PROGRAM_ID,
            })
            .instruction();
    }

    withdrawIx(user: User, amount: number, minX: number, minY: number) {
        return program.methods
            .withdraw(new BN(amount), new BN(minX), new BN(minY))
            .accountsStrict({
                config: this.config,
                mintLp: this.mintLp,
                user: user.pubkey(),
                userA: user.ataA,
                userB: user.ataB,
                userLp: user.ataLp,
                mintA: this.mintA,
                mintB: this.mintB,
                vaultA: this.vaultA,
                vaultB: this.vaultB,
                systemProgram: SystemProgram.programId,
                tokenProgram: TOKEN_PROGRAM_ID,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            })
            .instruction();
    }

    setLockedIx(authority: PublicKey, locked: boolean) {
        return program.methods
            .setLocked(locked)
            .accountsStrict({
                config: this.config,
                authority,
            })
            .instruction();
    }
}

export const freshPool = async (fee: number) => {
    const amm = await AmmAccounts.new();

    const ix = await amm.initIx(fee, amm.initializer.publicKey);

    const tx = new Transaction().add(ix);

    await sendTransaction(amm.initializer, tx);

    return amm;
};

export const assertTokenConservation = async (
    amm: AmmAccounts,
    users: User[]
) => {
    const usersA = (
        await Promise.all(users.map((u) => tokenBalance(u.ataA)))
    ).reduce((a, b) => a + b, 0);

    const usersB = (
        await Promise.all(users.map((u) => tokenBalance(u.ataB)))
    ).reduce((a, b) => a + b, 0);

    const vaultA = await tokenBalance(amm.vaultA);

    const vaultB = await tokenBalance(amm.vaultB);

    if (usersA + vaultA !== amm.mintedA) {
        throw new Error("token A was created or destroyed");
    }

    if (usersB + vaultB !== amm.mintedB) {
        throw new Error("token B was created or destroyed");
    }
};
