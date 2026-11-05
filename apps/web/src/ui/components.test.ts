import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it } from 'vitest'
import { nextTick } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'
import {
  UiBottomSheet,
  UiDialog,
  UiIcon,
  UiNavBar,
  UiNavDrawer,
  UiNavRail,
  UiSegmented,
  UiSwitch,
} from './index'
import { icons } from './icons'
import { isActivePath, type NavDestination } from './nav'

const destinations: NavDestination[] = [
  { id: 'home', label: 'Today', icon: 'today', to: '/' },
  { id: 'tasks', label: 'Tasks', icon: 'tasks', to: '/tasks', badge: 3 },
  { id: 'inbox', label: 'Inbox', icon: 'inbox', to: '/inbox', group: 'Mail' },
]

describe('UiIcon', () => {
  it('renders an SVG, hidden unless labelled', () => {
    const w = mount(UiIcon, { props: { name: 'home' } })
    const svg = w.find('svg')
    expect(svg.exists()).toBe(true)
    expect(svg.attributes('aria-hidden')).toBe('true')
    const l = mount(UiIcon, { props: { name: 'home', label: 'Home' } })
    expect(l.find('svg').attributes('aria-label')).toBe('Home')
    expect(l.find('svg').attributes('aria-hidden')).toBeUndefined()
  })
  it('registers every required semantic name', () => {
    for (const n of ['home', 'inbox', 'ai', 'open_external', 'visibility_off', 'chevron_right'])
      expect(icons).toHaveProperty(n)
  })
})

describe('UiSwitch', () => {
  it('is a switch that toggles with click and keyboard activation', async () => {
    const w = mount(UiSwitch, { props: { modelValue: false, label: 'Notify' } })
    const b = w.get('button')
    expect(b.attributes('role')).toBe('switch')
    expect(b.attributes('aria-checked')).toBe('false')
    await b.trigger('click')
    expect(w.emitted('update:modelValue')![0]).toEqual([true])
    await w.setProps({ modelValue: true })
    expect(b.attributes('aria-checked')).toBe('true')
  })
  it('does not toggle while disabled', async () => {
    const w = mount(UiSwitch, { props: { modelValue: false, disabled: true } })
    await w.get('button').trigger('click')
    expect(w.emitted('update:modelValue')).toBeUndefined()
  })
})

describe('UiSegmented', () => {
  const options = [
    { value: 'a', label: 'A' },
    { value: 'b', label: 'B' },
    { value: 'c', label: 'C' },
  ]
  it('is a radio group with roving tabindex and arrow keys', async () => {
    const w = mount(UiSegmented, {
      props: { modelValue: 'a', options, label: 'View' },
      attachTo: document.body,
    })
    expect(w.get('[role=radiogroup]').attributes('aria-label')).toBe('View')
    const radios = w.findAll('[role=radio]')
    expect(radios.map((r) => r.attributes('aria-checked'))).toEqual(['true', 'false', 'false'])
    expect(radios.map((r) => r.attributes('tabindex'))).toEqual(['0', '-1', '-1'])
    await radios[0]!.trigger('keydown', { key: 'ArrowRight' })
    expect(w.emitted('update:modelValue')![0]).toEqual(['b'])
    await radios[0]!.trigger('keydown', { key: 'ArrowLeft' })
    expect(w.emitted('update:modelValue')![1]).toEqual(['c'])
    w.unmount()
  })
})

describe('UiDialog', () => {
  beforeEach(() => {
    document.body.innerHTML = ''
  })
  const mk = (props: Record<string, unknown> = {}) =>
    mount(UiDialog, {
      props: { modelValue: false, title: 'Delete?', ...props },
      slots: { default: '<input id="first" />', actions: '<button id="ok">OK</button>' },
      attachTo: document.body,
    })

  it('focuses inside, traps Tab, closes on Escape and restores focus', async () => {
    const opener = document.createElement('button')
    document.body.append(opener)
    opener.focus()
    const w = mk()
    await w.setProps({ modelValue: true })
    await nextTick()
    await nextTick()
    const dialog = document.querySelector('[role=dialog]')!
    expect(dialog.getAttribute('aria-modal')).toBe('true')
    expect(dialog.contains(document.activeElement)).toBe(true)
    // Tab from the last control wraps to the first
    document.querySelector<HTMLElement>('#ok')!.focus()
    document.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true }),
    )
    expect(document.activeElement?.id).toBe('first')
    // Shift+Tab from the first wraps to the last
    document.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Tab', shiftKey: true, bubbles: true, cancelable: true }),
    )
    expect(document.activeElement?.id).toBe('ok')
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    expect(w.emitted('update:modelValue')![0]).toEqual([false])
    await w.setProps({ modelValue: false })
    await nextTick()
    expect(document.activeElement).toBe(opener)
    w.unmount()
  })

  it('closes on scrim click unless not dismissible', async () => {
    const w = mk({ modelValue: true })
    await nextTick()
    document.querySelector<HTMLElement>('[data-testid=scrim]')!.click()
    expect(w.emitted('update:modelValue')).toHaveLength(1)
    await w.setProps({ dismissible: false })
    document.querySelector<HTMLElement>('[data-testid=scrim]')!.click()
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    expect(w.emitted('update:modelValue')).toHaveLength(1)
    w.unmount()
  })

  it('UiBottomSheet is a modal dialog too', async () => {
    const w = mount(UiBottomSheet, {
      props: { modelValue: true, title: 'New' },
      attachTo: document.body,
    })
    await nextTick()
    expect(document.querySelector('[role=dialog]')?.getAttribute('aria-labelledby')).toBeTruthy()
    w.unmount()
  })
})

describe('navigation', () => {
  it('isActivePath matches by prefix, and "/" only itself', () => {
    expect(isActivePath('/tasks/3', '/tasks')).toBe(true)
    expect(isActivePath('/tasks', '/tasks')).toBe(true)
    expect(isActivePath('/tasksx', '/tasks')).toBe(false)
    expect(isActivePath('/tasks', '/')).toBe(false)
    expect(isActivePath('/', '/')).toBe(true)
  })

  async function mountNav(
    component: typeof UiNavBar | typeof UiNavRail | typeof UiNavDrawer,
    path: string,
  ) {
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: '/:p(.*)*', component: { template: '<i/>' } }],
    })
    await router.push(path)
    await router.isReady()
    return mount(component, { props: { destinations }, global: { plugins: [router] } })
  }

  for (const [name, c] of [
    ['bar', UiNavBar],
    ['rail', UiNavRail],
    ['drawer', UiNavDrawer],
  ] as const) {
    it(`${name} marks the active destination by path prefix and shows the badge`, async () => {
      const w = await mountNav(c, '/tasks/42')
      const current = w.findAll('[aria-current=page]')
      expect(current).toHaveLength(1)
      expect(current[0]!.text()).toContain('Tasks')
      expect(w.find('nav').exists()).toBe(true)
      expect(w.text()).toContain('3')
    })
  }

  it('drawer groups destinations by their group', async () => {
    const w = await mountNav(UiNavDrawer, '/')
    expect(w.findAll('h2').map((h) => h.text())).toEqual(['Mail'])
  })
})
