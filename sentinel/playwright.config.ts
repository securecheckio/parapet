import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  timeout: 60_000,
  retries: process.env.CI ? 2 : 0,
  workers: 1, // Extension tests must run serially (shared browser state)
  reporter: 'html',
  use: {
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'extension',
      use: {
        ...devices['Desktop Chrome'],
        headless: false, // Required — Chrome extensions don't work headless
      },
      testMatch: '**/*.extension.test.{ts,js}',
    },
  ],
});
