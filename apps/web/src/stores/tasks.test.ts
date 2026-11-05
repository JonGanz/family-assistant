import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '../api/http'
import { dispatchLive } from './live'
import { useTasksStore } from './tasks'
import { useListsStore } from './lists'
import { useRoutinesStore } from './routines'

vi.mock('../api/client', () => ({
  api: {
    tasks: vi.fn(),
    completeTask: vi.fn(),
    reopenTask: vi.fn(),
    patchTask: vi.fn(),
    claimTask: vi.fn(),
    taskLists: vi.fn(),
    routines: vi.fn(),
    routinesToday: vi.fn(),
    tickRoutineStep: vi.fn(),
  },
}))
import { api } from '../api/client'
const m = vi.mocked(api)

beforeEach(() => {
  localStorage.clear()
  setActivePinia(createPinia())
  vi.resetAllMocks()
})

describe('tasks store', () => {
  it('loads a tab with its query', async () => {
    m.tasks.mockResolvedValue({ items: [{ id: 't1' }] } as never)
    const s = useTasksStore()
    await s.load('now')
    expect(m.tasks).toHaveBeenCalledWith({ view: 'now', status: 'open' })
    expect(s.tabs.now).toHaveLength(1)
  })

  it('completing arms an undo that reopens, and expiry disarms it', async () => {
    m.tasks.mockResolvedValue({ items: [] })
    m.completeTask.mockResolvedValue({} as never)
    m.reopenTask.mockResolvedValue({} as never)
    const s = useTasksStore()
    await s.load('mine')
    await s.complete({ id: 't1', title: 'Dishes' })
    expect(m.completeTask).toHaveBeenCalledWith('t1', undefined)
    expect(s.undo).toEqual({ id: 't1', title: 'Dishes' })
    await s.undoLast()
    expect(m.reopenTask).toHaveBeenCalledWith('t1')
    expect(s.undo).toBeNull()
    await s.complete({ id: 't2', title: 'Bins' })
    s.expireUndo()
    expect(s.undo).toBeNull()
    await s.undoLast()
    expect(m.reopenTask).toHaveBeenCalledTimes(1)
  })

  it('flags a 409 on save instead of throwing', async () => {
    m.patchTask.mockRejectedValue(new ApiError(409, {}))
    const s = useTasksStore()
    const r = await s.save({ id: 't1', version: 1 } as never, {} as never)
    expect(r).toBeNull()
    expect(s.conflict).toBe(true)
  })
})

describe('live events', () => {
  it('loadMore appends the next page while the cursor is set', async () => {
    m.tasks.mockResolvedValueOnce({ items: [{ id: 'a' }], next_before: 'c1' } as never)
    m.tasks.mockResolvedValueOnce({ items: [{ id: 'b' }] } as never)
    const s = useTasksStore()
    await s.load('all')
    expect(s.nextBefore.all).toBe('c1')
    await s.loadMore('all')
    expect(m.tasks).toHaveBeenLastCalledWith({ view: 'all', status: 'open', before: 'c1' })
    expect(s.tabs.all?.map((t) => t.id)).toEqual(['a', 'b'])
    expect(s.nextBefore.all).toBeUndefined()
  })

  it('tasks.updated refetches only what is held; unrelated kinds do not', async () => {
    m.tasks.mockResolvedValue({ items: [] })
    m.taskLists.mockResolvedValue([])
    const t = useTasksStore()
    const l = useListsStore()
    await t.load('now')
    await l.load()
    m.tasks.mockClear()
    m.taskLists.mockClear()
    await dispatchLive('tasks.updated')
    expect(m.tasks).toHaveBeenCalledTimes(1)
    expect(m.taskLists).toHaveBeenCalledTimes(1)
    m.tasks.mockClear()
    await dispatchLive('weather.updated')
    expect(m.tasks).not.toHaveBeenCalled()
    await dispatchLive('resync')
    expect(m.tasks).toHaveBeenCalledTimes(1)
  })

  it('routines.updated refreshes today only when it is loaded', async () => {
    const r = useRoutinesStore()
    await dispatchLive('routines.updated')
    expect(m.routinesToday).not.toHaveBeenCalled()
    m.routinesToday.mockResolvedValue([])
    await r.loadToday()
    m.routinesToday.mockClear()
    await dispatchLive('routines.updated')
    expect(m.routinesToday).toHaveBeenCalledTimes(1)
  })
})

describe('tasks filter', () => {
  it('defaults to All with no signed-in member, and sends no member', async () => {
    m.tasks.mockResolvedValue({ items: [] })
    const s = useTasksStore()
    expect(s.filter).toBe('all')
    await s.load('all')
    expect(m.tasks).toHaveBeenCalledWith({ view: 'all', status: 'open' })
  })

  it('setFilter remembers the choice, restarts paging and reloads with member=', async () => {
    m.tasks.mockResolvedValue({
      items: [{ id: 't1' }],
      next_before: '2026-01-01T00:00:00Z',
    } as never)
    const s = useTasksStore()
    await s.load('all')
    expect(s.nextBefore.all).toBeTruthy()
    m.tasks.mockClear()
    m.tasks.mockResolvedValue({ items: [] })
    await s.setFilter('k')
    expect(m.tasks).toHaveBeenCalledWith({ view: 'all', status: 'open', member: 'k' })
    expect(s.nextBefore.all).toBeUndefined()
    expect(localStorage.getItem('fa.tasks.filter')).toBe('k')
  })

  it('does not filter suggestions and survives a throwing localStorage', async () => {
    m.tasks.mockResolvedValue({ items: [] })
    const s = useTasksStore()
    await s.setFilter('k')
    m.tasks.mockClear()
    await s.load('suggested')
    expect(m.tasks).toHaveBeenCalledWith({ view: 'all', status: 'suggested' })
    const spy = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    await expect(s.setFilter('all')).resolves.toBeUndefined()
    spy.mockRestore()
  })
})
