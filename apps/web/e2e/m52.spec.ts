import { expect, test, type Page } from '@playwright/test'

// UI spec for M5.2 on the seeded stack from scripts/e2e.sh: a recipe imported from a flat
// fictional page is rated and planned, then re-imported from a sectioned version of the page
// (D-126). The fixture server cannot change a page's body, so the recipe's source link is moved
// to the sectioned page (same title) through the API before the re-import.
// Env: E2E_BASE_URL, E2E_USERNAME/E2E_PASSWORD (adult), E2E_FIXTURES (the fixture server).
const FIXTURES = process.env.E2E_FIXTURES ?? 'http://127.0.0.1:18091'
const adult = {
  user: process.env.E2E_USERNAME ?? 'alex@example.com',
  pass: process.env.E2E_PASSWORD ?? '',
}
const hdr = { 'X-FA-Request': '1' }

async function uiLogin(page: Page) {
  await page.goto('/login')
  await page.getByLabel('Username').fill(adult.user)
  await page.getByLabel('Password').fill(adult.pass)
  await page.getByRole('button', { name: 'Log in' }).click()
  await expect(page).toHaveURL(/\/$/)
}

test.use({
  viewport: { width: 1280, height: 900 },
  extraHTTPHeaders: { 'X-Forwarded-For': '10.99.0.7' },
})

test('re-importing a recipe keeps its id, rating and plan entry and shows the new sections', async ({
  page,
}) => {
  await uiLogin(page)
  try {
    await page.goto('/food/recipes')
    await page.getByTestId('recipe-new').click()
    await page.getByTestId('recipe-import-url').click()
    await page.getByLabel('Recipe link').fill(`${FIXTURES}/recipes/reimport-flat.html`)
    await page.getByTestId('recipe-import-submit').click()
    await expect(page.getByLabel('Title')).toHaveValue('Fictional Pan Dumplings')
    await expect(page.getByTestId('recipe-section-edit-0')).toHaveCount(0)
    await page.getByTestId('recipe-save').getByRole('button', { name: 'Save' }).click()
    await expect(page).toHaveURL(/\/food\/recipes\/[0-9a-f-]{36}$/)
    const id = page.url().split('/').pop()!
    await expect(page.getByTestId('recipe-sections')).toHaveCount(0)

    // Rate it and put it on tomorrow's dinner through the API.
    expect(
      (
        await page.request.put(`/api/v1/recipes/${id}/rating`, { headers: hdr, data: { stars: 4 } })
      ).ok(),
    ).toBeTruthy()
    const slots = await page.request.get('/api/v1/meals/slots').then((r) => r.json())
    const tomorrow = new Date(Date.now() + 86_400_000).toISOString().slice(0, 10)
    const added = await page.request.post('/api/v1/meal-plan', {
      headers: hdr,
      data: { date: tomorrow, slot_id: slots[0].id, recipe_id: id },
    })
    expect(added.ok()).toBeTruthy()
    const entryId = (await added.json()).entries[0].id

    // The page now has sections: move the source link to that version of the page.
    const res = await page.request.get(`/api/v1/recipes/${id}`)
    const v = await res.json()
    const put = await page.request.put(`/api/v1/recipes/${id}`, {
      headers: { ...hdr, 'If-Match': String(v.version) },
      data: {
        title: v.title,
        source_url: `${FIXTURES}/recipes/reimport-sections.html`,
        servings: v.servings ?? null,
        yield_text: v.yield_text ?? null,
        prep_minutes: v.prep_minutes ?? null,
        cook_minutes: v.cook_minutes ?? null,
        total_minutes: v.total_minutes ?? null,
        import_method: v.import_method,
        tags: v.tags,
        note: v.note ?? null,
        sections: [],
        lines: v.lines.map((l: { raw_text: string }) => l.raw_text),
        steps: v.steps.map((s: { text: string }) => s.text),
      },
    })
    expect(put.ok()).toBeTruthy()

    await page.reload()
    await page.getByTestId('recipe-reimport').click()
    await page.getByTestId('recipe-reimport-confirm').click()
    await expect(page).toHaveURL(/\/edit$/)
    await expect(page.getByTestId('recipe-reimport-notice')).toBeVisible()
    await expect(page.getByTestId('recipe-section-name-0')).toHaveValue('Dough')
    await expect(page.getByTestId('recipe-section-name-1')).toHaveValue('Filling')
    await page.getByTestId('recipe-save').getByRole('button', { name: 'Save' }).click()
    await expect(page).toHaveURL(new RegExp(`/food/recipes/${id}$`))
    await expect(page.getByTestId('recipe-section-0')).toContainText('Dough')
    await expect(page.getByTestId('recipe-section-1')).toContainText('Filling')

    // Same recipe, rating and plan entry.
    const after = await page.request.get(`/api/v1/recipes/${id}`).then((r) => r.json())
    expect(after.ratings.mine).toBe(4)
    const plan = await page.request
      .get(`/api/v1/meal-plan?from=${tomorrow}&to=${tomorrow}`)
      .then((r) => r.json())
    expect(
      plan.some((e: { id: string; recipe_id?: string }) => e.id === entryId && e.recipe_id === id),
    ).toBe(true)
  } finally {
    const rs = await page.request.get('/api/v1/recipes?q=Fictional Pan').then((r) => r.json())
    for (const r of rs.items ?? [])
      await page.request.delete(`/api/v1/recipes/${r.id}`, { headers: hdr })
  }
})
