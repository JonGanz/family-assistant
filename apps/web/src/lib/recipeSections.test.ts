import { describe, expect, it } from 'vitest'
import { groupRecipe } from './recipeSections'
import type { RecipeLineView, RecipeStepView } from '../api/food'

const line = (id: string, ord: number, section_id?: string) =>
  ({ id, ord, raw_text: id, match_state: 'unmatched', section_id }) as RecipeLineView
const step = (id: string, ord: number, section_id?: string) =>
  ({ id, ord, text: id, section_id }) as RecipeStepView

describe('groupRecipe', () => {
  it('splits shared lines, sections in order and finish steps', () => {
    const g = groupRecipe({
      sections: [
        { id: 'b', ord: 1, name: 'Sauce' },
        { id: 'a', ord: 0, name: 'Chicken' },
      ],
      lines: [line('salt', 0), line('thigh', 1, 'a'), line('cream', 2, 'b'), line('x', 3, 'gone')],
      steps: [step('sear', 1, 'a'), step('simmer', 2, 'b'), step('serve', 3)],
    })
    expect(g.shared.map((l) => l.id)).toEqual(['salt', 'x'])
    expect(g.sections.map((s) => s.section.name)).toEqual(['Chicken', 'Sauce'])
    expect(g.sections[0].lines.map((l) => l.id)).toEqual(['thigh'])
    expect(g.sections[1].steps.map((s) => s.id)).toEqual(['simmer'])
    expect(g.finish.map((s) => s.id)).toEqual(['serve'])
  })

  it('leaves a recipe without sections flat', () => {
    const g = groupRecipe({ sections: [], lines: [line('a', 0)], steps: [step('s', 0)] })
    expect(g.sections).toEqual([])
    expect(g.shared).toHaveLength(1)
    expect(g.finish).toHaveLength(1)
  })
})
