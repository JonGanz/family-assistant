import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { FoodApi, GroceryItemView, GroceryView } from '../../api/food'
import ShoppingList from '../../components/food/grocery/ShoppingList.vue'
import { groupItems } from '../../components/food/grocery/grouping'
import { resetSnackbar } from '../../ui/composables/useSnackbar'

const item = (id: string, text: string, extra: Partial<GroceryItemView> = {}): GroceryItemView =>
  ({
    id,
    text,
    display: '',
    section: 'list',
    manual: true,
    sources: [],
    version: 1,
    ...extra,
  }) as GroceryItemView

const aisles = [
  { id: 'produce', name: 'Produce', ord: 0 },
  { id: 'dairy', name: 'Dairy', ord: 1 },
  { id: 'bakery', name: 'Bakery', ord: 2 },
] as never
const stores = [
  { id: 'sa', name: 'Corner Shop', ord: 0 },
  { id: 'sb', name: 'Market', ord: 1 },
] as never
// Corner Shop: dairy first, produce second, bakery hidden (D-121).
const storeAisles = {
  sa: [
    { aisle_id: 'dairy', ord: 0, hidden: false },
    { aisle_id: 'produce', ord: 1, hidden: false },
    { aisle_id: 'bakery', ord: 2, hidden: true },
  ],
} as never

describe('groupItems', () => {
  const items = [
    item('1', 'Apples', { store_id: 'sa', aisle_id: 'produce' } as never),
    item('2', 'Milk', { store_id: 'sa', aisle_id: 'dairy' } as never),
    item('3', 'Bread', { store_id: 'sa', aisle_id: 'bakery' } as never),
    item('4', 'Batteries', { store_id: 'sa' } as never),
    item('5', 'Pears', { store_id: 'sb', aisle_id: 'produce' } as never),
    item('6', 'Cheese', { store_id: 'sb', aisle_id: 'dairy' } as never),
    item('7', 'Soap'),
  ]

  it('orders stores, "Any store" last, then aisles in each store order; hidden goes to Other', () => {
    const g = groupItems(items, stores, aisles, storeAisles)
    expect(g.map((x) => x.name)).toEqual(['Corner Shop', 'Market', 'Any store'])
    expect(g[0]!.aisles.map((a) => a.name)).toEqual(['Dairy', 'Produce', 'Other'])
    expect(g[0]!.aisles[2]!.items.map((i) => i.text).sort()).toEqual(['Batteries', 'Bread'])
    // Market has no rows: household order.
    expect(g[1]!.aisles.map((a) => a.name)).toEqual(['Produce', 'Dairy'])
  })

  it('filters to chosen stores and keeps "Any store" items', () => {
    const g = groupItems(items, stores, aisles, storeAisles, ['sb'])
    expect(g.map((x) => x.name)).toEqual(['Market', 'Any store'])
  })

  it('with no stores there is one group by household aisle, no store heading', () => {
    const g = groupItems(items, [], aisles, {})
    expect(g).toHaveLength(1)
    expect(g[0]!.name).toBeNull()
    expect(g[0]!.aisles.map((a) => a.name)).toEqual(['Produce', 'Dairy', 'Bakery', 'Other'])
  })
})

function makeApi(view: GroceryView): FoodApi {
  return {
    grocery: vi.fn().mockResolvedValue(view),
    storeAisles: vi.fn().mockImplementation(async (id: string) => (storeAisles as never)[id] ?? []),
    addItem: vi
      .fn()
      .mockImplementation(async (b: { id: string; text: string }) => item(b.id, b.text)),
    checkItem: vi.fn().mockResolvedValue({}),
    uncheckItem: vi.fn().mockResolvedValue({}),
    lowItem: vi.fn().mockResolvedValue({}),
    updateItem: vi.fn(),
    deleteItem: vi.fn(),
    ingredients: vi.fn().mockResolvedValue([]),
    undo: vi.fn(),
  } as unknown as FoodApi
}

const base = (over: Partial<GroceryView> = {}): GroceryView => ({
  stores: [],
  aisles: [],
  items: [],
  staples: [],
  ...over,
})

async function mountList(view: GroceryView, props: { canEdit: boolean; kiosk?: boolean }) {
  const api = makeApi(view)
  const w = mount(ShoppingList, { props: { ...props, api }, attachTo: document.body })
  await flushPromises()
  return { w, api }
}

let pinia: ReturnType<typeof createPinia>
beforeEach(() => {
  pinia = createPinia()
  setActivePinia(pinia)
  resetSnackbar()
  localStorage.clear()
  window.dispatchEvent(new Event('online'))
})

// The queue is a module singleton: stop the previous test's store so its watchers don't flush it.
afterEach(() => {
  ;(pinia as unknown as { _s: Map<string, { $dispose(): void }> })._s.forEach((s) => s.$dispose())
})

describe('ShoppingList', () => {
  it('renders store groups and aisle headings in order', async () => {
    const { w } = await mountList(
      base({
        stores,
        aisles,
        items: [
          item('1', 'Apples', { store_id: 'sa', aisle_id: 'produce' } as never),
          item('2', 'Milk', { store_id: 'sa', aisle_id: 'dairy' } as never),
          item('3', 'Bread', { store_id: 'sa', aisle_id: 'bakery' } as never),
        ],
      }),
      { canEdit: false },
    )
    const heads = w.findAll('h2, h3').map((h) => h.text())
    expect(heads).toEqual(['Corner Shop', 'Dairy', 'Produce', 'Other'])
    w.unmount()
  })

  it('shows aisles only when the household has no stores', async () => {
    const { w } = await mountList(
      base({ aisles, items: [item('1', 'Apples', { aisle_id: 'produce' } as never)] }),
      { canEdit: false },
    )
    expect(w.find('.shopping__store-h').exists()).toBe(false)
    expect(w.findAll('h3').map((h) => h.text())).toEqual(['Produce'])
    w.unmount()
  })

  it('shows the staples check and sends Low', async () => {
    const { w, api } = await mountList(
      base({
        staples: [{ item_id: 's1', ingredient_id: 'i1', name: 'Salt', needed_for: [] }] as never,
      }),
      { canEdit: false },
    )
    expect(w.find('[data-testid="grocery-staples"]').text()).toContain('Salt')
    await w.find('[data-testid="grocery-low-s1"]').trigger('click')
    await flushPromises()
    expect(api.lowItem).toHaveBeenCalledWith('s1')
    expect(w.find('[data-testid="grocery-staples"]').exists()).toBe(false)
    expect(w.find('[data-testid="grocery-item-s1"]').exists()).toBe(true)
    w.unmount()
  })

  it('gives a child no edit or delete controls, an adult both', async () => {
    const v = base({ items: [item('1', 'Apples')] })
    const child = await mountList(v, { canEdit: false })
    expect(child.w.find('button[aria-label^="Edit"]').exists()).toBe(false)
    expect(child.w.find('button[aria-label^="Delete"]').exists()).toBe(false)
    expect(child.w.find('[data-testid="grocery-check-1"]').exists()).toBe(true)
    child.w.unmount()
    const adult = await mountList(v, { canEdit: true })
    expect(adult.w.find('button[aria-label="Edit Apples"]').exists()).toBe(true)
    expect(adult.w.find('button[aria-label="Delete Apples"]').exists()).toBe(true)
    adult.w.unmount()
    const kiosk = await mountList(v, { canEdit: false, kiosk: true })
    expect(kiosk.w.find('button[aria-label^="Delete"]').exists()).toBe(false)
    kiosk.w.unmount()
  })

  it('checks an item at once and moves it to the collapsed Checked group', async () => {
    const { w, api } = await mountList(base({ items: [item('1', 'Apples')] }), { canEdit: false })
    await w.find('[data-testid="grocery-check-1"]').setValue(true)
    await flushPromises()
    expect(api.checkItem).toHaveBeenCalledWith('1')
    expect(w.find('[data-testid="grocery-item-1"]').exists()).toBe(false)
    expect(w.find('[data-testid="grocery-checked-toggle"]').text()).toContain('Checked (1)')
    w.unmount()
  })

  it('queues an add while offline, shows the note, and flushes when back online', async () => {
    const { w, api } = await mountList(base(), { canEdit: false })
    window.dispatchEvent(new Event('offline'))
    await flushPromises()
    await w.find('[data-testid="grocery-add-input"]').setValue('Oat milk')
    await w.find('form').trigger('submit')
    await flushPromises()
    expect(api.addItem).not.toHaveBeenCalled()
    expect(w.find('[data-testid="grocery-offline-note"]').text()).toContain(
      'Offline, 1 change waiting',
    )
    expect(w.text()).toContain('Oat milk')

    ;(api.grocery as ReturnType<typeof vi.fn>).mockResolvedValue(
      base({ items: [item('x', 'Oat milk')] }),
    )
    window.dispatchEvent(new Event('online'))
    await flushPromises()
    expect(api.addItem).toHaveBeenCalledTimes(1)
    const sent = (api.addItem as ReturnType<typeof vi.fn>).mock.calls[0]![0] as {
      id: string
      text: string
    }
    expect(sent.text).toBe('Oat milk')
    expect(sent.id).toMatch(/^[0-9a-f-]{36}$/)
    expect(w.find('[data-testid="grocery-offline-note"]').exists()).toBe(false)
    w.unmount()
  })
})
