import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
    resolve: {
        alias: {
            "@contracts": path.resolve(__dirname, "target/types"),
            "@idl": path.resolve(__dirname, "target/idl"),
        },
    },
    test: {
        include: ["tests/*.test.ts"],
        testTimeout: 1_000_000,
        hookTimeout: 1_000_000,
        fileParallelism: false,
    },
});
