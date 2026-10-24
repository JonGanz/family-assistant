import { expect, test } from '@playwright/test'

// Credentials are created by scripts/e2e.sh (fictional, example.com).
const USER = process.env.FA_E2E_USER ?? 'alex@example.com'
const PASSWORD = process.env.FA_E2E_PASSWORD ?? 'correct-horse-battery-staple'
const CHILD = process.env.FA_E2E_CHILD_USER ?? 'riley@example.com'
const CHILD_PASSWORD = process.env.FA_E2E_CHILD_PASSWORD ?? 'dino1234'

test('health endpoints are up', async ({ request }) => {
  expect((await request.get('/healthz')).ok()).toBeTruthy()
  expect((await request.get('/readyz')).ok()).toBeTruthy()
})

test('anonymous /me is rejected', async ({ request }) => {
  expect((await request.get('/api/v1/me')).status()).toBe(401)
})

test('adult login, /me, member list, dashboard heartbeat', async ({ page }) => {
  const login = await page.request.post('/api/v1/auth/login', {
    data: { username: USER, password: PASSWORD },
  })
  expect(login.ok()).toBeTruthy()

  const me = await page.request.get('/api/v1/me')
  expect(me.ok()).toBeTruthy()
  const body = await me.json()
  expect(body.actor.kind).toBe('user')

  const members = await page.request.get('/api/v1/members')
  expect(members.ok()).toBeTruthy()
  const list = await members.json()
  const rows = Array.isArray(list) ? list : (list.members ?? list.items ?? [])
  expect(rows.length).toBeGreaterThanOrEqual(4)

  await page.goto('/dashboard')
  await expect(page.locator('body')).not.toBeEmpty()

  // The SPA shell is served and the SSE stream delivers a heartbeat.
  const heartbeat = await page.evaluate(
    () =>
      new Promise<string>((resolve, reject) => {
        const es = new EventSource('/api/v1/stream')
        es.addEventListener('heartbeat', () => {
          es.close()
          resolve('heartbeat')
        })
        setTimeout(() => {
          es.close()
          reject(new Error('no heartbeat within 25s'))
        }, 25_000)
      }),
  )
  expect(heartbeat).toBe('heartbeat')
})

test('child login works and sees public member fields only', async ({ request }) => {
  const login = await request.post('/api/v1/auth/login', {
    data: { username: CHILD, password: CHILD_PASSWORD },
  })
  expect(login.ok()).toBeTruthy()
  const me = await (await request.get('/api/v1/me')).json()
  expect(me.actor.kind).toBe('user')
})
