import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { h, nextTick, ref } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'
import AppLayout from '../layouts/AppLayout.vue'
import { useInboxStore } from '../stores/inbox'
import { useSessionStore } from '../stores/session'
import { UiTopAppBar, type Breakpoint } from '../ui'

const bp = ref<Breakpoint>('compact')
vi.mock('../ui/composables/useBreakpoint', async (orig) => ({
  ...(await orig<typeof import('../ui/composables/useBreakpoint')>()),
  useBreakpoint: () => bp,
}))
vi.mock('../api/client', () => ({
  api: { inbox: vi.fn(), inboxCounts: vi.fn(), logout: vi.fn() },
}))

async function mountAt(width: Breakpoint, role: 'adult' | 'child' = 'adult') {
  bp.value = width
  setActivePinia(createPinia())
  const session = useSessionStore()
  session.me = {
    actor: { kind: 'user', member: { id: 'a', role } },
    household: { tz: 'UTC' },
  } as never
  const inbox = useInboxStore()
  inbox.load = vi.fn()
  inbox.counts = { event: 0, bill: 0, task: 2, contact: 0, total: 2 }
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/:p(.*)*', component: { template: '<i />' } }],
  })
  await router.push('/tasks')
  await router.isReady()
  return mount(AppLayout, {
    slots: { default: () => h(UiTopAppBar, { title: 'Page' }) },
    global: { plugins: [router] },
    attachTo: document.body,
  })
}

beforeEach(() => {
  localStorage.removeItem('fa.nav.collapsed')
  document.body.innerHTML = ''
})

describe('AppLayout', () => {
  it('has no bottom bar on compact; the top bar menu opens a modal drawer', async () => {
    const w = await mountAt('compact')
    expect(w.find('.ui-navbar').exists()).toBe(false)
    expect(w.find('.ui-rail').exists()).toBe(false)
    expect(document.querySelector('.ui-drawer-modal')).toBeNull()
    await w.find('[aria-label="Open navigation"]').trigger('click')
    const text = document.body.textContent ?? ''
    for (const t of ['Today', 'Inbox', 'Routines', 'Mail', 'Bills', 'Settings', 'Log out']) {
      expect(text).toContain(t)
    }
    expect(document.querySelector('[role=dialog] nav[aria-label=Primary]')).not.toBeNull()
    w.unmount()
  })

  it('closes the compact drawer on Esc, on scrim click and on navigation', async () => {
    const w = await mountAt('compact')
    const open = async () => {
      await w.find('[aria-label="Open navigation"]').trigger('click')
      await nextTick()
    }
    await open()
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    await nextTick()
    expect(document.querySelector('.ui-drawer-modal')).toBeNull()
    await open()
    ;(document.querySelector('[data-testid=scrim]') as HTMLElement).click()
    await nextTick()
    expect(document.querySelector('.ui-drawer-modal')).toBeNull()
    await open()
    const router = w.vm.$router
    await router.push('/calendar')
    await nextTick()
    expect(document.querySelector('.ui-drawer-modal')).toBeNull()
    w.unmount()
  })

  it('hides adult destinations from a child', async () => {
    const w = await mountAt('compact', 'child')
    await w.find('[aria-label="Open navigation"]').trigger('click')
    const text = document.body.textContent ?? ''
    expect(text).toContain('Tasks')
    for (const t of ['Inbox', 'Mail', 'Bills']) expect(text).not.toContain(t)
    w.unmount()
  })

  it('shows the rail on medium, with no toggle', async () => {
    const w = await mountAt('medium')
    expect(w.find('.ui-rail').exists()).toBe(true)
    expect(w.find('.ui-drawer').exists()).toBe(false)
    expect(w.find('[aria-label="Expand navigation"]').exists()).toBe(false)
    w.unmount()
  })

  it('shows the drawer on expanded, and collapses to the rail and back', async () => {
    const w = await mountAt('expanded')
    expect(w.find('.ui-drawer').exists()).toBe(true)
    expect(w.find('nav[aria-label=Primary]').exists()).toBe(true)
    await w.find('[aria-label="Collapse navigation"]').trigger('click')
    expect(w.find('.ui-drawer').exists()).toBe(false)
    expect(w.find('.ui-rail').exists()).toBe(true)
    expect(localStorage.getItem('fa.nav.collapsed')).toBe('1')
    await w.find('[aria-label="Expand navigation"]').trigger('click')
    expect(w.find('.ui-drawer').exists()).toBe(true)
    expect(localStorage.getItem('fa.nav.collapsed')).toBeNull()
    w.unmount()
  })

  it('starts collapsed when the device preference says so', async () => {
    localStorage.setItem('fa.nav.collapsed', '1')
    const w = await mountAt('expanded')
    expect(w.find('.ui-rail').exists()).toBe(true)
    expect(w.find('.ui-drawer').exists()).toBe(false)
    w.unmount()
  })
})
