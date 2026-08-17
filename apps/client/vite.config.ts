import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "path"
import { readFileSync } from "fs";

// TPM-185: VERSION at the repo root is the single source of truth, shared with the engine.
const appVersion = readFileSync(path.resolve(__dirname, "../../VERSION"), "utf-8").trim();

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  define: {
    __APP_VERSION__: JSON.stringify(appVersion),
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src")
    }
  },
  server: {
    watch: {
      usePolling: true
    }
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./vitest-setup.ts"],
    coverage: {
      provider: "v8",
      reporter: ["text", "lcov"],
      include: ["src/**/*.{ts,tsx}"],
      exclude: [
        "src/**/*.spec.{ts,tsx}",
        "src/**/__tests__/**",
        "src/main.tsx",
        "src/App.tsx",
        "src/components/ui/**",
      ],
      thresholds: {
        lines: 90,
      },
    },
  },
});
