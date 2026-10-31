import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import Screensaver from './Screensaver.vue'

describe('Screensaver', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.stubGlobal('requestAnimationFrame', () => 1)
    vi.stubGlobal('cancelAnimationFrame', () => {})
  })
  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllGlobals()
  })

  const shown = (w: ReturnType<typeof mount>) => w.find('[data-testid="screensaver"]').exists()

  it('starts after the idle time and a tap wakes it', async () => {
    const w = mount(Screensaver, { props: { minutes: 15 } })
    vi.advanceTimersByTime(15 * 60_000 - 1000)
    await w.vm.$nextTick()
    expect(shown(w)).toBe(false)
    vi.advanceTimersByTime(1000)
    await w.vm.$nextTick()
    expect(shown(w)).toBe(true)
    await w.find('[data-testid="screensaver"]').trigger('click')
    expect(shown(w)).toBe(false)
    w.unmount()
  })

  it('interaction restarts the countdown', async () => {
    const w = mount(Screensaver, { props: { minutes: 15 } })
    vi.advanceTimersByTime(14 * 60_000)
    window.dispatchEvent(new Event('pointerdown'))
    vi.advanceTimersByTime(14 * 60_000)
    await w.vm.$nextTick()
    expect(shown(w)).toBe(false)
    vi.advanceTimersByTime(60_000)
    await w.vm.$nextTick()
    expect(shown(w)).toBe(true)
    w.unmount()
  })

  it('0 minutes turns it off', async () => {
    const w = mount(Screensaver, { props: { minutes: 0 } })
    vi.advanceTimersByTime(10 * 60 * 60_000)
    await w.vm.$nextTick()
    expect(shown(w)).toBe(false)
    w.unmount()
  })
})
