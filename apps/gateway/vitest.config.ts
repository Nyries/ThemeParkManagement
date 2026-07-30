import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      reporter: ["text", "lcov"],
      include: ["src/**/*.ts"],
      exclude: ["src/generated/**", "src/**/*.spec.ts", "src/**/__tests__/**", "src/index.ts"],
      thresholds: {
        lines: 90,
      },
    },
  },
});
