import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '../api/http'
import { authGuard } from '../router'
import { useHouseholdStore } from '../stores/household'
import { useSessionStore } from '../stores/session'
import RuleEditor from './parts/RuleEditor.vue'
import { VALIDATE_DEBOUNCE_MS } from './parts/useRuleValidation'
import SettingsHouseholdView from './SettingsHouseholdView.vue'

vi.mock('../api/client', () => ({
  api: {
    household: vi.fn(),
    patchHousehold: vi.fn(),
    validateRule: vi.fn(),
    me: vi.fn(),
  },
}))
import { api } from '../api/client'
const m = vi.mocked(api)

const household = {
  name: 'Example Household',
  tz: 'UTC',
  units: 'metric',
  calendar_past_days: 30,
  calendar_future_days: 400,
  weather_retention_days: 7,
  private_events_as_busy: true,
  dashboard_completion_override: true,
  google_oauth_enabled: false,
  version: 3,
  location: null,
  now_list_size: 7,
  now_weights: { due: 1, flag: 0.5, weather: 0.5 },
  quiet_from: '21:00',
  quiet_until: '07:00',
}

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
})
afterEach(() => vi.useRealTimers())

describe('household M2 fields', () => {
  it('saves the Now list size, weights and quiet hours', async () => {
    m.household.mockResolvedValue(household as never)
    m.patchHousehold.mockResolvedValue(household as never)
    const w = mount(SettingsHouseholdView)
    await flushPromises()
    await w.find('input[name=now_list_size]').setValue(9)
    await w.find('input[name=w_flag]').setValue(2)
    await w.find('input[name=quiet_from]').setValue('22:00')
    await w.find('form').trigger('submit')
    await flushPromises()
    expect(m.patchHousehold).toHaveBeenCalledWith(
      3,
      expect.objectContaining({
        now_list_size: 9,
        now_weights: { due: 1, flag: 2, weather: 0.5 },
        quiet_from: '22:00',
        quiet_until: '07:00',
      }),
    )
  })
})

describe('dashboard completion override', () => {
  it('shows the saved value and saves a change', async () => {
    m.household.mockResolvedValue(household as never)
    m.patchHousehold.mockResolvedValue(household as never)
    const w = mount(SettingsHouseholdView)
    await flushPromises()
    const box = w.find('input[name=dashboard_completion_override]')
    expect((box.element as HTMLInputElement).checked).toBe(true)
    await box.setValue(false)
    await w.find('form').trigger('submit')
    await flushPromises()
    expect(m.patchHousehold).toHaveBeenCalledWith(
      3,
      expect.objectContaining({ dashboard_completion_override: false }),
    )
  })
})

describe('409 conflict banner', () => {
  it('shows "changed elsewhere" and reloads on demand', async () => {
    m.household.mockResolvedValue(household as never)
    m.patchHousehold.mockRejectedValue(new ApiError(409, {}))
    const w = mount(SettingsHouseholdView)
    await flushPromises()
    await w.find('form').trigger('submit')
    await flushPromises()
    expect(m.patchHousehold).toHaveBeenCalledWith(
      3,
      expect.objectContaining({ name: 'Example Household' }),
    )
    expect(w.text()).toContain('changed elsewhere')
    expect(useHouseholdStore().conflict).toBe(true)
    m.household.mockClear()
    await w.find('.error-banner button').trigger('click')
    await flushPromises()
    expect(m.household).toHaveBeenCalledTimes(1)
    expect(w.text()).not.toContain('changed elsewhere')
  })
})

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

describe('router guard', () => {
  const to = (auth: 'user' | 'adult') => ({ meta: { auth }, fullPath: '/settings/status' }) as never
  const signIn = (role: string) =>
    m.me.mockResolvedValue({
      actor: { kind: 'user', member: { id: 'x', role } },
      household: { name: 'H' },
    } as never)

  it('lets an adult into settings', async () => {
    signIn('adult')
    expect(await authGuard(to('adult'))).toBe(true)
  })

  it('sends a child away from settings but allows user routes', async () => {
    signIn('child')
    expect(await authGuard(to('adult'))).toEqual({ name: 'home' })
    expect(await authGuard(to('user'))).toBe(true)
  })

  it('sends the signed-out to login with a redirect', async () => {
    m.me.mockRejectedValue(new ApiError(401, {}))
    expect(await authGuard(to('adult'))).toEqual({
      name: 'login',
      query: { redirect: '/settings/status' },
    })
    expect(useSessionStore().isUser).toBe(false)
  })
})
