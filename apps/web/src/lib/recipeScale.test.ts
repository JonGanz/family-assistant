import { describe, expect, it } from 'vitest'
import type { RecipeLineView } from '../api/food'
import { formatQty, scaleLine } from './recipeScale'

const line = (raw_text: string, qty?: number | null, extra: Partial<RecipeLineView> = {}) =>
  ({ id: 'l', ord: 0, raw_text, qty, match_state: 'unmatched', ...extra }) as RecipeLineView

describe('formatQty', () => {
  it.each([
    [0.5, '½'],
    [1.5, '1½'],
    [2.5, '2½'],
    [0.25, '¼'],
    [1 / 3, '⅓'],
    [2 / 3, '⅔'],
    [0.75, '¾'],
    [2, '2'],
    [0.2, '0.2'],
    [1.14, '1.14'],
    [2.999, '3'],
  ])('%d -> %s', (n, s) => expect(formatQty(n)).toBe(s))
})

describe('scaleLine', () => {
  it.each([
    ['½ cup milk', 0.5, 1, '½ cup milk'],
    ['0.75 cup milk', 0.75, 2, '1½ cup milk'],
    ['1/3 cup oil', 1 / 3, 1.5, '½ cup oil'],
    ['1 1/2 cups flour', 1.5, 2, '3 cups flour'],
    ['1-2 cups rice', 1, 2, '2-4 cups rice'],
    ['1 to 2 cups rice', 1, 2, '2 to 4 cups rice'],
    ['2 eggs', 2, 1.25, '2½ eggs'],
    ['1½ tsp salt', 1.5, 2, '3 tsp salt'],
  ])('%s', (raw, qty, f, out) => {
    expect(scaleLine(line(raw, qty), f === 1 ? 1 : f)).toBe(out)
  })
  it('factor 1 returns raw_text', () => {
    expect(scaleLine(line('½ cup milk', 0.5), 1)).toBe('½ cup milk')
  })
  it('no qty is unchanged', () => {
    expect(scaleLine(line('salt to taste'), 3)).toBe('salt to taste')
    expect(scaleLine(line('salt', null), 3)).toBe('salt')
  })
  it('falls back to qty, unit and item without a leading amount', () => {
    expect(scaleLine(line('a pinch of salt', 1, { unit: 'pinch', item: 'salt' }), 2)).toBe(
      '2 pinch salt',
    )
  })
})
