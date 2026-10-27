import type { ImportReport } from '../../api/client'

export interface ReportRow {
  label: string
  created: number
  updated: number
  unchanged: number
  skipped: number
}

const AREAS: [string, string][] = [
  ['household', 'Household'],
  ['members', 'Members'],
  ['sources', 'Calendar sources'],
  ['rules', 'Weather rules'],
  ['task_lists', 'Task lists'],
  ['task_definitions', 'Chores and schedules'],
  ['routines', 'Routines'],
  ['notify', 'Notification settings'],
  ['orgs', 'Organizations'],
  ['enrollments', 'Enrollments'],
  ['mail_sources', 'Mail sources'],
  ['categories', 'Mail categories'],
  ['mail_rules', 'Mail rules'],
]

/** One row per area that has anything to say. */
export function summarise(r: ImportReport): ReportRow[] {
  const n = (k: string): number => {
    const v = (r as unknown as Record<string, unknown>)[k]
    return typeof v === 'number' ? v : v === true ? 1 : 0
  }
  return AREAS.map(([k, label]) => ({
    label,
    created: n(`${k}_created`),
    updated: n(`${k}_updated`),
    unchanged: n(`${k}_unchanged`),
    skipped: n(`${k}_skipped`),
  })).filter((x) => x.created + x.updated + x.unchanged + x.skipped > 0)
}
