import type { Member, TaskListView, TaskView } from '../api/client'
import type { MemberRef, TaskItemModel } from '../lib/tasks'
import { hueVar } from '../lib/hues'
import { dayKey } from '../lib/time'

const FALLBACK = hueVar(null)

export function memberRef(m: Member): MemberRef {
  return {
    id: m.id,
    name: m.display_name,
    color: hueVar(m.color),
    avatarUrl: m.avatar_url ?? null,
  }
}

export function memberRefs(members: Member[]): Map<string, MemberRef> {
  return new Map(members.map((m) => [m.id, memberRef(m)]))
}

/** Maps a generated `TaskView` onto the row view-model of `components/tasks`. */
export function toTaskItem(t: TaskView, refs: Map<string, MemberRef>): TaskItemModel {
  const ref = (id: string): MemberRef => refs.get(id) ?? { id, name: '?', color: FALLBACK }
  return {
    id: t.id,
    title: t.title,
    dueAt: t.due_at,
    status: t.status,
    priorityFlag: t.priority_flag,
    assignees: t.assignees.map(ref),
    stepsDone: t.steps.filter((s) => s.done_at).length,
    stepsTotal: t.steps.length,
    claimedBy: t.claimed_by ? ref(t.claimed_by) : null,
    origin: t.origin,
  }
}

export interface Viewer {
  id: string
  role: 'adult' | 'child'
}

/** Lists the viewer may add a task to: adults every list, children only `child_can_add = own`. */
export function addableLists(lists: TaskListView[], viewer: Viewer | null): TaskListView[] {
  if (!viewer) return []
  if (viewer.role === 'adult') return lists
  return lists.filter((l) => l.child_can_add === 'own')
}

/** Members a new task may be assigned to: children only themselves. */
export function assignableMembers<M extends { id: string }>(
  members: M[],
  viewer: Viewer | null,
): M[] {
  if (!viewer) return []
  return viewer.role === 'adult' ? members : members.filter((m) => m.id === viewer.id)
}

export interface TaskChip {
  id: string
  title: string
  done: boolean
  color: string
  taskId: string
}

/**
 * Task chips by household-local day. Done tasks stay only when their display is `always`
 * (D-046); `hidden` tasks and skipped ones never show.
 */
export function chipsByDay(
  tasks: TaskView[],
  refs: Map<string, MemberRef>,
  tz: string,
): Map<string, TaskChip[]> {
  const out = new Map<string, TaskChip[]>()
  for (const t of tasks) {
    if (!t.due_at || t.calendar_display === 'hidden') continue
    if (t.status === 'skipped' || t.status === 'suggested') continue
    if (t.status === 'done' && t.calendar_display !== 'always') continue
    const first = t.assignees[0]
    const key = dayKey(t.due_at, tz)
    const list = out.get(key) ?? []
    list.push({
      id: t.id,
      taskId: t.id,
      title: t.title,
      done: t.status === 'done',
      color: (first && refs.get(first)?.color) || FALLBACK,
    })
    out.set(key, list)
  }
  return out
}
