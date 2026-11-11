import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent } from 'vue'
import undoToastSrc from '../components/tasks/UndoToast.vue?raw'

vi.mock('../api/client', () => ({ api: { elevate: vi.fn(), dashboard: vi.fn() } }))
vi.mock('../kiosk/deviceToken', () => ({ getDeviceToken: () => 'tok' }))
vi.mock('../kiosk/keepAlive', () => ({ startKeepAlive: () => () => {} }))
vi.mock('../kiosk/watchdog', () => ({
  startWatchdog: () => ({ heartbeat() {}, stop() {} }),
}))
vi.mock('../lib/appUpdate', () => ({ watchForUpdates: () => ({ stop() {} }) }))
import { useDashboardStore } from '../stores/dashboard'
import DashboardView from './DashboardView.vue'

const Canvas = defineComponent({
  template: '<div><div class="navend"><slot name="nav-end" /></div></div>',
})

function setup(connection: string, elevatedUntil: number | null = null) {
  setActivePinia(createPinia())
  const store = useDashboardStore()
  vi.spyOn(store, 'start').mockResolvedValue(undefined as never)
  store.snapshot = { household: { tz: 'UTC' }, settings: {}, members: [] } as never
  store.connection = connection as never
  store.elevatedUntil = elevatedUntil
  const w = mount(DashboardView, {
    attachTo: document.body,
    global: {
      stubs: { DashboardCanvas: Canvas, TaskFlow: true, Screensaver: true },
    },
  })
  return { store, w }
}

describe('DashboardView menu', () => {
  beforeEach(() => {
    document.body.innerHTML = ''
  })

  it('shows no status when connected, and the status otherwise', () => {
    const ok = setup('open')
    expect(ok.w.find('[data-testid=connection]').exists()).toBe(false)
    ok.w.unmount()
    const bad = setup('reconnecting')
    const el = bad.w.find('[data-testid=connection]')
    expect(el.attributes('data-status')).toBe('reconnecting')
    bad.w.unmount()
  })

  it('the lock button is last (far right): connection, fullscreen, lock', () => {
    const { w } = setup('closed')
    const ids = w.findAll('.navend [data-testid]').map((e) => e.attributes('data-testid'))
    expect(ids.at(-1)).toBe('unlock')
    expect(ids.indexOf('connection')).toBeLessThan(ids.indexOf('unlock'))
    w.unmount()
  })

  it('opens an anchored popover when locked; closes on Escape and outside click', async () => {
    const { w } = setup('open')
    await w.find('[data-testid=unlock]').trigger('click')
    const pop = w.find('[data-testid=elevation]')
    expect(pop.exists()).toBe(true)
    expect(pop.classes()).toContain('popover')
    expect(pop.attributes('style')).toMatch(/top:/)
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    await flushPromises()
    expect(w.find('[data-testid=elevation]').exists()).toBe(false)
    await w.find('[data-testid=unlock]').trigger('click')
    document.body.dispatchEvent(new Event('pointerdown', { bubbles: true }))
    await flushPromises()
    expect(w.find('[data-testid=elevation]').exists()).toBe(false)
    w.unmount()
  })

  it('pressing lock while elevated exits elevation at once', async () => {
    const { w, store } = setup('open', Date.now() + 60_000)
    await flushPromises()
    expect(store.snapshot?.elevated).toBe(true)
    await w.find('[data-testid=unlock]').trigger('click')
    await flushPromises()
    expect(store.elevatedUntil).toBeNull()
    expect(store.snapshot?.elevated).toBe(false)
    expect(w.find('[data-testid=elevation]').exists()).toBe(false)
    w.unmount()
  })
})

describe('undo toast width', () => {
  it('has a real minimum width instead of shrinking to the container', () => {
    expect(undoToastSrc).toMatch(/width:\s*min\(36rem,\s*calc\(100vw/)
  })
})
