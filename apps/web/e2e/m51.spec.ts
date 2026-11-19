import { expect, test, type Page } from '@playwright/test'

// UI spec for M5.1 on the seeded stack from scripts/e2e.sh: a sectioned recipe is imported from a
// fictional fixture page, edited as sections, saved and shown in section columns (D-124).
// Env: E2E_BASE_URL, E2E_USERNAME/E2E_PASSWORD (adult), E2E_FIXTURES (the fixture server).
const FIXTURES = process.env.E2E_FIXTURES ?? 'http://127.0.0.1:18091'
const adult = {
  user: process.env.E2E_USERNAME ?? 'alex@example.com',
  pass: process.env.E2E_PASSWORD ?? '',
}

async function uiLogin(page: Page) {
  await page.goto('/login')
  await page.getByLabel('Username').fill(adult.user)
  await page.getByLabel('Password').fill(adult.pass)
  await page.getByRole('button', { name: 'Log in' }).click()
  await expect(page).toHaveURL(/\/$/)
}

test.use({
  viewport: { width: 1280, height: 900 },
  extraHTTPHeaders: { 'X-Forwarded-For': '10.99.0.6' },
})

test('a sectioned page is imported, saved and shown as two section columns and a Finish card', async ({
  page,
}) => {
  await uiLogin(page)
  try {
    await page.goto('/food/recipes')
    await page.getByTestId('recipe-new').click()
    await page.getByTestId('recipe-import-url').click()
    await page.getByLabel('Recipe link').fill(`${FIXTURES}/recipes/sections.html`)
    await page.getByTestId('recipe-import-submit').click()
    await expect(page).toHaveURL(/\/food\/recipes\/new/)
    await expect(page.getByLabel('Title')).toHaveValue('Fictional Glazed Drumsticks')
    await expect(page.getByTestId('recipe-section-edit-0')).toBeVisible()
    await expect(page.getByTestId('recipe-section-edit-1')).toBeVisible()
    await expect(page.getByTestId('recipe-section-name-0')).toHaveValue('Chicken')
    await page.getByTestId('recipe-save').getByRole('button', { name: 'Save' }).click()
    await expect(page).toHaveURL(/\/food\/recipes\/[0-9a-f-]{36}$/)
    await expect(page.getByTestId('recipe-section-0')).toContainText('Chicken')
    await expect(page.getByTestId('recipe-section-1')).toContainText('Glaze')
    await expect(page.getByTestId('recipe-finish')).toContainText('Finish')
    await expect(page.getByTestId('recipe-finish')).toContainText('Brush the glaze')
  } finally {
    const hdr = { 'X-FA-Request': '1' }
    const rs = await page.request.get('/api/v1/recipes?q=Fictional Glazed').then((r) => r.json())
    for (const r of rs.items ?? [])
      await page.request.delete(`/api/v1/recipes/${r.id}`, { headers: hdr })
  }
})
