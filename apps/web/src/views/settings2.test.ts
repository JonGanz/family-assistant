import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import RuleEditor from './parts/RuleEditor.vue'
import { VALIDATE_DEBOUNCE_MS } from './parts/useRuleValidation'

vi.mock('../api/client', () => ({
  api: {
    validateRule: vi.fn(),
  },
}))
import { api } from '../api/client'
const m = vi.mocked(api)

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
})
afterEach(() => vi.useRealTimers())

describe('rule editor', () => {
  it('debounces validation by 300 ms and shows the error at its position', async () => {
    vi.useFakeTimers()
    m.validateRule.mockResolvedValue({
      ok: false,
      error: { position: 4, message: 'unknown field' },
    } as never)
    const w = mount(RuleEditor, { props: { rule: null } })
    const input = w.find('input[name="condition"]')
    await input.setValue('min(')
    await input.setValue('min(tmp')
    await input.setValue('min(tmpx, today) < 1')
    expect(m.validateRule).not.toHaveBeenCalled()
    await vi.advanceTimersByTimeAsync(VALIDATE_DEBOUNCE_MS - 1)
    expect(m.validateRule).not.toHaveBeenCalled()
    await vi.advanceTimersByTimeAsync(1)
    expect(m.validateRule).toHaveBeenCalledTimes(1)
    expect(m.validateRule.mock.calls[0]![0]).toBe('min(tmpx, today) < 1')
    const err = w.get('[data-testid="rule-error"]').text()
    expect(err).toContain('unknown field')
    expect(err).toContain('    ^')
    expect(w.get('button[type="submit"]').attributes('disabled')).toBeDefined()
  })

  it('shows the preview for a valid rule', async () => {
    vi.useFakeTimers()
    m.validateRule.mockResolvedValue({
      ok: true,
      preview: { matches: true, value: -2, when: 'night' },
    } as never)
    const w = mount(RuleEditor, { props: { rule: null } })
    await w.find('input[name="condition"]').setValue('min(temp_c, tonight) <= 0')
    await vi.advanceTimersByTimeAsync(VALIDATE_DEBOUNCE_MS)
    expect(w.get('[data-testid="rule-preview"]').text()).toContain('would fire')
  })
})
