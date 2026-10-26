import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { emptyFilters, filtersToQuery, useMailStore } from './mail'

vi.mock('../api/client', () => ({
  api: {
    messages: vi.fn(),
    message: vi.fn(),
    messageAction: vi.fn(),
    undoMailAction: vi.fn(),
    labelMessage: vi.fn(),
    labellingNext: vi.fn(),
    setEvalSample: vi.fn(),
    makeTask: vi.fn(),
    categories: vi.fn(),
    mailSources: vi.fn(),
  },
}))
import { api } from '../api/client'
const m = vi.mocked(api)

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
})

describe('mail filters', () => {
  it('drops empty filters from the query', () => {
    expect(filtersToQuery(emptyFilters())).toEqual({
      category: undefined,
      member: undefined,
      source: undefined,
      q: undefined,
      include_ignored: undefined,
      before: undefined,
    })
    const f = { ...emptyFilters(), category: 'school', q: ' hi ', includeIgnored: true }
    expect(filtersToQuery(f, 'c1')).toMatchObject({
      category: 'school',
      q: 'hi',
      include_ignored: true,
      before: 'c1',
    })
  })
})

describe('mail store', () => {
  it('pages with the cursor and appends', async () => {
    m.messages
      .mockResolvedValueOnce({ items: [{ id: 'a' }], next_before: '2026-01-01T00:00:00Z' } as never)
      .mockResolvedValueOnce({ items: [{ id: 'b' }], next_before: null } as never)
    const s = useMailStore()
    await s.load()
    expect(s.nextBefore).toBe('2026-01-01T00:00:00Z')
    await s.more()
    expect(m.messages).toHaveBeenLastCalledWith(
      expect.objectContaining({ before: '2026-01-01T00:00:00Z' }),
    )
    expect(s.items.map((i) => i.id)).toEqual(['a', 'b'])
    expect(s.nextBefore).toBeNull()
  })

  it('keeps the error when the list fails', async () => {
    m.messages.mockRejectedValue(new Error('boom'))
    const s = useMailStore()
    await s.load()
    expect(s.error).toBe('boom')
  })

  it('fix replaces the detail with the server answer', async () => {
    m.labelMessage.mockResolvedValue({ summary: { id: 'x' } } as never)
    const s = useMailStore()
    await s.fix('x', { decision_point: 'email.category', value: 'school' })
    expect(m.labelMessage).toHaveBeenCalledWith('x', {
      decision_point: 'email.category',
      value: 'school',
    })
    expect(s.detail?.summary.id).toBe('x')
  })

  it('undo reloads the detail', async () => {
    m.undoMailAction.mockResolvedValue({} as never)
    m.message.mockResolvedValue({ summary: { id: 'x' } } as never)
    const s = useMailStore()
    await s.undo('x', 'act1')
    expect(m.undoMailAction).toHaveBeenCalledWith('act1')
    expect(m.message).toHaveBeenCalledWith('x', undefined)
  })

  it('labelling: empty queue and eval toggle', async () => {
    m.labellingNext.mockResolvedValueOnce(undefined)
    const s = useMailStore()
    await s.nextToLabel()
    expect(s.labelEmpty).toBe(true)
    m.labellingNext.mockResolvedValueOnce({
      message: { summary: { id: 'x', in_eval_sample: false } },
      progress: { in_sample: 1, target: 200, labelled: 0 },
    } as never)
    await s.nextToLabel('x')
    expect(s.labelEmpty).toBe(false)
    m.setEvalSample.mockResolvedValue(undefined as never)
    await s.setEvalSample('x', true)
    expect(s.label?.message.summary.in_eval_sample).toBe(true)
  })
})
