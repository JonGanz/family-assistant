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

  it('week table has a row per day and a column per person', () => {
    const w = mount(PanelHost, {
      props: { data: data(), settings: settings(), panel: { kind: 'week_table' }, now: NOW },
    })
    expect(w.findAll('tbody tr')).toHaveLength(7)
    const heads = w.findAll('thead th').map((h) => h.text())
    expect(heads).not.toContain('Everyone')
    expect(heads.length).toBe(data().members.length + 1)
  })

  it('week table starts at the given day and can hide ended events and completed tasks', () => {
    const ev = (id: string, h: number) => ({
      id,
      title: `E${id}`,
      location: null,
      start_at: `2026-10-26T${String(h).padStart(2, '0')}:00:00Z`,
      end_at: `2026-10-26T${String(h + 1).padStart(2, '0')}:00:00Z`,
      all_day: false,
      start_date: null,
      end_date: null,
      members: [data().members[0]!.id],
      visibility: 'full',
    })
    const task = (id: string, status: string) => ({
      id,
      title: `T${id}`,
      due_at: '2026-10-26T12:00:00Z',
      status,
      assignees: [data().members[0]!.id],
    })
    const d = data({
      events: [ev('past', 1), ev('later', 20)],
      tasks: [task('t1', 'done'), task('t2', 'open')],
    } as unknown as Partial<DashboardData>)
    const mk = (hidePast: boolean) =>
      mount(PanelHost, {
        props: {
          data: d,
          settings: settings(),
          panel: { kind: 'week_table', start: '2026-10-26', hidePast },
          now: new Date('2026-10-26T10:00:00Z'),
        },
      })
    const shown = mk(false)
    expect(shown.find('tbody tr').attributes('data-day')).toBe('2026-10-26')
    const hidden = mk(true)
    expect(hidden.text()).not.toContain('Epast')
    expect(hidden.text()).toContain('Elater')
    expect(hidden.text()).not.toContain('Tt1')
    expect(hidden.text()).toContain('Tt2')
  })

  it('home agenda hides the location until the event is opened in a modal', async () => {
    const d = data()
    d.events[0]!.location = 'Main St Clinic'
    d.events[0]!.description = 'Bring the forms'
    const w = mount(DashboardCanvas, {
      props: { data: d, settings: settings(), now: NOW },
    })
    expect(w.text()).not.toContain('Main St Clinic')
    expect(w.find('[data-testid="detail-modal"]').exists()).toBe(false)
    await w.find('button.event').trigger('click')
    const modal = w.find('[data-testid="detail-modal"]')
    expect(modal.text()).toContain('Main St Clinic')
    expect(modal.text()).toContain('Bring the forms')
    await modal.find('button.close').trigger('click')
    expect(w.find('[data-testid="detail-modal"]').exists()).toBe(false)
  })

  it('clicking an event in the week view opens its details', async () => {
    const d = data()
    d.events[0]!.location = 'Main St Clinic'
    const w = mount(DashboardCanvas, { props: { data: d, settings: settings(), now: NOW } })
    await w.find('[data-view="week"]').trigger('click')
    const ev = w.findAll('li.event').find((e) => e.text().includes('Dentist'))!
    await ev.trigger('click')
    expect(w.find('[data-testid="detail-modal"]').text()).toContain('Main St Clinic')
  })

  it('week table shows an event in each member column with that member colour', () => {
    const d = data()
    d.members.push({ ...d.members[0]!, id: 'm2', display_name: 'Sam', color: 'red' })
    d.events[0]!.members = ['m1', 'm2']
    const w = mount(PanelHost, {
      props: {
        data: d,
        settings: settings(),
        panel: { kind: 'week_table', start: '2026-10-24' },
        now: NOW,
      },
    })
    const cells = w.findAll('td[data-member] li.event').filter((e) => e.text().includes('Dentist'))
    expect(cells).toHaveLength(2)
    const colours = cells.map((c) => c.attributes('style'))
    expect(colours[0]).not.toEqual(colours[1])
  })

  it('home agenda stripes an event for two members with both colours', () => {
    const d = data()
    d.members.push({ ...d.members[0]!, id: 'm2', display_name: 'Sam', color: 'red' })
    d.events[0]!.members = ['m1', 'm2']
    const w = mount(PanelHost, {
      props: { data: d, settings: settings(), panel: { kind: 'agenda' }, now: NOW },
    })
    const ev = w.findAll('button.event').find((e) => e.text().includes('Dentist'))!
    expect(ev.attributes('style')).toContain('linear-gradient')
  })

  it('week table shows "+N" for what a cell cannot fit, and it opens everything for that cell', async () => {
    const orig = globalThis.ResizeObserver
    class RO {
      constructor(private cb: () => void) {}
      observe() {
        this.cb()
      }
      disconnect() {}
    }
    globalThis.ResizeObserver = RO as never
    // Every cell reports 300 x 120 px (one column, two rows of slots).
    const w0 = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'clientWidth')
    const h0 = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'clientHeight')
    Object.defineProperty(HTMLElement.prototype, 'clientWidth', { configurable: true, value: 300 })
    Object.defineProperty(HTMLElement.prototype, 'clientHeight', { configurable: true, value: 120 })
    try {
      const d = data()
      const ev = (id: string, h: number) => ({
        ...d.events[0]!,
        id,
        title: `E${id}`,
        start_at: `2026-10-24T${String(h).padStart(2, '0')}:00:00Z`,
        end_at: `2026-10-24T${String(h + 1).padStart(2, '0')}:00:00Z`,
      })
      d.events = [ev('a', 8), ev('b', 9), ev('c', 10), ev('d', 11)]
      const w = mount(DashboardCanvas, {
        props: { data: d, settings: settings(), now: NOW },
      })
      await w.find('[data-view="week"]').trigger('click')
      const cell = w.find('tr[data-day="2026-10-24"] td[data-member="m1"]')
      expect(cell.findAll('li.event')).toHaveLength(1)
      const more = cell.find('li.more button')
      expect(more.text()).toBe('+3')
      await more.trigger('click')
      expect(w.find('[data-testid="detail-modal"]').findAll('li.event')).toHaveLength(4)
    } finally {
      globalThis.ResizeObserver = orig
      if (w0) Object.defineProperty(HTMLElement.prototype, 'clientWidth', w0)
      if (h0) Object.defineProperty(HTMLElement.prototype, 'clientHeight', h0)
    }
  })

  it('clicking the weather opens the detailed forecast with today by the hour', async () => {
    const hourly = {
      time: ['2026-10-24T09:00', '2026-10-24T10:00', '2026-10-25T09:00'],
      temperature_2m: [8, 9, 7],
      weather_code: [1, 1, 1],
    }
    const daily = {
      time: ['2026-10-24', '2026-10-25'],
      weather_code: [1, 3],
      temperature_2m_max: [12, 11],
      temperature_2m_min: [4, 5],
    }
    const d = data({
      weather: { current: { weather_code: 1, temperature_2m: 9 }, hourly, daily },
    } as unknown as Partial<DashboardData>)
    const w = mount(DashboardCanvas, {
      props: { data: d, settings: settings({ panels: [{ kind: 'weather' }] }), now: NOW },
    })
    expect(w.find('[aria-label="Hourly forecast"]').exists()).toBe(false)
    await w.find('[data-testid="weather-panel"]').trigger('click')
    expect(w.findAll('[aria-label="Hourly forecast"] li')).toHaveLength(2)
    expect(w.findAll('[aria-label="Daily forecast"] li')).toHaveLength(2)
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
