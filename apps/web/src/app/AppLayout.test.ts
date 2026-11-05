import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'
import AppLayout from '../layouts/AppLayout.vue'
import { useInboxStore } from '../stores/inbox'
import { useSessionStore } from '../stores/session'
import type { Breakpoint } from '../ui'

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
    slots: { default: '<p>page</p>' },
    global: { plugins: [router] },
    attachTo: document.body,
  })
}

beforeEach(() => {
  document.body.innerHTML = ''
})

describe('AppLayout', () => {
  it('shows the bottom bar on compact, with More and the inbox badge', async () => {
    const w = await mountAt('compact')
    expect(w.find('.ui-navbar').exists()).toBe(true)
    expect(w.find('.ui-rail').exists()).toBe(false)
    expect(w.find('.ui-drawer').exists()).toBe(false)
    const labels = w.findAll('.ui-navbar__label').map((l) => l.text())
    expect(labels).toEqual(['Today', 'Calendar', 'Tasks', 'Inbox', 'More'])
    expect(w.find('.ui-navbar [aria-label="2 waiting"], .ui-navbar .ui-badge').exists()).toBe(true)
    expect(w.find('.ui-navbar__item.is-active').text()).toContain('Tasks')
    w.unmount()
  })

  it('opens the More sheet from the bar', async () => {
    const w = await mountAt('compact')
    const more = w.findAll('.ui-navbar__item').find((i) => i.text().includes('More'))!
    await more.trigger('click')
    const sheet = document.body.textContent ?? ''
    for (const t of ['Routines', 'Mail', 'Bills', 'Settings', 'Log out']) expect(sheet).toContain(t)
    w.unmount()
  })

  it('shows the rail on medium', async () => {
    const w = await mountAt('medium')
    expect(w.find('.ui-rail').exists()).toBe(true)
    expect(w.find('.ui-navbar').exists()).toBe(false)
    expect(w.find('.ui-drawer').exists()).toBe(false)
    w.unmount()
  })

  it('shows the drawer on expanded', async () => {
    const w = await mountAt('expanded')
    expect(w.find('.ui-drawer').exists()).toBe(true)
    expect(w.find('.ui-navbar').exists()).toBe(false)
    expect(w.find('.ui-rail').exists()).toBe(false)
    w.unmount()
  })

  it('hides adult destinations from a child', async () => {
    const w = await mountAt('compact', 'child')
    const labels = w.findAll('.ui-navbar__label').map((l) => l.text())
    expect(labels).toEqual(['Today', 'Calendar', 'Tasks', 'More'])
    w.unmount()
  })
})
