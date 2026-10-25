import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useSessionStore } from '../stores/session'
import { toInstant, fromInstant } from './parts/dateInput'
import TaskAddForm from './parts/TaskAddForm.vue'
import TasksView from './TasksView.vue'

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: vi.fn() }),
  useRoute: () => ({ query: {} }),
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
})

describe('TaskAddForm', () => {
  it('offers a child only child_can_add lists and assigns to themself', async () => {
    m.createTask.mockResolvedValue({} as never)
    m.tasks.mockResolvedValue([])
    const w = mount(TaskAddForm, {
      props: {
        lists: lists as never,
        members: members as never,
        viewer: { id: 'k', role: 'child' },
        tz: 'UTC',
      },
    })
    const options = w.findAll('select[name=list] option').map((o) => o.text())
    expect(options).toEqual(['Homework'])
    expect(w.find('select[name=assignee]').exists()).toBe(false)
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
    expect(w.findAll('select[name=list] option')).toHaveLength(2)
    expect(w.findAll('select[name=assignee] option')).toHaveLength(3)
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
    m.tasks.mockResolvedValue([t] as never)
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
    expect(w.text()).toContain('Completed “Dishes”')
    await w.find('button.undo').trigger('click')
    await flushPromises()
    expect(m.reopenTask).toHaveBeenCalledWith('t1')
    expect(w.find('button.undo').exists()).toBe(false)
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
