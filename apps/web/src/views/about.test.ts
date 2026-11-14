import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { describe, expect, it, vi } from 'vitest'
import { buildsDiffer, builtAtText } from './aboutModel'

describe('about model', () => {
  it('flags differing commits only when both are known', () => {
    expect(buildsDiffer('abc', 'abc')).toBe(false)
    expect(buildsDiffer('abc', 'def')).toBe(true)
    expect(buildsDiffer('abc-dirty', 'abc')).toBe(true)
    expect(buildsDiffer(null, 'abc')).toBe(false)
    expect(buildsDiffer('abc', '')).toBe(false)
  })
  it('formats a build time in the given zone', () => {
    expect(builtAtText('2026-10-29T12:00:00Z', 'UTC')).toMatch(/2026/)
    expect(builtAtText(null, 'UTC')).toBe('unknown')
    expect(builtAtText('not a date', 'UTC')).toBe('not a date')
  })
})

describe('About release notes', () => {
  it('renders the section, marks the running version and toggles older entries', async () => {
    setActivePinia(createPinia())
    const mod = await import('./SettingsAboutView.vue')
    const { api } = await import('../api/client')
    vi.spyOn(api, 'about').mockResolvedValue({
      version: '0.1.31',
      commit: null,
      built_at: null,
    } as never)
    const w = mount(mod.default, {
      global: { stubs: { SettingsPage: { template: '<div><slot /></div>' } } },
    })
    await flushPromises()
    expect(w.find('[data-testid="release-notes"]').exists()).toBe(true)
    expect(w.find('[data-testid="notes-0.1.31"]').text()).toContain("You're on this version")
    expect(w.findAll('.notes__entry')).toHaveLength(5)
    await w.find('[data-testid="notes-toggle"]').trigger('click')
    expect(w.findAll('.notes__entry').length).toBeGreaterThan(30)
  })
})
