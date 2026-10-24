import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: '.',
  testMatch: /.*\.spec\.ts/,
  timeout: 60_000,
  retries: 0,
  reporter: [['list']],
  use: {
    baseURL: process.env.FA_E2E_BASE_URL ?? 'http://127.0.0.1:18080',
    headless: true,
    extraHTTPHeaders: { 'X-FA-Request': '1' },
  },
})
