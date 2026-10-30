import { mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import DashboardCanvas from '../dashboard/DashboardCanvas.vue'
import { FAMILIES } from '../themes'
import type { ThemeFamily } from '../themes'
import SettingsAppearanceView from './SettingsAppearanceView.vue'
import type { DashboardData } from '../dashboard/types'

const base = FAMILIES[0]!
const mocha = base.variants.find((v) => v.mode === 'dark')!
const darkOnly: ThemeFamily = {
  id: 'darkonly',
  label: 'Dark only',
  source: 'https://example.test',
  license: 'MIT',
  variants: [mocha],
}
const twoDark: ThemeFamily = {
  id: 'twodark',
  label: 'Two dark',
  source: 'https://example.test',
  license: 'MIT',
  variants: [
    { ...mocha, id: 'd1', label: 'D1' },
    { ...mocha, id: 'd2', label: 'D2' },
  ],
}
const added = [darkOnly, twoDark]

beforeEach(() => {
  localStorage.clear()
  document.documentElement.removeAttribute('style')
  FAMILIES.push(...added)
})
afterEach(() => {
  for (const f of added) FAMILIES.splice(FAMILIES.indexOf(f), 1)
})

describe('Appearance page', () => {
  it('shows the mode control for a two-mode family and saves the choice', async () => {
    const w = mount(SettingsAppearanceView)
    expect(w.find('[data-testid="mode"]').exists()).toBe(true)
    await w.find('[data-testid="mode"]').setValue('dark')
    await w.vm.$nextTick()
    expect(JSON.parse(localStorage.getItem('fa.theme')!)).toMatchObject({ mode: 'dark' })
    expect(document.documentElement.getAttribute('data-theme-mode')).toBe('dark')
    expect(w.find('[data-testid="device-only"]').exists()).toBe(true)
  })

  it('hides the mode control for a single-variant family and says why', async () => {
    const w = mount(SettingsAppearanceView)
    await w.find('[data-family="darkonly"] button').trigger('click')
    expect(w.find('[data-testid="mode"]').exists()).toBe(false)
    expect(w.find('[data-testid="mode-note"]').text()).toContain('dark variant')
    expect(w.find('[data-testid="variant"]').exists()).toBe(false)
  })

  it('shows the variant select only with several variants in the mode', async () => {
    const w = mount(SettingsAppearanceView)
    await w.find('[data-family="twodark"] button').trigger('click')
    expect(w.find('[data-testid="mode"]').exists()).toBe(false)
    await w.find('[data-testid="variant"]').setValue('d2')
    await w.vm.$nextTick()
    expect(JSON.parse(localStorage.getItem('fa.theme')!)).toMatchObject({
      family: 'twodark',
      variant: 'd2',
    })
  })
})

describe('kiosk canvas theme', () => {
  const data = (theme: unknown): DashboardData =>
    ({
      household: { name: 'Example Household', tz: 'UTC', units: 'metric', location: null },
      members: [],
      events: [],
      advisories: [],
      routines: [],
      tasks: [],
      task_lists: [],
      weather: null,
      settings: {
        orientation: 'landscape',
        theme,
        agenda_days: 2,
        members: 'all',
        panels: [{ kind: 'clock' }],
      },
    }) as unknown as DashboardData
  const noon = new Date('2026-10-24T12:00:00Z')

  it('a dark choice renders dark at noon, on the canvas root only', async () => {
    const w = mount(DashboardCanvas, {
      props: { data: data({ family: 'catppuccin', mode: 'dark' }), now: noon },
    })
    await w.vm.$nextTick()
    const el = w.element as HTMLElement
    expect(el.getAttribute('data-theme-mode')).toBe('dark')
    expect(el.style.getPropertyValue('--color-bg')).toBe(mocha.neutrals.bg)
    expect(document.documentElement.style.getPropertyValue('--color-bg')).not.toBe(
      mocha.neutrals.bg,
    )
  })

  it('accepts the legacy string', async () => {
    const w = mount(DashboardCanvas, { props: { data: data('dark'), now: noon } })
    await w.vm.$nextTick()
    expect((w.element as HTMLElement).getAttribute('data-theme-mode')).toBe('dark')
  })

  it('burn-in shift applies on dark only', () => {
    // 10-minute slot 1 shifts by [3, 1]
    const t = new Date(600_000)
    const grid = (theme: unknown) =>
      mount(DashboardCanvas, { props: { data: data(theme), now: t, burnIn: true } }).find('.grid')
    expect(grid({ family: 'catppuccin', mode: 'dark' }).attributes('style')).toContain('3px, 1px')
    expect(grid({ family: 'catppuccin', mode: 'light' }).attributes('style')).toContain('0px, 0px')
  })
})
