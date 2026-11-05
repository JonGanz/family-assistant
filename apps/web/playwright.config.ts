import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  // Screenshot specs (@shots) run only from scripts/ui-shots.sh, which sets UI_SHOTS=1.
  grepInvert: process.env.UI_SHOTS ? undefined : /@shots/,
  use: { baseURL: process.env.E2E_BASE_URL ?? 'http://localhost:18080' },
})
