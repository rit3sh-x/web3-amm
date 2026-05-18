import { defineConfig } from "vitest/config";
import { fileURLToPath } from "url";
import path from "path";

const root = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
    resolve: {
        alias: {
            "@contracts": path.resolve(root, "target/types"),
            "@idl": path.resolve(root, "target/idl"),
        },
    },
    test: {
        include: ["tests/amm_*.ts"],
        testTimeout: 1_000_000,
        hookTimeout: 1_000_000,
        fileParallelism: false,
    },
});
