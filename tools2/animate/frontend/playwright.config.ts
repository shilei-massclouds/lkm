import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  timeout: 60_000,
  expect: { timeout: 5_000 },
  fullyParallel: false,
  workers: 1,
  outputDir: 'test-results',
  use: {
    ...devices['Desktop Chrome'],
    colorScheme: 'light',
    viewport: { width: 1280, height: 900 }
  }
});
