import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick, ref } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'
import {
  UiBottomSheet,
  UiDialog,
  UiIcon,
  UiNavBar,
  UiNavDrawer,
  UiNavRail,
  UiButton,
  UiSelect,
  UiTextField,
  UiSegmented,
  UiSwitch,
  UiTopAppBar,
} from './index'
import type { Breakpoint } from './composables/useBreakpoint'
import { icons } from './icons'
import * as uiIndex from './index'
import saveBarSrc from './UiSaveBar.vue?raw'
import { isActivePath, navControlKey, type NavDestination } from './nav'

const bpRef = ref<Breakpoint>('compact')
vi.mock('./composables/useBreakpoint', async (orig) => ({
  ...(await orig<typeof import('./composables/useBreakpoint')>()),
  useBreakpoint: () => bpRef,
}))

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

describe('top app bar menu button', () => {
  const shell = { openNav: vi.fn() }
  const mountBar = (width: 'compact' | 'medium' | 'expanded', inShell: boolean, back = false) => {
    bpRef.value = width
    return mount(UiTopAppBar, {
      props: { title: 'Page', back },
      global: { provide: inShell ? { [navControlKey as symbol]: shell } : {} },
    })
  }
  it('appears only on compact inside the shell, and opens the nav', async () => {
    const w = mountBar('compact', true)
    await w.find('[aria-label="Open navigation"]').trigger('click')
    expect(shell.openNav).toHaveBeenCalled()
    expect(mountBar('medium', true).find('[aria-label="Open navigation"]').exists()).toBe(false)
    expect(mountBar('expanded', true).find('[aria-label="Open navigation"]').exists()).toBe(false)
    expect(mountBar('compact', false).find('[aria-label="Open navigation"]').exists()).toBe(false)
  })
  it('keeps the back arrow on nested pages', () => {
    const w = mountBar('compact', true, true)
    expect(w.find('[aria-label="Open navigation"]').exists()).toBe(false)
    expect(w.find('[aria-label="Back"]').exists()).toBe(true)
  })
})

describe('save bar', () => {
  it('is sticky at the bottom on every window class (no media-query override)', () => {
    const src = saveBarSrc
    expect(src).toMatch(/position: sticky;\s*bottom: 0;/)
    expect(src).not.toMatch(/@media/)
    expect(src).toContain('--ui-surface-container-high')
    expect(src).toContain('--ui-elev-2')
  })
})

describe('ui gaps (T-19)', () => {
  it('registers the arrow and menu_open icons and re-exports navControlKey', () => {
    for (const n of ['arrow_upward', 'arrow_downward', 'menu_open']) expect(icons).toHaveProperty(n)
    expect(uiIndex.navControlKey).toBe(navControlKey)
  })
  it('UiTextField forwards name and id to the input and class to the wrapper', () => {
    const w = mount(UiTextField, {
      props: { modelValue: '', label: 'Title' },
      attrs: { name: 'title', id: 'x-title', class: 'wide', placeholder: 'p' },
    })
    expect(w.classes()).toContain('wide')
    const input = w.get('input')
    expect(input.attributes('name')).toBe('title')
    expect(input.attributes('id')).toBe('x-title')
    expect(w.get('label').attributes('for')).toBe('x-title')
    expect(input.classes()).not.toContain('wide')
  })
  it('UiSelect forwards name and id to the select and class to the wrapper', () => {
    const w = mount(UiSelect, {
      props: { modelValue: 'a', label: 'Pick', options: [{ value: 'a', label: 'A' }] },
      attrs: { name: 'pick', id: 'x-pick', class: 'wide' },
    })
    expect(w.classes()).toContain('wide')
    expect(w.get('select').attributes('name')).toBe('pick')
    expect(w.get('select').attributes('id')).toBe('x-pick')
  })
  it('UiButton renders a link for to and an anchor for href', () => {
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: '/', component: { template: '<i/>' } },
        { path: '/t', component: { template: '<i/>' } },
      ],
    })
    const a = mount(UiButton, {
      props: { to: '/t' },
      slots: { default: 'Go' },
      global: { plugins: [router] },
    })
    expect(a.element.tagName).toBe('A')
    expect(a.attributes('href')).toBe('/t')
    const b = mount(UiButton, { props: { href: 'https://example.com' }, slots: { default: 'Out' } })
    expect(b.element.tagName).toBe('A')
    expect(b.attributes('href')).toBe('https://example.com')
    expect(mount(UiButton).element.tagName).toBe('BUTTON')
  })
})
