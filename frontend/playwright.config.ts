import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/e2e",
  testMatch: "*.spec.ts",
  fullyParallel: false,
  workers: 1,
  timeout: 30_000,
  use: {
    baseURL: "http://127.0.0.1:3107",
    viewport: { width: 1440, height: 1000 },
    channel: process.env.PLAYWRIGHT_CHANNEL || undefined,
    trace: "retain-on-failure",
  },
  webServer: [
    { command: "node tests/e2e/backend.mjs", url: "http://127.0.0.1:8107/__counts", reuseExistingServer: false },
    {
      command: "npm run build && node tests/e2e/start-frontend.mjs",
      url: "http://127.0.0.1:3107/kraj",
      env: { BACKEND_URL: "http://127.0.0.1:8107" },
      reuseExistingServer: false,
      timeout: 120_000,
    },
  ],
});
