import { defineConfig } from '@playwright/test'

const use = {
  baseURL: process.env.FA_E2E_BASE_URL ?? 'http://127.0.0.1:18080',
  headless: true,
  extraHTTPHeaders: { 'X-FA-Request': '1' },
}

export default defineConfig({
  timeout: 60_000,
  retries: 0,
  reporter: [['list']],
  projects: [
    // Everything runs in parallel except the specs that add a mail source to the shared stack:
    // the M3 specs count the messages of the seeded mailbox, so those run afterwards.
    { name: 'main', testDir: '.', testMatch: /.*\.spec\.ts/, testIgnore: /approvals\.spec\.ts/, use },
    { name: 'after', testDir: '.', testMatch: /approvals\.spec\.ts/, dependencies: ['main'], use },
  ],
})
