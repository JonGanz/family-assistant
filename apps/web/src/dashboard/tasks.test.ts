import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'

vi.mock('../kiosk/taskApi', () => ({
  taskApi: {
    lists: vi.fn().mockResolvedValue([]),
    household: vi.fn().mockResolvedValue({ now_list_size: 7 }),
    complete: vi.fn().mockResolvedValue({}),
    reopen: vi.fn().mockResolvedValue({}),
    create: vi.fn().mockResolvedValue({}),
    tickStep: vi.fn().mockResolvedValue({}),
  },
}))
vi.mock('../api/client', () => ({
  api: {
    me: vi.fn(),
    dashboard: vi.fn().mockRejectedValue(new Error('x')),
    dismissAdvisory: vi.fn(),
  },
}))
import { taskApi } from '../kiosk/taskApi'
import { useDashboardStore } from '../stores/dashboard'
import DashboardCanvas from './DashboardCanvas.vue'
import LayoutEditor from './LayoutEditor.vue'
import TaskFlow from './TaskFlow.vue'
import { memberColumns, nowTasks, panelVisible, routineActive } from './tasksModel'
import { sampleRoutines, sampleTasks } from './preview'
import type { DashboardData, DashboardSettings } from './types'

const NOW = new Date('2026-10-24T12:00:00Z')

const member = (id: string, name: string, role = 'adult') => ({
  id,
  display_name: name,
  role,
  avatar: null,
  avatar_url: null,
  color: '#3366cc',
})

const task = (id: string, over: Record<string, unknown> = {}) => ({
  id,
  list: 'l1',
  list_name: 'Chores',
  title: `Task ${id}`,
  due_at: '2026-10-24T17:00:00Z',
  status: 'open',
  origin: 'manual',
  priority_flag: 'normal',
  calendar_display: 'hidden',
  assignees: [] as string[],
  steps: [],
  claimed_by: null,
  score: 1,
  ...over,
})

const run = (over: Record<string, unknown> = {}) => ({
  id: 'run1',
  routine: 'ro1',
  routine_name: 'Morning',
  member: 'm2',
  run_date: '2026-10-24',
  status: 'open',
  starts_at: '10:00',
  done_by: '13:00',
  steps: [
    { step: 's1', ord: 0, text: 'Brush teeth', done_at: null },
    { step: 's2', ord: 1, text: 'Pack bag', done_at: '2026-10-24T11:00:00Z' },
  ],
  done: 1,
  total: 2,
  ...over,
})

function data(over: Record<string, unknown> = {}): DashboardData {
  return {
    household: { name: 'Example Household', tz: 'UTC', units: 'metric', location: null },
    members: [member('m1', 'Alex'), member('m2', 'Sam', 'child')],
    events: [],
    advisories: [],
    weather: null,
    tasks: [],
    routines: [],
    settings: {
      orientation: 'landscape',
      theme: 'light',
      agenda_days: 2,
      members: 'all',
      panels: [],
    },
    ...over,
  } as unknown as DashboardData
}

const settings = (panels: unknown[]): DashboardSettings =>
  ({
    orientation: 'landscape',
    theme: 'light',
    agenda_days: 2,
    members: 'all',
    panels,
  }) as DashboardSettings

function canvas(d: DashboardData, panels: unknown[]) {
  return mount(DashboardCanvas, { props: { data: d, settings: settings(panels), now: NOW } })
}

beforeEach(() => {
  setActivePinia(createPinia())
  vi.clearAllMocks()
})

describe('now panel', () => {
  it('lists open tasks by score, limited to the Now size, filtered by options', () => {
    const d = data({
      now_list_size: 2,
      tasks: [
        task('a', { score: 1, assignees: ['m1'] }),
        task('b', { score: 3, assignees: ['m2'] }),
        task('c', { score: 2, assignees: ['m1'], list: 'l2' }),
        task('d', { score: 9, status: 'done' }),
      ],
    })
    expect(nowTasks(d, { kind: 'now' }).map((t) => t.id)).toEqual(['b', 'c'])
    expect(nowTasks(d, { kind: 'now', members: ['m1'] }).map((t) => t.id)).toEqual(['c', 'a'])
    expect(nowTasks(d, { kind: 'now', lists: ['l1'] }).map((t) => t.id)).toEqual(['b', 'a'])
    const w = canvas(d, [{ kind: 'now' }])
    expect(w.findAll('.task-item')).toHaveLength(2)
    expect(w.text()).toContain('Task b')
  })

  it('shows + only when a list allows dashboard adding', () => {
    const none = canvas(
      data({ task_lists: [{ id: 'l1', name: 'Chores', dashboard_add: false }] }),
      [{ kind: 'now' }],
    )
    expect(none.find('button.add').exists()).toBe(false)
    const some = canvas(data({ task_lists: [{ id: 'l1', name: 'Chores', dashboard_add: true }] }), [
      { kind: 'now' },
    ])
    expect(some.find('button.add').exists()).toBe(true)
  })

  it('is read-only while completion is off', () => {
    const d = data({ completion_enabled: false, tasks: [task('a')] })
    const w = canvas(d, [{ kind: 'now' }])
    expect(w.find('li.ro').attributes('inert')).toBeDefined()
  })
})

describe('member_tasks panel', () => {
  it('has a column per member with today and overdue tasks, plus Anyone', () => {
    const d = data({
      tasks: [
        task('a', { assignees: ['m1'] }),
        task('late', { assignees: ['m1'], due_at: '2026-10-20T10:00:00Z' }),
        task('later', { assignees: ['m1'], due_at: '2026-10-30T10:00:00Z' }),
        task('b', { assignees: ['m2'] }),
        task('free'),
      ],
    })
    const cols = memberColumns(d, d.members, { kind: 'member_tasks' }, NOW)
    expect(cols.map((c) => [c.key, c.tasks.map((t) => t.id)])).toEqual([
      ['m1', ['late', 'a']],
      ['m2', ['b']],
      ['anyone', ['free']],
    ])
    const w = canvas(d, [{ kind: 'member_tasks' }])
    expect(w.findAll('[data-member]')).toHaveLength(3)
  })
})

describe('routine panel', () => {
  it('is visible only inside its window in the household tz', () => {
    const d = data({ household: { name: 'x', tz: 'Pacific/Auckland', units: 'metric' } })
    // 12:00Z on Oct 24 is 01:00 on Oct 25 in Auckland (NZDT, +13)
    const r = run({ run_date: '2026-10-25', starts_at: '00:30', done_by: '02:00' })
    expect(routineActive(r as never, NOW, 'Pacific/Auckland')).toBe(true)
    expect(
      routineActive(
        run({ run_date: '2026-10-24', starts_at: '00:30', done_by: '02:00' }) as never,
        NOW,
        'Pacific/Auckland',
      ),
    ).toBe(false)
    expect(routineActive(run({ starts_at: '12:01' }) as never, NOW, 'UTC')).toBe(false)
    expect(routineActive(run({ done_by: '12:00' }) as never, NOW, 'UTC')).toBe(false)
    expect(panelVisible({ kind: 'routine' }, { ...d, routines: [r] } as never, NOW)).toBe(true)
    expect(panelVisible({ kind: 'routine' }, d, NOW)).toBe(false)
  })

  it('shows outside the window when pinned, and drops out of the layout when idle', () => {
    const idle = run({ starts_at: '18:00', done_by: '19:00' })
    const d = data({ routines: [idle] })
    expect(
      canvas(d, [{ kind: 'routine' }])
        .find('.routine')
        .exists(),
    ).toBe(false)
    expect(
      canvas(d, [{ kind: 'routine', pinned: true }])
        .find('.routine')
        .exists(),
    ).toBe(true)
  })

  it('renders step buttons, progress and the done state, and emits ticks', async () => {
    const w = canvas(data({ routines: [run()] }), [{ kind: 'routine' }])
    const steps = w.findAll('button.step')
    expect(steps).toHaveLength(2)
    expect(w.find('[role=progressbar]').attributes('aria-valuenow')).toBe('1')
    await steps[0]!.trigger('click')
    expect(w.emitted('tick')![0]).toEqual(['run1', 's1', true])
    await steps[1]!.trigger('click')
    expect(w.emitted('tick')![1]).toEqual(['run1', 's2', false])

    const done = run({
      steps: [{ step: 's1', ord: 0, text: 'Brush teeth', done_at: '2026-10-24T11:00:00Z' }],
    })
    expect(canvas(data({ routines: [done] }), [{ kind: 'routine' }]).text()).toContain('Well done')
  })
})

describe('agenda and week', () => {
  const dated = [
    task('kd', { calendar_display: 'always', title: 'School photo day' }),
    task('ud', { calendar_display: 'until_done', title: 'Return library book' }),
    task('hid', { calendar_display: 'hidden', title: 'Secret' }),
  ]
  it('agenda adds key dates only', () => {
    const w = canvas(data({ tasks: dated }), [{ kind: 'agenda' }])
    expect(w.findAll('[data-testid=key-date]')).toHaveLength(1)
    expect(w.text()).toContain('School photo day')
    expect(w.text()).not.toContain('Return library book')
  })
  it('week adds until_done and always chips on their day', () => {
    const w = canvas(data({ tasks: dated }), [{ kind: 'week' }])
    const chips = w.find('[data-day="2026-10-24"]').findAll('[data-testid=task-chip]')
    expect(chips.map((c) => c.text())).toEqual(['⚑ School photo day', '⚑ Return library book'])
  })
})

describe('completion flow', () => {
  function flow(d: DashboardData) {
    const store = useDashboardStore()
    store.snapshot = d
    vi.spyOn(store, 'refresh').mockResolvedValue()
    return { store, w: mount(TaskFlow) }
  }

  it('asks who did it (preselecting the single assignee), completes, then offers undo', async () => {
    const { w } = flow(data({ tasks: [task('a', { assignees: ['m2'] })] }))
    ;(w.vm as unknown as { beginComplete(id: string): void }).beginComplete('a')
    await nextTick()
    expect(w.find('[data-testid=complete-picker]').exists()).toBe(true)
    expect(w.text()).toContain('Who did it?')
    const members = w.findAll('button.member')
    expect(members.map((b) => b.classes('pre'))).toEqual([false, true])
    await members[1]!.trigger('click')
    await vi.waitFor(() => expect(taskApi.complete).toHaveBeenCalledWith('a', 'm2'))
    await vi.waitFor(() => expect(w.find('.toast').exists()).toBe(true))
    expect(w.find('[data-testid=countdown]').text()).toBe('10s')
    await w.find('button.undo').trigger('click')
    await vi.waitFor(() => expect(taskApi.reopen).toHaveBeenCalledWith('a', 'm2'))
    expect(w.find('.toast').exists()).toBe(false)
  })

  it('does nothing when dashboard completion is off', async () => {
    const { w } = flow(data({ completion_enabled: false, tasks: [task('a')] }))
    ;(w.vm as unknown as { beginComplete(id: string): void }).beginComplete('a')
    await nextTick()
    expect(w.find('[data-testid=complete-picker]').exists()).toBe(false)
  })

  it('cancelling the picker completes nothing and resets the checkbox', async () => {
    const { store, w } = flow(data({ tasks: [task('a')] }))
    ;(w.vm as unknown as { beginComplete(id: string): void }).beginComplete('a')
    await nextTick()
    await w.find('button.cancel').trigger('click')
    expect(taskApi.complete).not.toHaveBeenCalled()
    expect(store.snapshot?.epoch).toBe(1)
  })

  it('adds a task from a dashboard_add list, asking who adds it', async () => {
    const { w } = flow(
      data({
        task_lists: [
          { id: 'l1', name: 'Chores', dashboard_add: true },
          { id: 'l2', name: 'Private', dashboard_add: false },
        ],
      }),
    )
    ;(w.vm as unknown as { beginAdd(): void }).beginAdd()
    await nextTick()
    expect(w.findAll('[data-testid=add-list] option')).toHaveLength(1)
    await w.find('[data-testid=add-title]').setValue('Buy milk')
    await w.find('[data-testid=add-assignee]').setValue('m1')
    await w.find('form').trigger('submit')
    expect(w.text()).toContain('Who is adding it?')
    await w.findAll('button.member')[1]!.trigger('click')
    await vi.waitFor(() => expect(taskApi.create).toHaveBeenCalled())
    expect(vi.mocked(taskApi.create).mock.calls[0]![0]).toMatchObject({
      list: 'l1',
      title: 'Buy milk',
      assignees: ['m1'],
      member: 'm2',
    })
  })
})

describe('layout editor and preview', () => {
  it('offers pinned, members and lists options for the new panels', async () => {
    const members = [member('m1', 'Alex'), member('m2', 'Sam')] as never
    const lists = [{ id: 'l1', name: 'Chores' }] as never
    const w = mount(LayoutEditor, {
      props: {
        modelValue: settings([{ kind: 'now' }, { kind: 'routine' }]),
        members,
        lists,
      },
    })
    expect(w.find('[data-testid=pinned]').exists()).toBe(true)
    await w.find('li[data-panel=now] [data-opt="lists:l1"]').setValue(false)
    const emitted = w.emitted('update:modelValue')!
    expect(emitted).toHaveLength(1)
    await w.find('li[data-panel=now] [data-opt="members:m1"]').setValue(false)
    const next = w.emitted('update:modelValue')![1]![0] as DashboardSettings
    expect(next.panels[0]!['members']).toEqual(['m2'])
  })

  it('sample data fills the task panels', () => {
    const ms = [member('m1', 'Alex'), member('m2', 'Sam', 'child')] as never
    const d = data({
      tasks: sampleTasks(ms, [], NOW, 'UTC'),
      routines: sampleRoutines(ms, NOW, 'UTC'),
    })
    const w = canvas(d, [{ kind: 'now' }, { kind: 'routine' }, { kind: 'member_tasks' }])
    expect(w.find('.now .task-item').exists()).toBe(true)
    expect(w.find('.routine').exists()).toBe(true)
  })
})
