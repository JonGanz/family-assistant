import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '../api/http'
import { useHouseholdStore } from '../stores/household'
import { resetSnackbar, useSnackbar } from '../ui/composables/useSnackbar'
import SettingsHouseholdView from './SettingsHouseholdView.vue'

const leaveGuards: (() => unknown)[] = []
vi.mock('vue-router', () => ({
  useRouter: () => ({ push: vi.fn() }),
  useRoute: () => ({ path: '/settings/household', query: {} }),
  onBeforeRouteLeave: (fn: () => unknown) => leaveGuards.push(fn),
}))
vi.mock('../api/client', () => ({ api: { household: vi.fn(), patchHousehold: vi.fn() } }))
import { api } from '../api/client'
const m = vi.mocked(api)

const household = {
  name: 'Example Household',
  tz: 'UTC',
  units: 'metric',
  week_start: 'monday',
  calendar_past_days: 30,
  calendar_future_days: 400,
  weather_retention_days: 7,
  private_events_as_busy: true,
  dashboard_completion_override: true,
  dashboard_pending_approvals: true,
  google_oauth_enabled: false,
  version: 3,
  location: null,
  now_list_size: 7,
  now_weights: { due: 1, flag: 0.5, weather: 0.5 },
  quiet_from: '21:00',
  quiet_until: '07:00',
}

async function mountPage() {
  m.household.mockResolvedValue(household as never)
  const w = mount(SettingsHouseholdView, { global: { stubs: { RouterLink: true } } })
  await flushPromises()
  return w
}

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
  resetSnackbar()
  leaveGuards.length = 0
})

describe('SettingsHouseholdView', () => {
  it('saves the multi-field draft through the Save bar in one PATCH', async () => {
    m.patchHousehold.mockResolvedValue(household as never)
    const w = await mountPage()
    expect(w.find('.ui-savebar').exists()).toBe(false)
    await w.find('input[name=now_list_size]').setValue('9')
    await w.find('input[name=w_flag]').setValue('2')
    await w.find('input[name=quiet_from]').setValue('22:00')
    expect(w.find('.ui-savebar').exists()).toBe(true)
    const save = w.findAll('.ui-savebar button').find((b) => b.text() === 'Save')!
    await save.trigger('click')
    await flushPromises()
    expect(m.patchHousehold).toHaveBeenCalledTimes(1)
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

  it('blocks leaving while the draft is dirty', async () => {
    const w = await mountPage()
    expect(leaveGuards).toHaveLength(1)
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false)
    expect(await leaveGuards[0]!()).toBe(true) // clean: leaves freely
    expect(confirm).not.toHaveBeenCalled()
    await w.find('input[name=name]').setValue('Other')
    expect(await leaveGuards[0]!()).toBe(false)
    expect(confirm).toHaveBeenCalled()
    confirm.mockRestore()
  })

  it('applies a switch at once and offers Undo', async () => {
    m.patchHousehold.mockResolvedValue(household as never)
    const w = await mountPage()
    const sw = w.findAll('[role=switch]').find((s) => s.text().includes('ticking off tasks'))!
    expect(sw.attributes('aria-checked')).toBe('true')
    await sw.trigger('click')
    await flushPromises()
    expect(m.patchHousehold).toHaveBeenCalledWith(
      3,
      expect.objectContaining({ dashboard_completion_override: false }),
    )
    const snack = useSnackbar().state.current
    expect(snack?.action?.label).toBe('Undo')
    // A switch is not a form: it never raises the Save bar.
    expect(w.find('.ui-savebar').exists()).toBe(false)
    await snack!.action!.run()
    expect(m.patchHousehold).toHaveBeenLastCalledWith(
      3,
      expect.objectContaining({ dashboard_completion_override: true }),
    )
  })

  it('applies a units choice at once', async () => {
    m.patchHousehold.mockResolvedValue({ ...household, units: 'imperial' } as never)
    const w = await mountPage()
    const imperial = w
      .findAll('[role=radio], [role=tab], button')
      .find((b) => b.text().startsWith('Imperial'))!
    await imperial.trigger('click')
    await flushPromises()
    expect(m.patchHousehold).toHaveBeenCalledWith(3, expect.objectContaining({ units: 'imperial' }))
  })

  it('shows "changed elsewhere" on a 409 and reloads on demand', async () => {
    m.patchHousehold.mockRejectedValue(new ApiError(409, {}))
    const w = await mountPage()
    await w.find('input[name=name]').setValue('Other')
    const save = w.findAll('.ui-savebar button').find((b) => b.text() === 'Save')!
    await save.trigger('click')
    await flushPromises()
    expect(useHouseholdStore().conflict).toBe(true)
    expect(w.text()).toContain('changed elsewhere')
    m.household.mockClear()
    const reload = w.findAll('button').find((b) => b.text() === 'Reload')!
    await reload.trigger('click')
    await flushPromises()
    expect(m.household).toHaveBeenCalledTimes(1)
    expect(w.text()).not.toContain('changed elsewhere')
    expect(w.find('.ui-savebar').exists()).toBe(false)
  })
})
