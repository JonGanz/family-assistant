import { expect, test, type Page } from '@playwright/test'

// UI spec for M5.3 (food parts) on the seeded stack from scripts/e2e.sh: the compact meal plan [+] in each day's
// heading, and the recipe page's back button returning to the page it was opened from.
// Env: E2E_BASE_URL, E2E_USERNAME/E2E_PASSWORD (adult), E2E_FIXTURES (the fixture server).
const FIXTURES = process.env.E2E_FIXTURES ?? 'http://127.0.0.1:18091'
const adult = {
  user: process.env.E2E_USERNAME ?? 'alex@example.com',
  pass: process.env.E2E_PASSWORD ?? '',
}
const child = {
  user: process.env.E2E_CHILD_USERNAME ?? 'riley@example.com',
  pass: process.env.E2E_CHILD_PASSWORD ?? '',
}
const hdr = { 'X-FA-Request': '1' }

async function uiLogin(page: Page, who = adult) {
  await page.goto('/login')
  await page.getByLabel('Username').fill(who.user)
  await page.getByLabel('Password').fill(who.pass)
  await page.getByRole('button', { name: 'Log in' }).click()
  await expect(page).toHaveURL(/\/$/)
}

/** The New York calendar date N days from today (the household's zone). */
const day = (n: number) => {
  const [y, m, d] = new Intl.DateTimeFormat('en-CA', { timeZone: 'America/New_York' })
    .format(new Date())
    .split('-')
    .map(Number)
  return new Date(Date.UTC(y!, m! - 1, d! + n)).toISOString().slice(0, 10)
}

test.use({
  viewport: { width: 1280, height: 900 },
  extraHTTPHeaders: { 'X-Forwarded-For': '10.99.0.7' },
})

test('the meal plan heading [+] adds a meal, and back from a recipe returns to the plan', async ({
  page,
  browser,
}) => {
  await uiLogin(page)
  await page.goto('/food/recipes')
  await page.getByTestId('recipe-new').click()
  await page.getByTestId('recipe-import-url').click()
  await page.getByLabel('Recipe link').fill(`${FIXTURES}/recipes/jsonld.html`)
  await page.getByTestId('recipe-import-submit').click()
  await expect(page.getByLabel('Title')).toHaveValue('Fictional Lentil Stew')
  await page.getByTestId('recipe-save').getByRole('button', { name: 'Save' }).click()
  await expect(page).toHaveURL(/\/food\/recipes\/[0-9a-f-]{36}$/)
  const id = page.url().split('/').pop()!
  try {
    // Add through the compact [+] in a heading nine days out (other specs use tomorrow's cells).
    await page.goto('/food/plan')
    await page.getByTestId(`plan-day-add-${day(9)}`).click()
    const sheet = page.getByTestId('entry-sheet')
    await sheet.getByLabel('Search recipes').fill('Lentil')
    await page
      .getByTestId(/^pick-result-/)
      .first()
      .click()
    await page.getByTestId('entry-save').click()
    await expect(
      page
        .getByTestId(new RegExp(`^plan-cell-${day(9)}-`))
        .filter({ hasText: 'Fictional Lentil Stew' }),
    ).toHaveCount(1)

    // The Today control exists on the plan page.
    await expect(page.getByTestId('plan-today')).toBeVisible()

    // A child cannot edit the plan: its chip opens the recipe, and the top-left back returns to the plan.
    const ctx = await browser.newContext({
      baseURL: process.env.E2E_BASE_URL ?? 'http://localhost:18080',
      viewport: { width: 1280, height: 900 },
      extraHTTPHeaders: { 'X-Forwarded-For': '10.99.0.8' },
    })
    try {
      const kid = await ctx.newPage()
      await uiLogin(kid, child)
      await kid.goto('/food/plan')
      await kid
        .locator(`.agenda__day[data-day="${day(9)}"]`)
        .getByRole('button', { name: 'Fictional Lentil Stew' })
        .first()
        .click()
      // Other specs may plan a same-named recipe on this day: any recipe page proves the chip opened one.
      await expect(kid).toHaveURL(/\/food\/recipes\/[0-9a-f-]{36}$/)
      await kid.getByRole('button', { name: 'Back' }).click()
      await expect(kid).toHaveURL(/\/food\/plan$/)
    } finally {
      await ctx.close()
    }
  } finally {
    const plan = await page.request
      .get(`/api/v1/meal-plan?from=${day(-7)}&to=${day(50)}`)
      .then((r) => r.json())
    for (const e of plan.filter((e: { recipe_id?: string }) => e.recipe_id === id))
      await page.request.delete(`/api/v1/meal-plan/${e.id}`, { headers: hdr })
    await page.request.delete(`/api/v1/recipes/${id}`, { headers: hdr })
  }
})
