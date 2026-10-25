import { addDays, dayKey, startOfDay } from '../lib/time'
import type { MemberRef, TaskItemModel } from '../lib/tasks'
import type { PanelConfig } from './layout'
import type { DashboardData, MemberPublic, RoutineRunView, TaskView } from './types'

const FALLBACK = '#6b7280'
export const DEFAULT_NOW_SIZE = 7

export function toMemberRef(m: MemberPublic): MemberRef {
  return { id: m.id, name: m.display_name, color: m.color ?? FALLBACK, avatarUrl: m.avatar_url }
}

export function toTaskItem(t: TaskView, members: MemberPublic[]): TaskItemModel {
  const byId = new Map(members.map((m) => [m.id, m]))
  const ref = (id: string): MemberRef | null => {
    const m = byId.get(id)
    return m ? toMemberRef(m) : null
  }
  return {
    id: t.id,
    title: t.title,
    dueAt: t.due_at,
    status: t.status,
    priorityFlag: t.priority_flag,
    assignees: t.assignees.map(ref).filter((m): m is MemberRef => m !== null),
    stepsDone: t.steps.filter((s) => s.done_at).length,
    stepsTotal: t.steps.length,
    claimedBy: t.claimed_by ? ref(t.claimed_by) : null,
    origin: t.origin,
  }
}

/** A panel option holding a list of ids; absent or empty means "no filter". */
export function idFilter(panel: PanelConfig, key: string): Set<string> | null {
  const v = panel[key]
  return Array.isArray(v) && v.length > 0 ? new Set(v.map(String)) : null
}

export const tasksOf = (d: DashboardData): TaskView[] => d.tasks ?? []
export const runsOf = (d: DashboardData): RoutineRunView[] => d.routines ?? []

export function filterTasks(tasks: TaskView[], panel: PanelConfig): TaskView[] {
  const members = idFilter(panel, 'members')
  const lists = idFilter(panel, 'lists')
  return tasks.filter(
    (t) => (!members || t.assignees.some((a) => members.has(a))) && (!lists || lists.has(t.list)),
  )
}

const dueMs = (t: TaskView) => (t.due_at ? Date.parse(t.due_at) : Number.POSITIVE_INFINITY)

/** Open tasks by score (then due), filtered by the panel options, at most the Now list size. */
export function nowTasks(data: DashboardData, panel: PanelConfig): TaskView[] {
  const open = tasksOf(data).filter((t) => t.status === 'open')
  const size = data.now_list_size ?? DEFAULT_NOW_SIZE
  return filterTasks(open, panel)
    .sort((a, b) => b.score - a.score || dueMs(a) - dueMs(b))
    .slice(0, Math.max(1, size))
}

export interface MemberColumn {
  key: string
  member: MemberPublic | null
  tasks: TaskView[]
}

/** One column per member: open tasks due by the end of today (or undated), overdue first. */
export function memberColumns(
  data: DashboardData,
  members: MemberPublic[],
  panel: PanelConfig,
  now: Date,
): MemberColumn[] {
  const tz = data.household.tz
  const endOfToday = startOfDay(tz, addDays(dayKey(now, tz), 1)).getTime()
  const wanted = idFilter(panel, 'members')
  const shown = wanted ? members.filter((m) => wanted.has(m.id)) : members
  const open = filterTasksByLists(tasksOf(data), panel).filter(
    (t) => t.status === 'open' && dueMs(t) < endOfToday,
  )
  const sorted = (l: TaskView[]) => l.sort((a, b) => dueMs(a) - dueMs(b) || b.score - a.score)
  const cols: MemberColumn[] = shown.map((m) => ({
    key: m.id,
    member: m,
    tasks: sorted(open.filter((t) => t.assignees.includes(m.id))),
  }))
  if (!wanted) {
    const anyone = open.filter((t) => t.assignees.length === 0)
    if (anyone.length) cols.push({ key: 'anyone', member: null, tasks: sorted(anyone) })
  }
  return cols
}

function filterTasksByLists(tasks: TaskView[], panel: PanelConfig): TaskView[] {
  const lists = idFilter(panel, 'lists')
  return lists ? tasks.filter((t) => lists.has(t.list)) : tasks
}

/** "HH:MM" of an instant in the given tz. */
function hhmm(now: Date, tz: string): string {
  return new Intl.DateTimeFormat('en-GB', {
    timeZone: tz,
    hourCycle: 'h23',
    hour: '2-digit',
    minute: '2-digit',
  }).format(now)
}

/** `starts_at ≤ now < done_by` as household-local "HH:MM", and the run belongs to today. */
export function routineActive(run: RoutineRunView, now: Date, tz: string): boolean {
  const t = hhmm(now, tz)
  return run.starts_at <= t && t < run.done_by && run.run_date === dayKey(now, tz)
}

export function routineRuns(data: DashboardData, panel: PanelConfig): RoutineRunView[] {
  const members = idFilter(panel, 'members')
  return runsOf(data).filter((r) => !members || members.has(r.member))
}

/** Runs the routine panel shows: all of them when pinned, else the ones in their window. */
export function shownRuns(data: DashboardData, panel: PanelConfig, now: Date): RoutineRunView[] {
  const runs = routineRuns(data, panel)
  if (panel['pinned'] === true) return runs
  return runs.filter((r) => routineActive(r, now, data.household.tz))
}

/** Layout hook: a routine panel without anything to show takes no space. */
export function panelVisible(panel: PanelConfig, data: DashboardData, now: Date): boolean {
  return panel.kind !== 'routine' || shownRuns(data, panel, now).length > 0
}

/** Calendar-displayed tasks by due day: key dates (`always`), plus `until_done` ones when asked. */
export function datedTasks(
  data: DashboardData,
  tz: string,
  includeUntilDone: boolean,
): Map<string, TaskView[]> {
  const out = new Map<string, TaskView[]>()
  for (const t of tasksOf(data)) {
    if (!t.due_at) continue
    const shown =
      t.calendar_display === 'always' ||
      (includeUntilDone && t.calendar_display === 'until_done' && t.status === 'open')
    if (!shown) continue
    const k = dayKey(t.due_at, tz)
    out.set(k, [...(out.get(k) ?? []), t])
  }
  return out
}
