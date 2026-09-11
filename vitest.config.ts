import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
    // Hosted runners share CPU. Keep DOM-heavy functional tests from competing
    // for all cores; their completion deadline is not a performance assertion.
    fileParallelism: !process.env.CI,
    testTimeout: process.env.CI ? 20_000 : 5_000,
  },
});
