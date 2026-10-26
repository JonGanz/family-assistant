import { expect, test } from '@playwright/test'

// The home-network dashboard listener (D-064), started by scripts/e2e.sh on FA_E2E_LAN_URL.
const LAN = process.env.FA_E2E_LAN_URL ?? 'http://127.0.0.1:18081'
const USER = process.env.FA_E2E_USER ?? 'alex@example.com'
const PASSWORD = process.env.FA_E2E_PASSWORD ?? 'correct-horse-battery-staple'
const W = { 'X-FA-Request': '1' }

test.describe.configure({ mode: 'serial' })

let token = ''
let lanPairUrl = ''

test('pairing returns a home-network link', async ({ request }) => {
  const login = await request.post('/api/v1/auth/login', {
    data: { username: USER, password: PASSWORD },
  })
  expect(login.ok()).toBeTruthy()
  const res = await request.post('/api/v1/devices', { data: { name: `lan-${Date.now()}` }, headers: W })
  expect(res.status()).toBe(201)
  const body = await res.json()
  token = body.token
  lanPairUrl = body.lan_pair_url
  expect(lanPairUrl).toBe(`${LAN}/dashboard?device=${token}`)
})

test('only the dashboard is served, to devices only', async ({ playwright }) => {
  const lan = await playwright.request.newContext({ baseURL: LAN })
  const dev = { Authorization: `Bearer ${token}` }
  expect((await lan.get('/api/v1/dashboard?days=3', { headers: dev })).ok()).toBeTruthy()
  expect((await lan.get('/dashboard')).ok()).toBeTruthy()

  // Not routed on this listener, even for a device.
  expect((await lan.get('/api/v1/members', { headers: dev })).status()).toBe(404)
  expect((await lan.get('/api/v1/messages', { headers: dev })).status()).toBe(404)
  const login = await lan.post('/api/v1/auth/login', {
    data: { username: USER, password: PASSWORD },
    headers: W,
  })
  expect(login.status()).toBe(404)
  // No user session can be used here.
  expect((await lan.get('/api/v1/me')).status()).toBe(401)
  // PIN entry is off by default.
  const me = await (await lan.get('/api/v1/me', { headers: dev })).json()
  expect(me.actor.kind).toBe('device')
  const el = await lan.post('/api/v1/device/elevate', {
    data: { member_id: '00000000-0000-0000-0000-000000000000', pin: '0000' },
    headers: { ...dev, ...W },
  })
  expect(el.status()).toBe(403)
  await lan.dispose()
})

test('the dashboard renders through the home-network address', async ({ browser }) => {
  const ctx = await browser.newContext()
  const screen = await ctx.newPage()
  const refused: string[] = []
  screen.on('response', (r) => {
    if (r.url().startsWith(`${LAN}/api/`) && [401, 403, 404].includes(r.status())) {
      refused.push(`${r.status()} ${r.request().method()} ${r.url()}`)
    }
  })
  await screen.goto(lanPairUrl)
  await expect(screen.getByRole('region', { name: 'Agenda' })).toBeVisible()
  await expect(screen.getByTestId('temp')).toBeVisible()
  // Every call the dashboard makes is allowed on this listener.
  expect(refused).toEqual([])
  await ctx.close()
})
