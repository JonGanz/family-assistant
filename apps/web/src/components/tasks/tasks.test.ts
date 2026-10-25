import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import MemberPicker from './MemberPicker.vue'
import ProgressBar from './ProgressBar.vue'
import RruleBuilder from './RruleBuilder.vue'
import StepList from './StepList.vue'
import TaskItem from './TaskItem.vue'
import UndoToast from './UndoToast.vue'
import type { MemberRef, TaskItemModel } from '../../lib/tasks'

const ana: MemberRef = { id: 'm1', name: 'Ana', color: '#336699' }
const ben: MemberRef = { id: 'm2', name: 'Ben', color: '#aa5522' }
const now = new Date('2026-10-24T10:00:00Z')
const task: TaskItemModel = {
  id: 't1',
  title: 'Take out bins',
  dueAt: '2026-10-22T10:00:00Z',
  status: 'open',
  priorityFlag: 'high',
  assignees: [ana],
  stepsDone: 1,
  stepsTotal: 3,
  origin: 'manual',
}

describe('TaskItem', () => {
  const props = { task, now, tz: 'Europe/Berlin' }
  it('shows title, overdue due label, chips, steps and priority', () => {
    const w = mount(TaskItem, { props })
    expect(w.text()).toContain('Take out bins')
    expect(w.get('.due').text()).toBe('overdue 2 d')
    expect(w.get('.due').classes()).toContain('late')
    expect(w.text()).toContain('Ana')
    expect(w.text()).toContain('1/3 steps')
    expect(w.find('[aria-label="High priority"]').exists()).toBe(true)
  })
  it('emits complete, open and claim', async () => {
    const w = mount(TaskItem, {
      props: { ...props, task: { ...task, assignees: [] } },
    })
    await w.get('input.check').trigger('change')
    await w.get('.body').trigger('click')
    await w.get('.claim-btn').trigger('click')
    expect(w.emitted('complete')).toEqual([['t1']])
    expect(w.emitted('open')).toEqual([['t1']])
    expect(w.emitted('claim')).toEqual([['t1']])
  })
  it('applies the kiosk size and the claim slot', () => {
    const w = mount(TaskItem, {
      props: { ...props, size: 'kiosk' },
      slots: { claim: '<b class="slot">x</b>' },
    })
    expect(w.classes()).toContain('kiosk')
    expect(w.find('.slot').exists()).toBe(true)
  })
  it('does not mark done tasks overdue', () => {
    const w = mount(TaskItem, { props: { ...props, task: { ...task, status: 'done' } } })
    expect(w.get('.due').classes()).not.toContain('late')
  })
})

describe('StepList', () => {
  it('emits tick with the new state', async () => {
    const w = mount(StepList, {
      props: {
        steps: [
          { id: 's1', text: 'Sort', done: false },
          { id: 's2', text: 'Carry', done: true },
        ],
      },
    })
    const boxes = w.findAll('input')
    await boxes[0]!.setValue(true)
    await boxes[1]!.setValue(false)
    expect(w.emitted('tick')).toEqual([
      ['s1', true],
      ['s2', false],
    ])
  })
})

describe('ProgressBar', () => {
  it('is an accessible progressbar', () => {
    const w = mount(ProgressBar, { props: { done: 2, total: 5, label: 'Ana' } })
    const bar = w.get('[role=progressbar]')
    expect(bar.attributes('aria-valuenow')).toBe('2')
    expect(bar.attributes('aria-valuemax')).toBe('5')
    expect(w.get('.fill').attributes('style')).toContain('width: 40%')
    expect(w.classes()).not.toContain('complete')
  })
  it('turns complete and supports kiosk', () => {
    const w = mount(ProgressBar, { props: { done: 5, total: 5, size: 'kiosk' } })
    expect(w.classes()).toEqual(expect.arrayContaining(['complete', 'kiosk']))
  })
  it('copes with an empty total', () => {
    const w = mount(ProgressBar, { props: { done: 0, total: 0 } })
    expect(w.get('.fill').attributes('style')).toContain('width: 0%')
  })
})

describe('MemberPicker', () => {
  it('emits pick and cancel, and marks the preselected member', async () => {
    const w = mount(MemberPicker, { props: { members: [ana, ben], preselected: 'm2' } })
    const btns = w.findAll('.member')
    expect(btns[1]!.attributes('aria-pressed')).toBe('true')
    expect(btns[0]!.attributes('aria-pressed')).toBe('false')
    await btns[0]!.trigger('click')
    await w.get('.cancel').trigger('click')
    expect(w.emitted('pick')).toEqual([['m1']])
    expect(w.emitted('cancel')).toHaveLength(1)
  })
})

describe('UndoToast', () => {
  it('counts down, emits undo and expire', async () => {
    vi.useFakeTimers()
    try {
      const w = mount(UndoToast, { props: { message: 'Done', seconds: 2 } })
      expect(w.get('[data-testid=countdown]').text()).toBe('2s')
      await vi.advanceTimersByTimeAsync(1000)
      expect(w.get('[data-testid=countdown]').text()).toBe('1s')
      await w.get('.undo').trigger('click')
      expect(w.emitted('undo')).toHaveLength(1)
      await vi.advanceTimersByTimeAsync(1000)
      expect(w.emitted('expire')).toHaveLength(1)
      w.unmount()
    } finally {
      vi.useRealTimers()
    }
  })
})

describe('RruleBuilder', () => {
  it('builds a weekly rule from the controls', async () => {
    const w = mount(RruleBuilder, {
      props: {
        modelValue: 'FREQ=WEEKLY;BYDAY=SA',
        'onUpdate:modelValue': (v: string) => w.setProps({ modelValue: v }),
      },
    })
    await w.get('[data-testid=day-MO]').setValue(true)
    await w.get('[data-testid=interval]').setValue(2)
    expect(w.props('modelValue')).toBe('FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,SA')
  })
  it('builds monthly nth weekday', async () => {
    const w = mount(RruleBuilder, {
      props: {
        modelValue: 'FREQ=MONTHLY;BYDAY=2TU',
        'onUpdate:modelValue': (v: string) => w.setProps({ modelValue: v }),
      },
    })
    expect(w.get<HTMLSelectElement>('[data-testid=nth]').element.value).toBe('2')
    await w.get('[data-testid=nth]').setValue('-1')
    expect(w.props('modelValue')).toBe('FREQ=MONTHLY;BYDAY=-1TU')
  })
  it('falls back to a raw input for unsupported rules', async () => {
    const text = 'FREQ=YEARLY;BYMONTH=3'
    const w = mount(RruleBuilder, {
      props: {
        modelValue: text,
        'onUpdate:modelValue': (v: string) => w.setProps({ modelValue: v }),
      },
    })
    expect(w.get<HTMLInputElement>('[data-testid=raw]').element.value).toBe(text)
    await w.get('[data-testid=raw]').setValue('FREQ=YEARLY;BYMONTH=4')
    expect(w.props('modelValue')).toBe('FREQ=YEARLY;BYMONTH=4')
  })
})
