import type { RoutineInput, RoutinePatch, RoutineView } from '../../api/client'

export const DEFAULT_KEYWORDS = ['no school', 'holiday', 'inset', 'closed']

export interface RoutineForm {
  name: string
  members: string[]
  startsAt: string
  doneBy: string
  steps: { text: string; target: string }[]
  dayFilter: RoutineInput['day_filter']
  schoolCalendar: string
  /** Comma separated. */
  keywords: string
  showOnDashboard: boolean
}

export function emptyRoutineForm(): RoutineForm {
  return {
    name: '',
    members: [],
    startsAt: '07:00',
    doneBy: '08:15',
    steps: [{ text: '', target: '' }],
    dayFilter: 'weekdays',
    schoolCalendar: '',
    keywords: DEFAULT_KEYWORDS.join(', '),
    showOnDashboard: true,
  }
}

export function routineFormFrom(r: RoutineView): RoutineForm {
  return {
    name: r.name,
    members: [...r.members],
    startsAt: r.starts_at.slice(0, 5),
    doneBy: r.done_by.slice(0, 5),
    steps: r.steps.map((s) => ({ text: s.text, target: s.target_time?.slice(0, 5) ?? '' })),
    dayFilter: r.day_filter,
    schoolCalendar: r.school_calendar ?? '',
    keywords: r.no_school_match.join(', '),
    showOnDashboard: r.show_on_dashboard,
  }
}

const keywordList = (s: string) =>
  s
    .split(',')
    .map((k) => k.trim())
    .filter(Boolean)

const stepInputs = (f: RoutineForm) =>
  f.steps
    .filter((s) => s.text.trim())
    .map((s) => ({ text: s.text.trim(), target_time: s.target || null }))

export function routineProblems(f: RoutineForm): string[] {
  const out: string[] = []
  if (!f.name.trim()) out.push('Give it a name.')
  if (f.members.length === 0) out.push('Pick at least one person.')
  if (stepInputs(f).length === 0) out.push('Add at least one step.')
  if (f.startsAt && f.doneBy && f.doneBy <= f.startsAt)
    out.push('The finish time must be after the start time.')
  return out
}

export function routineInput(f: RoutineForm): RoutineInput {
  return {
    name: f.name.trim(),
    members: f.members,
    steps: stepInputs(f),
    starts_at: f.startsAt,
    done_by: f.doneBy,
    day_filter: f.dayFilter,
    school_calendar: f.dayFilter === 'school_days' && f.schoolCalendar ? f.schoolCalendar : null,
    no_school_match: keywordList(f.keywords),
    show_on_dashboard: f.showOnDashboard,
  }
}

export function routinePatch(f: RoutineForm): RoutinePatch {
  const i = routineInput(f)
  return { ...i, school_calendar: i.school_calendar }
}
