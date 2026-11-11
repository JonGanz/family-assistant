import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'

vi.mock('./sound', () => ({ unlockAudio: vi.fn(), startAlarm: vi.fn(), stopAlarm: vi.fn() }))
import { startAlarm, stopAlarm, unlockAudio } from './sound'
import AddTimerModal from './AddTimerModal.vue'
import TimersView from './TimersView.vue'
import { resetTimers } from './useTimers'

describe('timers UI', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(5_000_000)
    localStorage.clear()
    vi.clearAllMocks()
    resetTimers()
  })
  afterEach(() => {
    resetTimers()
    vi.useRealTimers()
  })

  it('presets set the duration and Save emits it with the name', async () => {
    const w = mount(AddTimerModal)
    await w.find('[data-testid="timer-preset-15"]').trigger('click')
    expect((w.find('[data-testid="timer-minutes"]').element as HTMLInputElement).value).toBe('15')
    await w.find('[data-testid="timer-name"]').setValue('Pasta')
    await w.find('[data-testid="timer-form"]').trigger('submit')
    expect(w.emitted('save')![0]).toEqual(['Pasta', 900_000])
  })

  it('does not save a zero duration', async () => {
    const w = mount(AddTimerModal)
    await w.find('[data-testid="timer-minutes"]').setValue(0)
    await w.find('[data-testid="timer-form"]').trigger('submit')
    expect(w.emitted('save')).toBeUndefined()
  })

  async function addOne(w: ReturnType<typeof mount>) {
    await w.find('[data-testid="timer-add"]').trigger('click')
    await w.find('[data-testid="timer-preset-5"]').trigger('click')
    await w.find('[data-testid="timer-form"]').trigger('submit')
  }

  it('adds a running card, pauses, adds a minute, removes', async () => {
    const w = mount(TimersView, { attachTo: document.body })
    await addOne(w)
    expect(unlockAudio).toHaveBeenCalled()
    expect(w.find('[data-testid="timer-time"]').text()).toBe('05:00')
    await w.find('[data-testid="timer-add-time"]').trigger('click')
    expect(w.find('[data-testid="timer-time"]').text()).toBe('06:00')
    await w.find('[data-testid="timer-pause"]').trigger('click')
    expect(w.find('[data-testid="timer-card"]').attributes('data-state')).toBe('paused')
    expect(localStorage.getItem('fa.dashboard.timers')).toContain('paused')
    await w.find('[data-testid="timer-remove"]').trigger('click')
    expect(w.findAll('[data-testid="timer-card"]')).toHaveLength(0)
    w.unmount()
  })

  it('rings at zero, starts the alarm, and Dismiss stops it', async () => {
    const w = mount(TimersView)
    await addOne(w)
    vi.advanceTimersByTime(300_500)
    await w.vm.$nextTick()
    expect(w.find('[data-testid="timer-card"]').attributes('data-state')).toBe('ringing')
    expect(startAlarm).toHaveBeenCalled()
    await w.find('[data-testid="timer-dismiss"]').trigger('click')
    expect(stopAlarm).toHaveBeenCalled()
    expect(w.find('[data-testid="timer-card"]').attributes('data-state')).toBe('idle')
    w.unmount()
  })
})
