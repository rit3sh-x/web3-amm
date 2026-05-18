import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
    resolve: {
        alias: {
            "@contracts": path.resolve("target/types"),
            "@idl": path.resolve("target/idl"),
        },
    },
    test: {
        include: ["tests/amm_*.ts"],
        testTimeout: 1_000_000,
        hookTimeout: 1_000_000,
        fileParallelism: false,
    },
});
