import type { TaskListInput, TaskListPatch, TaskListView } from '../../api/client'

export type ListKind = TaskListInput['kind']

/** Mirrors `fa_domain::tasks::list_defaults` (arch/Tasks.md §3.2); the server stays the authority. */
export function listDefaults(
  kind: ListKind,
): Pick<
  TaskListInput,
  'completion_policy' | 'child_can_add' | 'dashboard_add' | 'calendar_default'
> {
  switch (kind) {
    case 'chores':
      return {
        completion_policy: 'assignee',
        child_can_add: 'none',
        dashboard_add: false,
        calendar_default: 'hidden',
      }
    case 'homework':
      return {
        completion_policy: 'assignee',
        child_can_add: 'own',
        dashboard_add: false,
        calendar_default: 'until_done',
      }
    case 'general':
      return {
        completion_policy: 'assignee',
        child_can_add: 'none',
        dashboard_add: true,
        calendar_default: 'hidden',
      }
    case 'email':
      return {
        completion_policy: 'assignee',
        child_can_add: 'none',
        dashboard_add: false,
        calendar_default: 'until_done',
      }
  }
}

export interface ListForm extends Omit<TaskListInput, 'claim_ttl_minutes' | 'color'> {
  claim: 'end_of_day' | 'minutes'
  claimMinutes: number
  color: string
  useColor: boolean
}

export function emptyListForm(sort: number): ListForm {
  const kind: ListKind = 'general'
  return {
    name: '',
    kind,
    ...listDefaults(kind),
    visible_on_dashboard: true,
    sort,
    claim: 'end_of_day',
    claimMinutes: 120,
    color: '#2f6fed',
    useColor: false,
  }
}

export function listFormFrom(l: TaskListView): ListForm {
  return {
    name: l.name,
    kind: l.kind,
    completion_policy: l.completion_policy,
    child_can_add: l.child_can_add,
    dashboard_add: l.dashboard_add,
    calendar_default: l.calendar_default,
    visible_on_dashboard: l.visible_on_dashboard,
    sort: l.sort,
    claim: l.claim_ttl_minutes == null ? 'end_of_day' : 'minutes',
    claimMinutes: l.claim_ttl_minutes ?? 120,
    color: l.color ?? '#2f6fed',
    useColor: !!l.color,
  }
}

export function listInput(f: ListForm): TaskListInput {
  return {
    name: f.name.trim(),
    kind: f.kind,
    completion_policy: f.completion_policy,
    child_can_add: f.child_can_add,
    dashboard_add: f.dashboard_add,
    calendar_default: f.calendar_default,
    claim_ttl_minutes: f.claim === 'minutes' ? Math.max(1, Math.floor(f.claimMinutes) || 1) : null,
    visible_on_dashboard: f.visible_on_dashboard,
    color: f.useColor ? f.color : null,
    sort: f.sort,
  }
}

export function listPatch(f: ListForm): TaskListPatch {
  return listInput(f)
}
