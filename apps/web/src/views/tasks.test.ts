import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useSessionStore } from '../stores/session'
import { resetSnackbar, useSnackbar } from '../ui/composables/useSnackbar'
import { toInstant, fromInstant } from './parts/dateInput'
import TaskAddForm from './parts/TaskAddForm.vue'
import TaskDetailView from './TaskDetailView.vue'
import TasksView from './TasksView.vue'

const leaveGuards: (() => unknown)[] = []
vi.mock('vue-router', () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn() }),
  useRoute: () => ({ query: {}, params: { id: 't1' } }),
  onBeforeRouteLeave: (fn: () => unknown) => leaveGuards.push(fn),
}))
vi.mock('../api/client', () => ({
  api: {
    tasks: vi.fn(),
    members: vi.fn(),
    taskLists: vi.fn(),
    createTask: vi.fn(),
    completeTask: vi.fn(),
    reopenTask: vi.fn(),
    household: vi.fn(),
    task: vi.fn(),
    patchTask: vi.fn(),
  },
}))
import { api } from '../api/client'
const m = vi.mocked(api)

const lists = [
  { id: 'chores', name: 'Chores', child_can_add: 'none', sort: 0 },
  { id: 'hw', name: 'Homework', child_can_add: 'own', sort: 1 },
]
const members = [
  { id: 'a', display_name: 'Alex', role: 'adult', color: '#f00' },
  { id: 'k', display_name: 'Kai', role: 'child', color: '#0f0' },
]

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
  resetSnackbar()
  leaveGuards.length = 0
  localStorage.clear()
})

describe('TaskAddForm', () => {
  it('offers a child only child_can_add lists and assigns to themself', async () => {
    m.createTask.mockResolvedValue({} as never)
    m.tasks.mockResolvedValue({ items: [] })
    const w = mount(TaskAddForm, {
      props: {
        lists: lists as never,
        members: members as never,
        viewer: { id: 'k', role: 'child' },
        tz: 'UTC',
      },
    })
    const options = w
      .findAll('select')[0]!
      .findAll('option')
      .map((o) => o.text())
    expect(options).toEqual(['Homework'])
    expect(w.findAll('select')).toHaveLength(1)
    await w.find('input[name=title]').setValue('Read chapter 3')
    await w.find('form').trigger('submit')
    await flushPromises()
    expect(m.createTask).toHaveBeenCalledWith(
      expect.objectContaining({ list: 'hw', title: 'Read chapter 3', assignees: ['k'] }),
    )
  })

  it('lets an adult pick any list and assignee', () => {
    const w = mount(TaskAddForm, {
      props: {
        lists: lists as never,
        members: members as never,
        viewer: { id: 'a', role: 'adult' },
        tz: 'UTC',
      },
    })
    expect(w.findAll('select')[0]!.findAll('option')).toHaveLength(2)
    expect(w.findAll('select')[1]!.findAll('option')).toHaveLength(3)
  })
})

describe('complete and undo', () => {
  it('completing shows the undo toast and Undo reopens the task', async () => {
    const t = {
      id: 't1',
      list: 'chores',
      list_name: 'Chores',
      title: 'Dishes',
      status: 'open',
      origin: 'manual',
      priority_flag: 'normal',
      assignees: ['a'],
      steps: [],
      claimed_by: null,
      due_at: null,
      version: 1,
    }
    m.members.mockResolvedValue(members as never)
    m.taskLists.mockResolvedValue(lists as never)
    m.household.mockResolvedValue({ tz: 'UTC' } as never)
    m.tasks.mockResolvedValue({ items: [t] } as never)
    m.completeTask.mockResolvedValue({} as never)
    m.reopenTask.mockResolvedValue({} as never)
    const s = useSessionStore()
    s.me = {
      actor: { kind: 'user', member: { id: 'a', role: 'adult' } },
      household: { tz: 'UTC' },
    } as never
    const w = mount(TasksView, { global: { stubs: { RouterLink: true } } })
    await flushPromises()
    await w.find('input.check').trigger('change')
    await flushPromises()
    expect(m.completeTask).toHaveBeenCalledWith('t1', undefined)
    const snack = useSnackbar().state.current
    expect(snack?.text).toBe('Completed “Dishes”')
    expect(snack?.timeoutMs).toBe(10_000)
    await snack!.action!.run()
    await flushPromises()
    expect(m.reopenTask).toHaveBeenCalledWith('t1')
  })
})

describe('date inputs', () => {
  it('round-trips household-local date and time through an instant', () => {
    const iso = toInstant('2026-10-26', '18:30', 'Europe/London')
    expect(iso).toBe('2026-10-26T18:30:00.000Z')
    expect(fromInstant(iso, 'Europe/London')).toEqual({ date: '2026-10-26', time: '18:30' })
    expect(toInstant('2026-07-01', '09:00', 'Europe/London')).toBe('2026-07-01T08:00:00.000Z')
    expect(toInstant('', '09:00', 'UTC')).toBeNull()
    expect(toInstant('2026-10-26', '', 'UTC')).toBe('2026-10-26T23:59:00.000Z')
  })
})

const task = {
  id: 't1',
  list: 'chores',
  list_name: 'Chores',
  title: 'Dishes',
  notes: null,
  status: 'open',
  origin: 'manual',
  priority_flag: 'normal',
  calendar_display: 'hidden',
  assignees: ['a'],
  steps: [],
  claimed_by: null,
  due_at: null,
  version: 1,
}

function signIn(id = 'a', role = 'adult') {
  useSessionStore().me = {
    actor: { kind: 'user', member: { id, role } },
    household: { tz: 'UTC' },
  } as never
}

async function mountTasks() {
  m.members.mockResolvedValue(members as never)
  m.taskLists.mockResolvedValue(lists as never)
  m.household.mockResolvedValue({ tz: 'UTC' } as never)
  m.tasks.mockResolvedValue({ items: [task] } as never)
  signIn()
  const w = mount(TasksView, { global: { stubs: { RouterLink: true } } })
  await flushPromises()
  return w
}
const chip = (w: ReturnType<typeof mount>, name: string) =>
  w.findAll('.filter button').find((b) => b.text() === name)!

describe('Tasks filter chips', () => {
  it('defaults to Mine and sends member=', async () => {
    const w = await mountTasks()
    expect(m.tasks).toHaveBeenCalledWith(expect.objectContaining({ member: 'a' }))
    expect(chip(w, 'Mine').attributes('aria-checked')).toBe('true')
    expect(w.findAll('.filter button').map((b) => b.text())).toEqual(['All', 'Mine', 'Alex', 'Kai'])
  })

  it('All sends no member, a member chip sends that member, and the choice persists', async () => {
    const w = await mountTasks()
    m.tasks.mockClear()
    await chip(w, 'All').trigger('click')
    await flushPromises()
    expect(m.tasks.mock.calls.every(([q]) => !(q as { member?: string }).member)).toBe(true)
    expect(localStorage.getItem('fa.tasks.filter')).toBe('all')
    m.tasks.mockClear()
    await chip(w, 'Kai').trigger('click')
    await flushPromises()
    expect(m.tasks).toHaveBeenCalledWith(expect.objectContaining({ member: 'k' }))
    expect(localStorage.getItem('fa.tasks.filter')).toBe('k')
  })

  it('restores the stored choice and falls back to Mine for an unknown member', async () => {
    localStorage.setItem('fa.tasks.filter', 'gone')
    const w = await mountTasks()
    expect(chip(w, 'Mine').attributes('aria-checked')).toBe('true')
    expect(m.tasks).toHaveBeenCalledWith(expect.objectContaining({ member: 'a' }))
  })
})

describe('TaskDetailView', () => {
  async function mountDetail() {
    m.members.mockResolvedValue(members as never)
    m.taskLists.mockResolvedValue(lists as never)
    m.household.mockResolvedValue({ tz: 'UTC' } as never)
    m.task.mockResolvedValue(task as never)
    m.tasks.mockResolvedValue({ items: [] } as never)
    signIn()
    const w = mount(TaskDetailView, { global: { stubs: { RouterLink: true } } })
    await flushPromises()
    return w
  }

  it('shows the Save bar once the form is dirty and saves one PATCH', async () => {
    m.patchTask.mockResolvedValue({ ...task, title: 'Dishes now', version: 2 } as never)
    const w = await mountDetail()
    expect(w.find('.ui-savebar').exists()).toBe(false)
    await w.find('input[name=title]').setValue('Dishes now')
    expect(w.find('.ui-savebar').exists()).toBe(true)
    await w
      .findAll('.ui-savebar button')
      .find((b) => b.text() === 'Save')!
      .trigger('click')
    await flushPromises()
    expect(m.patchTask).toHaveBeenCalledWith(
      't1',
      1,
      expect.objectContaining({ title: 'Dishes now', assignees: ['a'] }),
    )
    expect(w.find('.ui-savebar').exists()).toBe(false)
  })

  it('applies the priority flag at once with an Undo snackbar', async () => {
    m.patchTask.mockResolvedValue({ ...task, priority_flag: 'high', version: 2 } as never)
    const w = await mountDetail()
    await w.find('button[role=switch]').trigger('click')
    await flushPromises()
    expect(m.patchTask).toHaveBeenCalledWith('t1', 1, { priority_flag: 'high' })
    const snack = useSnackbar().state.current
    expect(snack?.action?.label).toBe('Undo')
    expect(w.find('.ui-savebar').exists()).toBe(false)
    await snack!.action!.run()
    expect(m.patchTask).toHaveBeenLastCalledWith('t1', 2, { priority_flag: 'normal' })
  })
})
