import type { RecipeDraft } from '../api/food'
import { fromDraft, type SectionsForm } from '../components/food/recipes/sectionsForm'

/** The editor form fields a re-import touches (D-126). */
export interface ReimportForm extends SectionsForm {
  servings: string
  prep: string
  cook: string
  total: string
}

const n2s = (n?: number | null) => (n == null ? '' : String(n))

/**
 * Replaces ingredients, steps, sections, servings and times with the draft's. Title, tags, note
 * and image stay as they are. Returns the new import metadata (`import_method`, `yield_text`).
 */
export function applyReimport(
  form: ReimportForm,
  d: RecipeDraft,
): { import_method: RecipeDraft['origin']; yield_text: string | null } {
  const s = fromDraft(d)
  form.lines = s.lines
  form.steps = s.steps
  form.sections = s.sections
  form.servings = n2s(d.servings)
  form.prep = n2s(d.prep_minutes)
  form.cook = n2s(d.cook_minutes)
  form.total = n2s(d.total_minutes)
  return { import_method: d.origin, yield_text: d.yield_text ?? null }
}
