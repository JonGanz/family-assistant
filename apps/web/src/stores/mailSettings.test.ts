import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('../api/mailSettings', () => ({
  mailApi: {
    sources: vi.fn(),
    categories: vi.fn(),
    rules: vi.fn(),
    signals: vi.fn(),
    patchSource: vi.fn(),
    patchCategory: vi.fn(),
    patchSignal: vi.fn(),
    createRule: vi.fn(),
  },
}))
import { ApiError } from '../api/http'
import { mailApi } from '../api/mailSettings'
import { useMailSettingsStore } from './mailSettings'

const m = vi.mocked(mailApi, true)

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
  m.sources.mockResolvedValue([{ id: 's1', name: 'Home', version: 3 }] as never)
  m.categories.mockResolvedValue([{ id: 'c1', key: 'school', version: 2 }] as never)
  m.rules.mockResolvedValue([])
  m.signals.mockResolvedValue([])
})

describe('mail settings store', () => {
  it('loads sources, categories, rules and signals', async () => {
    const s = useMailSettingsStore()
    await s.load()
    expect(s.sources).toHaveLength(1)
    expect(s.categories[0]?.key).toBe('school')
    expect(s.loaded).toBe(true)
    expect(s.error).toBeNull()
  })

  it('records a load failure', async () => {
    m.sources.mockRejectedValue(new Error('boom'))
    const s = useMailSettingsStore()
    await s.load()
    expect(s.error).toBe('boom')
  })

  it('pause sends state=paused with the source version, then reloads', async () => {
    m.patchSource.mockResolvedValue({} as never)
    const s = useMailSettingsStore()
    await s.load()
    expect(await s.pause(s.sources[0]!)).toBe(true)
    expect(m.patchSource).toHaveBeenCalledWith('s1', 3, { state: 'paused' })
    expect(m.sources).toHaveBeenCalledTimes(2)
  })

  it('archive sends archived=true', async () => {
    m.patchCategory.mockResolvedValue({} as never)
    const s = useMailSettingsStore()
    await s.load()
    await s.archiveCategory(s.categories[0]!)
    expect(m.patchCategory).toHaveBeenCalledWith('c1', 2, { archived: true })
  })

  it('a 409 sets conflict instead of throwing', async () => {
    m.patchSource.mockRejectedValue(new ApiError(409, { status: 409 }))
    const s = useMailSettingsStore()
    await s.load()
    expect(await s.resume(s.sources[0]!)).toBe(false)
    expect(s.conflict).toBe(true)
  })

  it('other errors propagate (invalid rule: 422)', async () => {
    m.createRule.mockRejectedValue(new ApiError(422, { status: 422, detail: 'bad' }))
    const s = useMailSettingsStore()
    await expect(
      s.createRule({
        decision_point: 'email.category',
        name: 'x',
        conditions: [],
        value: 'a' as never,
      }),
    ).rejects.toMatchObject({ status: 422 })
  })
})
