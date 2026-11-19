// Editor-side model of a recipe's sections (D-124): shared lines, finish steps and named sections.
import type { DraftSection, RecipeDraft, RecipeSectionInput, RecipeView } from '../../../api/food'
import { groupRecipe } from '../../../lib/recipeSections'

export interface SectionForm {
  name: string
  lines: string[]
  steps: string[]
}

export interface SectionsForm {
  lines: string[]
  steps: string[]
  sections: SectionForm[]
}

/** Where an item lives: the top level (shared lines / finish steps) or a section index. */
export type Place = 'top' | number

export const blankSections = (): SectionsForm => ({ lines: [''], steps: [''], sections: [] })

export function fromView(v: Pick<RecipeView, 'lines' | 'steps' | 'sections'>): SectionsForm {
  const g = groupRecipe(v)
  return {
    lines: g.shared.map((l) => l.raw_text),
    steps: g.finish.length ? g.finish.map((s) => s.text) : g.sections.length ? [] : [''],
    sections: g.sections.map((x) => ({
      name: x.section.name,
      lines: x.lines.map((l) => l.raw_text),
      steps: x.steps.map((s) => s.text),
    })),
  }
}

export function fromDraft(
  d: Pick<RecipeDraft, 'ingredients' | 'steps'> & { sections?: DraftSection[] },
): SectionsForm {
  const sections = (d.sections ?? []).map((s) => ({
    name: s.name,
    lines: (s.ingredients ?? []).map((i) => i.text),
    steps: [...(s.steps ?? [])],
  }))
  const lines = d.ingredients.map((i) => i.text)
  return {
    lines: lines.length || sections.length ? lines : [''],
    steps: d.steps.length || sections.length ? [...d.steps] : [''],
    sections,
  }
}

const clean = (l: string[]) => l.map((x) => x.trim()).filter(Boolean)

export function toInput(f: SectionsForm): {
  lines: string[]
  steps: string[]
  sections: RecipeSectionInput[]
} {
  return {
    lines: clean(f.lines),
    steps: clean(f.steps),
    sections: f.sections.map((s) => ({
      name: s.name.trim(),
      lines: clean(s.lines),
      steps: clean(s.steps),
    })),
  }
}

export function addSection(f: SectionsForm, name = ''): void {
  f.sections.push({ name, lines: [], steps: [] })
}

export function moveSection(f: SectionsForm, i: number, d: number): void {
  const j = i + d
  if (j < 0 || j >= f.sections.length) return
  const s = f.sections
  ;[s[i], s[j]] = [s[j]!, s[i]!]
}

/** Appends section `from`'s lines and steps to section `into`, then removes `from`. */
export function mergeSection(f: SectionsForm, from: number, into: number): void {
  const a = f.sections[from]
  const b = f.sections[into]
  if (!a || !b || from === into) return
  b.lines.push(...a.lines)
  b.steps.push(...a.steps)
  f.sections.splice(from, 1)
}

/** Removes a section; its lines go to the shared lines and its steps to the finish steps. */
export function deleteSection(f: SectionsForm, i: number): void {
  const s = f.sections[i]
  if (!s) return
  f.lines.push(...s.lines)
  f.steps.push(...s.steps)
  f.sections.splice(i, 1)
}

const list = (f: SectionsForm, kind: 'lines' | 'steps', p: Place): string[] | undefined =>
  p === 'top' ? f[kind] : f.sections[p]?.[kind]

/** Moves item `idx` of `kind` from one place to the end of another. */
export function moveItem(
  f: SectionsForm,
  kind: 'lines' | 'steps',
  from: Place,
  idx: number,
  to: Place,
): void {
  if (from === to) return
  const src = list(f, kind, from)
  const dst = list(f, kind, to)
  if (!src || !dst || idx < 0 || idx >= src.length) return
  dst.push(...src.splice(idx, 1))
}
