import { addDays, dayKey, startOfDay } from '../lib/time'
import { api, type Member } from '../api/client'
import { taskApi } from '../kiosk/taskApi'
import type {
  AdvisoryView,
  DashboardData,
  MemberPublic,
  RoutineRunView,
  TaskListView,
  TaskView,
  WeatherView,
} from './types'

export type PreviewData = Omit<DashboardData, 'settings'>

/** Household data for the editor preview, fetched with the user session (not a device token). */
export async function loadPreviewData(): Promise<PreviewData> {
  const h = await api.household()
  const today = dayKey(new Date(), h.tz)
  const from = startOfDay(h.tz, today).toISOString()
  const to = startOfDay(h.tz, addDays(today, 14)).toISOString()
  const [members, events, weather, advisories, lists] = await Promise.all([
    api.members(),
    api.events(from, to),
    api.weather().catch((): WeatherView | null => null),
    api.advisories().catch((): AdvisoryView[] => []),
    taskApi.lists(false).catch((): TaskListView[] => []),
  ])
  const pub = members.map(publicMember)
  return {
    household: {
      name: h.name,
      tz: h.tz,
      units: h.units,
      location: h.location ?? null,
      now_list_size: h.now_list_size,
      completion_override: true,
    },
    members: pub,
    events,
    weather,
    advisories,
    tasks: sampleTasks(pub, lists, new Date(), h.tz),
    routines: sampleRoutines(pub, new Date(), h.tz),
    task_lists: lists,
    now_list_size: h.now_list_size,
  }
}

export function publicMember(m: Member): MemberPublic {
  return {
    id: m.id,
    display_name: m.display_name,
    role: m.role,
    avatar: m.avatar ?? null,
    avatar_url: m.avatar_url ?? null,
    color: m.color ?? null,
  }
}

/** Fictional tasks so the editor preview shows the task panels whatever the household holds. */
export function sampleTasks(
  members: MemberPublic[],
  lists: TaskListView[],
  now: Date,
  tz: string,
): TaskView[] {
  const titles = [
    'Take out the recycling',
    'Water the plants',
    'Tidy the living room',
    'Plan the weekend trip',
    'Empty the dishwasher',
  ]
  const today = dayKey(now, tz)
  return titles.map((title, i) => {
    const m = members[i % Math.max(1, members.length)]
    const day = startOfDay(tz, addDays(today, i === 3 ? 4 : 0))
    return {
      id: `sample-task-${i}`,
      list: lists[0]?.id ?? 'sample-list',
      list_name: lists[0]?.name ?? 'Chores',
      definition: null,
      title,
      notes: null,
      due_at: new Date(day.getTime() + (17 + i) * 3_600_000).toISOString(),
      visible_from: null,
      status: 'open',
      origin: 'manual',
      priority_flag: i === 0 ? 'high' : 'normal',
      calendar_display: i === 3 ? 'always' : 'hidden',
      assignees: m ? [m.id] : [],
      assignee_away: false,
      steps: [],
      claimed_by: null,
      claimed_until: null,
      completed_at: null,
      completed_by: null,
      undo_until: null,
      score: 5 - i,
      version: 1,
    } as TaskView
  })
}

/** A sample routine whose window surrounds `now`, so the panel is visible in the preview. */
export function sampleRoutines(members: MemberPublic[], now: Date, tz: string): RoutineRunView[] {
  const m = members.find((x) => x.role === 'child') ?? members[0]
  if (!m) return []
  const steps = ['Get dressed', 'Brush teeth', 'Pack the bag']
  return [
    {
      id: 'sample-run',
      routine: 'sample-routine',
      routine_name: 'Morning routine',
      member: m.id,
      run_date: dayKey(now, tz),
      status: 'open',
      starts_at: '00:00',
      done_by: '23:59',
      steps: steps.map((text, i) => ({
        step: `sample-step-${i}`,
        ord: i,
        text,
        target_time: null,
        done_at: i === 0 ? now.toISOString() : null,
      })),
      done: 1,
      total: steps.length,
    } as RoutineRunView,
  ]
}
