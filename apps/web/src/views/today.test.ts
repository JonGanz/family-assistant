import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useMembersStore } from '../stores/members'
import { resetSnackbar, useSnackbar } from '../ui/composables/useSnackbar'
import AdvisoryList from './parts/AdvisoryList.vue'
import RoutinesView from './RoutinesView.vue'

vi.mock('vue-router', () => ({ useRouter: () => ({ push: vi.fn() }), useRoute: () => ({}) }))
vi.mock('../api/client', () => ({
  api: { routinesToday: vi.fn(), tickRoutineStep: vi.fn(), members: vi.fn() },
}))
import { api } from '../api/client'
const m = vi.mocked(api)

const run = (done: boolean) => ({
  id: 'run1',
  routine_name: 'Morning routine',
  member: 'm1',
  done: done ? 1 : 0,
  total: 2,
  steps: [
    { step: 's1', text: 'Brush teeth', done_at: done ? '2026-11-05T12:00:00Z' : null },
    { step: 's2', text: 'Pack bag', done_at: null },
  ],
})

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
  resetSnackbar()
})

describe('RoutinesView', () => {
  it('ticks a step at once and offers Undo, which ticks it back', async () => {
    m.members.mockResolvedValue([])
    m.routinesToday.mockResolvedValue([run(false)] as never)
    m.tickRoutineStep.mockResolvedValueOnce(run(true) as never)
    useMembersStore().items = [{ id: 'm1', display_name: 'Riley Example', color: 'blue' }] as never
    const w = mount(RoutinesView, { global: { stubs: { RouterLink: true } } })
    await flushPromises()
    expect(w.find('[data-testid="run-Morning routine"]').exists()).toBe(true)
    expect(w.get('[role=progressbar]').attributes('aria-valuenow')).toBe('0')

    await w.findAll('input[type=checkbox]')[0]!.setValue(true)
    await flushPromises()
    expect(m.tickRoutineStep).toHaveBeenCalledWith('run1', 's1', true)
    const snack = useSnackbar().state.current
    expect(snack?.action?.label).toBe('Undo')

    m.tickRoutineStep.mockResolvedValueOnce(run(false) as never)
    await snack!.action!.run()
    expect(m.tickRoutineStep).toHaveBeenLastCalledWith('run1', 's1', false)
  })

  it('shows an empty state when there are no routines', async () => {
    m.members.mockResolvedValue([])
    m.routinesToday.mockResolvedValue([])
    const w = mount(RoutinesView, { global: { stubs: { RouterLink: true } } })
    await flushPromises()
    expect(w.text()).toContain('No routines today')
  })
})

describe('AdvisoryList', () => {
  it('marks each advisory by severity', () => {
    const w = mount(AdvisoryList, {
      props: {
        advisories: [
          { id: 'a', severity: 'severe', text: 'Storm' },
          { id: 'b', severity: 'info', text: 'Breezy' },
        ] as never,
      },
    })
    const cards = w.findAll('.advisory')
    expect(cards[0]!.classes()).toContain('severe')
    expect(cards[1]!.text()).toContain('Breezy')
  })
})
