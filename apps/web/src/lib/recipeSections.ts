// Groups a recipe's lines and steps by section (D-124, plan/M5.1.md "Web scaffold").
// Shared lines and finish steps have no section; a section_id that names no section counts as none.
import type { RecipeLineView, RecipeSectionView, RecipeStepView, RecipeView } from '../api/food'

export interface RecipeGroup {
  section: RecipeSectionView
  lines: RecipeLineView[]
  steps: RecipeStepView[]
}

export interface GroupedRecipe {
  shared: RecipeLineView[]
  sections: RecipeGroup[]
  finish: RecipeStepView[]
}

const byOrd = <T extends { ord: number }>(a: T, b: T) => a.ord - b.ord

export function groupRecipe(r: Pick<RecipeView, 'lines' | 'steps' | 'sections'>): GroupedRecipe {
  const sections = [...(r.sections ?? [])].sort(byOrd)
  const groups = new Map<string, RecipeGroup>(
    sections.map((s) => [s.id, { section: s, lines: [], steps: [] }]),
  )
  const shared: RecipeLineView[] = []
  const finish: RecipeStepView[] = []
  for (const l of [...r.lines].sort(byOrd)) {
    const g = l.section_id ? groups.get(l.section_id) : undefined
    if (g) g.lines.push(l)
    else shared.push(l)
  }
  for (const s of [...r.steps].sort(byOrd)) {
    const g = s.section_id ? groups.get(s.section_id) : undefined
    if (g) g.steps.push(s)
    else finish.push(s)
  }
  return { shared, sections: sections.map((s) => groups.get(s.id)!), finish }
}
