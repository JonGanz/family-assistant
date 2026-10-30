import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import DashboardCanvas from './DashboardCanvas.vue'
import { addPanel, applyPreset, isValid, movePanel, removePanel, toggleMember } from './editor'
import { resolveMode } from './theme'
import LayoutEditor from './LayoutEditor.vue'
import PanelHost from './panels/PanelHost.vue'
import { dailyOf, hourlyOf } from './weatherModel'
import type { DashboardData, DashboardSettings } from './types'

const settings = (over: Partial<DashboardSettings> = {}): DashboardSettings =>
  ({
    orientation: 'landscape',
    theme: { family: 'catppuccin', mode: 'light' },
    agenda_days: 2,
    members: 'all',
    panels: [{ kind: 'agenda' }],
    ...over,
  }) as DashboardSettings

const data = (over: Partial<DashboardData> = {}): DashboardData =>
  ({
    household: { name: 'Example Household', tz: 'UTC', units: 'metric', location: null },
    members: [
      {
        id: 'm1',
        display_name: 'Alex',
        role: 'adult',
        avatar: null,
        avatar_url: null,
        color: '#3366cc',
      },
    ],
    events: [
      {
        id: 'e1',
        title: 'Dentist',
        start_at: '2026-10-24T14:00:00Z',
        end_at: '2026-10-24T15:00:00Z',
        all_day: false,
        members: ['m1'],
        color: '#3366cc',
        visibility: 'full',
      },
      {
        id: 'e2',
        title: 'SECRET TITLE',
        start_at: '2026-10-24T16:00:00Z',
        end_at: '2026-10-24T17:00:00Z',
        all_day: false,
        members: ['m1'],
        color: '#3366cc',
        visibility: 'busy',
      },
    ],
    advisories: [
      {
        id: 'a1',
        text: 'Frost tonight',
        severity: 'warning',
        valid_from: '',
        valid_until: '',
        source: 'rule',
      },
    ],
    weather: null,
    settings: settings(),
    ...over,
  }) as DashboardData

const NOW = new Date('2026-10-24T10:00:00Z')

describe('panels', () => {
  it('unknown panel kinds render nothing', () => {
    const w = mount(PanelHost, {
      props: { data: data(), settings: settings(), panel: { kind: 'hologram' }, now: NOW },
    })
    expect(w.html()).toBe('<!--v-if-->')
  })

  it('canvas drops unknown panels but renders known ones', () => {
    const s = settings({ panels: [{ kind: 'hologram' }, { kind: 'clock' }] as never })
    const w = mount(DashboardCanvas, { props: { data: data({ settings: s }), now: NOW } })
    expect(w.findAll('.cell')).toHaveLength(1)
    expect(w.find('[data-testid="clock"]').exists()).toBe(true)
  })

  it('a busy event renders without its title', () => {
    const w = mount(PanelHost, {
      props: { data: data(), settings: settings(), panel: { kind: 'agenda' }, now: NOW },
    })
    expect(w.text()).toContain('Dentist')
    expect(w.text()).toContain('Busy')
    expect(w.text()).not.toContain('SECRET TITLE')
  })

  it('the member filter hides other members events but keeps household-wide ones', () => {
    const d = data()
    d.events.push({ ...d.events[0]!, id: 'e3', title: 'Other', members: ['m2'] })
    d.events.push({ ...d.events[0]!, id: 'e4', title: 'Whole house', members: [] })
    const w = mount(PanelHost, {
      props: {
        data: d,
        settings: settings({ members: ['m1'] }),
        panel: { kind: 'agenda' },
        now: NOW,
      },
    })
    expect(w.text()).not.toContain('Other')
    expect(w.text()).toContain('Whole house')
  })

  it('weather panel shows advisories and an upcoming thunderstorm', () => {
    const daily = {
      time: ['2026-10-29', '2026-10-30', '2026-10-31'],
      weather_code: [1, 2, 95],
    }
    const d = data({
      weather: { current: { weather_code: 1, temperature_2m: 10 }, daily },
    } as unknown as Partial<DashboardData>)
    const w = mount(PanelHost, {
      props: { data: d, settings: settings(), panel: { kind: 'weather' }, now: NOW },
    })
    expect(w.find('[data-severity="warning"]').text()).toContain('Frost tonight')
    expect(w.text()).toContain('Thunderstorms forecast')
    expect(w.find('button').exists()).toBe(false)
  })

  it('week table has a row per day and a column per person, plus Everyone', () => {
    const w = mount(PanelHost, {
      props: { data: data(), settings: settings(), panel: { kind: 'week_table' }, now: NOW },
    })
    expect(w.findAll('tbody tr')).toHaveLength(7)
    const heads = w.findAll('thead th').map((h) => h.text())
    expect(heads).toContain('Everyone')
    expect(heads.length).toBe(data().members.length + 2)
  })

  it('weather shows the day forecast before noon only', () => {
    const wx = {
      current: { weather_code: 1, temperature_2m: 10 },
      daily: {
        time: ['2026-10-29'],
        weather_code: [1],
        temperature_2m_max: [18],
        temperature_2m_min: [9],
      },
    }
    const d = data({ weather: wx } as unknown as Partial<DashboardData>)
    const at = (iso: string) =>
      mount(PanelHost, {
        props: { data: d, settings: settings(), panel: { kind: 'weather' }, now: new Date(iso) },
      })
    const has = (iso: string) => at(iso).find('[data-testid="today-forecast"]').exists()
    expect(has('2026-10-29T09:00:00Z')).toBe(true)
    expect(has('2026-10-29T13:00:00Z')).toBe(false)
  })

  it('members strip shows a count badge only for adults with approvals waiting', () => {
    const mk = (pa: { member_id: string; count: number }[]) =>
      mount(PanelHost, {
        props: {
          data: data({ pending_approvals: pa } as Partial<DashboardData>),
          settings: settings(),
          panel: { kind: 'members' },
          now: NOW,
        },
      })
    expect(
      mk([{ member_id: 'm1', count: 3 }])
        .find('[data-testid="approvals-m1"]')
        .text(),
    ).toBe('3')
    expect(mk([]).find('.approvals').exists()).toBe(false)
    expect(
      mk([{ member_id: 'm1', count: 0 }])
        .find('.approvals')
        .exists(),
    ).toBe(false)
  })

  it('weather columns convert to points', () => {
    const w = {
      fetched_at: '',
      current: {},
      hourly: {
        time: ['2026-10-24T10:00', '2026-10-24T11:00'],
        temperature_2m: [10, 12],
        weather_code: [1, 3],
      },
      daily: {
        time: ['2026-10-24'],
        temperature_2m_max: [14],
        temperature_2m_min: [6],
        weather_code: [61],
      },
    }
    expect(hourlyOf(w as never)[1]).toMatchObject({ label: '11:00', temp: 12, code: 3 })
    expect(dailyOf(w as never)[0]).toMatchObject({ date: '2026-10-24', max: 14, min: 6, code: 61 })
  })
})

describe('theme', () => {
  const up = '2026-10-24T06:00:00Z'
  const down = '2026-10-24T18:00:00Z'
  it('auto is dark between sunset and sunrise', () => {
    expect(resolveMode('auto', new Date('2026-10-24T12:00:00Z'), up, down)).toBe('light')
    expect(resolveMode('auto', new Date('2026-10-24T19:00:00Z'), up, down)).toBe('dark')
    expect(resolveMode('auto', new Date('2026-10-24T04:00:00Z'), up, down)).toBe('dark')
    expect(resolveMode('dark', new Date('2026-10-24T12:00:00Z'), up, down)).toBe('dark')
  })
})

describe('layout editor', () => {
  it('pure helpers keep settings valid', () => {
    let s = settings({ panels: [{ kind: 'clock' }] as never })
    s = addPanel(s, 'weather')
    s = addPanel(s, 'nonsense')
    expect(s.panels.map((p) => p.kind)).toEqual(['clock', 'weather'])
    s = movePanel(s, 1, -1)
    expect(s.panels.map((p) => p.kind)).toEqual(['weather', 'clock'])
    s = removePanel(s, 0)
    expect(s.panels).toHaveLength(1)
    s = applyPreset(s, 'Calendar wall')
    expect(s.agenda_days).toBe(7)
    expect(s.orientation).toBe('landscape')
    expect(isValid(s)).toBe(true)
    expect(isValid({ ...s, agenda_days: 15 })).toBe(false)
    expect(toggleMember(settings(), 'a', ['a', 'b']).members).toEqual(['b'])
    expect(toggleMember(settings({ members: ['b'] }), 'a', ['a', 'b']).members).toBe('all')
  })

  it('emits valid DashboardSettings from user input', async () => {
    const w = mount(LayoutEditor, {
      props: {
        modelValue: settings({ panels: [{ kind: 'clock' }] as never }),
        members: data().members,
      },
    })
    await w.find('[data-testid="orientation"]').setValue('portrait')
    await w.find('[data-testid="theme"]').setValue('dark')
    await w.find('[data-testid="agenda-days"]').setValue('99')
    await w.find('[data-testid="add-kind"]').setValue('week')
    await w.find('[data-testid="add-panel"]').trigger('click')
    await w.find('[data-testid="preset"]').setValue('Minimal')
    const emits = w.emitted('update:modelValue') as DashboardSettings[][]
    expect(emits.length).toBeGreaterThanOrEqual(5)
    for (const [s] of emits) expect(isValid(s!)).toBe(true)
    expect(emits[0]![0]!.orientation).toBe('portrait')
    expect(emits[2]![0]!.agenda_days).toBe(14)
    await w.find('form').trigger('submit')
    expect(w.emitted('save')).toHaveLength(1)
  })
})
