import { defineConfig } from "vitest/config";

export default defineConfig({
    test: {
        include: ["src/main.ts"],
        fileParallelism: false,
        testTimeout: 300_000,
        hookTimeout: 120_000,
        retry: 0,
    },
});
