import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises } from '@vue/test-utils'
import { useSessionStore } from '../stores/session'
import { resetSnackbar, useSnackbar } from '../ui/composables/useSnackbar'
import RuleEditor from './parts/RuleEditor.vue'
import { VALIDATE_DEBOUNCE_MS } from './parts/useRuleValidation'
import SettingsNotificationsView from './SettingsNotificationsView.vue'
import SettingsStatusView from './SettingsStatusView.vue'

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: vi.fn() }),
  useRoute: () => ({ path: '/settings/notifications', query: {} }),
  onBeforeRouteLeave: vi.fn(),
}))
vi.mock('../api/client', () => ({
  api: {
    validateRule: vi.fn(),
    members: vi.fn(),
    household: vi.fn(),
    notifyPrefs: vi.fn(),
    putNotifyPrefs: vi.fn(),
    status: vi.fn(),
  },
}))
import { api } from '../api/client'
const m = vi.mocked(api)

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
  resetSnackbar()
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

const prefs = {
  member: 'm1',
  version: 1,
  enabled_triggers: ['assigned'],
  due_soon_minutes: 60,
  nudge_minutes: 15,
  quiet_from: null,
  quiet_until: null,
  copy_to_member: null,
  approval_mode: 'batched',
  server_configured: true,
  subscribe_url: null,
}

async function mountNotifications() {
  useSessionStore().me = {
    actor: { kind: 'user', member: { id: 'm1', role: 'adult' } },
  } as never
  m.members.mockResolvedValue([{ id: 'm1', display_name: 'Alex', role: 'adult' }] as never)
  m.household.mockResolvedValue({ quiet_from: '21:00', quiet_until: '07:00', version: 1 } as never)
  m.notifyPrefs.mockResolvedValue(prefs as never)
  const w = mount(SettingsNotificationsView, { global: { stubs: { RouterLink: true } } })
  await flushPromises()
  return w
}

describe('Settings, Notifications', () => {
  it('a trigger switch saves at once and offers Undo', async () => {
    m.putNotifyPrefs.mockResolvedValue({
      ...prefs,
      version: 2,
      enabled_triggers: ['assigned', 'overdue'],
    } as never)
    const w = await mountNotifications()
    const sw = w
      .findAll('button[role="switch"]')
      .find((b) => b.text().includes('A task is overdue'))!
    expect(sw.attributes('aria-checked')).toBe('false')
    await sw.trigger('click')
    await flushPromises()
    expect(m.putNotifyPrefs).toHaveBeenCalledWith('m1', 1, {
      enabled_triggers: ['assigned', 'overdue'],
    })
    const bar = useSnackbar()
    const action = bar.state.current?.action
    expect(action?.label).toBe('Undo')
    m.putNotifyPrefs.mockResolvedValue({ ...prefs, version: 3 } as never)
    await action!.run()
    expect(m.putNotifyPrefs).toHaveBeenLastCalledWith('m1', 2, {
      enabled_triggers: ['assigned'],
    })
  })

  it('the minutes form shows the Save bar only while dirty and saves in one PUT', async () => {
    m.putNotifyPrefs.mockResolvedValue({ ...prefs, version: 2, due_soon_minutes: 30 } as never)
    const w = await mountNotifications()
    expect(w.find('.ui-savebar').exists()).toBe(false)
    await w.get('input[name=due_soon]').setValue('30')
    expect(w.find('.ui-savebar').exists()).toBe(true)
    await w.get('.ui-savebar button:last-child').trigger('click')
    await flushPromises()
    expect(m.putNotifyPrefs).toHaveBeenCalledWith('m1', 1, {
      due_soon_minutes: 30,
      nudge_minutes: 15,
      quiet_from: null,
      quiet_until: null,
    })
    expect(w.find('.ui-savebar').exists()).toBe(false)
  })
})

describe('Settings, Status', () => {
  it('shows a state chip per subsystem and the queue counts', async () => {
    m.household.mockResolvedValue({ version: 1 } as never)
    m.status.mockResolvedValue({
      sources: [{ id: 's1', name: 'Family', kind: 'ics', state: 'ok', last_synced_at: null }],
      weather: { ok: false, fetched_at: null },
      backup: { ok: true, last_at: null },
      queue: { ready: 1, running: 0, dead: 2 },
    } as never)
    const w = mount(SettingsStatusView, { global: { stubs: { RouterLink: true } } })
    await flushPromises()
    expect(w.text()).toContain('failing')
    expect(w.get('[data-testid="queue"]').text()).toContain('2 failed')
  })
})
