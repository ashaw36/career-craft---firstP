import { defineConfig } from "vitest/config";

// A developer and CI worker can run test gates at the same time.  V8 uses a
// shared `.tmp` directory below reportsDirectory, so give every process an
// isolated directory instead of allowing one run to delete another run's data.
const coverageRunDirectory = `coverage/run-${Date.now()}-${Math.random().toString(36).slice(2)}`;

export default defineConfig({
  clearScreen: false,
  server: { strictPort: true, port: 1420, host: "127.0.0.1", hmr: false },
  envPrefix: ["VITE_", "TAURI_"],
  // The desktop shell imports Tauri APIs at runtime. Avoid Vite's eager
  // dependency crawler on Windows; it can leave the dev HTTP listener alive
  // while blocking every request when the optimizer worker stalls.
  optimizeDeps: { noDiscovery: true },
  build: { target: "es2021", sourcemap: true, emptyOutDir: true, assetsDir: "assets" },
  test: {
    environment: "jsdom",
    include: ["tests/frontend/**/*.test.ts"],
    coverage: {
      provider: "v8",
      reportsDirectory: coverageRunDirectory,
      reporter: ["text", "json-summary"],
      exclude: ["src/api/contract-type-gates.ts"],
      include: ["src/api/**/*.ts", "src/shared/state/**/*.ts", "src/app.ts", "src/features/pages.ts", "src/features/actions/**/*.ts"]
    }
  }
});
