import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './tests/e2e',
  timeout: 60_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  retries: 1,
  outputDir: 'test-results/pw-artifacts',
  reporter: [
    ['list'],
    ['html', { outputFolder: 'test-results/pw-report', open: 'never' }],
  ],
  use: {
    baseURL: 'http://localhost:1420',
    trace: 'on',
    screenshot: 'only-on-failure',
    video: 'on-first-retry',
    actionTimeout: 8_000,
  },
  projects: [
    {
      name: 'winserver-e2e',
      use: { viewport: { width: 1400, height: 900 } },
    },
  ],
})
