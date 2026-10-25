import { describe, expect, it } from 'vitest'
import type { TaskListView, TaskView } from '../api/client'
import { addableLists, assignableMembers, chipsByDay, toTaskItem } from './taskMap'

const list = (id: string, child: 'own' | 'none') =>
  ({ id, name: id, child_can_add: child }) as TaskListView
const task = (o: Partial<TaskView>): TaskView =>
  ({
    id: 't',
    title: 'Water plants',
    status: 'open',
    origin: 'manual',
    priority_flag: 'normal',
    calendar_display: 'until_done',
    assignees: [],
    steps: [],
    claimed_by: null,
    due_at: '2026-10-26T10:00:00Z',
    ...o,
  }) as TaskView

const refs = new Map([
  ['a', { id: 'a', name: 'Alex', color: '#ff0000' }],
  ['k', { id: 'k', name: 'Kai', color: '#00ff00' }],
])

describe('add permissions', () => {
  const lists = [list('chores', 'none'), list('homework', 'own')]
  it('adults may add to every list, children only to child_can_add = own', () => {
    expect(addableLists(lists, { id: 'a', role: 'adult' }).map((l) => l.id)).toEqual([
      'chores',
      'homework',
    ])
    expect(addableLists(lists, { id: 'k', role: 'child' }).map((l) => l.id)).toEqual(['homework'])
    expect(addableLists(lists, null)).toEqual([])
  })
  it('children can only assign to themselves', () => {
    const people = [{ id: 'a' }, { id: 'k' }]
    expect(assignableMembers(people, { id: 'k', role: 'child' })).toEqual([{ id: 'k' }])
    expect(assignableMembers(people, { id: 'a', role: 'adult' })).toHaveLength(2)
  })
})

describe('toTaskItem', () => {
  it('maps assignees and counts done steps', () => {
    const m = toTaskItem(
      task({
        assignees: ['a'],
        steps: [
          { id: 's1', ord: 0, text: 'x', done_at: 'now', done_by: 'a' },
          { id: 's2', ord: 1, text: 'y', done_at: null, done_by: null },
        ],
      }),
      refs,
    )
    expect(m.assignees[0]?.name).toBe('Alex')
    expect([m.stepsDone, m.stepsTotal]).toEqual([1, 2])
  })
})

describe('calendar chips', () => {
  it('groups by household-local day and colours by first assignee', () => {
    const m = chipsByDay([task({ assignees: ['k'] })], refs, 'UTC')
    expect(m.get('2026-10-26')?.[0]?.color).toBe('#00ff00')
  })
  it('keeps done chips only for key dates (always) and drops hidden/skipped', () => {
    const m = chipsByDay(
      [
        task({ id: '1', status: 'done', calendar_display: 'always' }),
        task({ id: '2', status: 'done', calendar_display: 'until_done' }),
        task({ id: '3', calendar_display: 'hidden' }),
        task({ id: '4', status: 'skipped', calendar_display: 'always' }),
        task({ id: '5', due_at: null }),
      ],
      refs,
      'UTC',
    )
    const all = [...m.values()].flat()
    expect(all.map((c) => c.id)).toEqual(['1'])
    expect(all[0]?.done).toBe(true)
  })
  it('uses the household timezone for the day', () => {
    const m = chipsByDay([task({ due_at: '2026-10-26T23:30:00Z' })], refs, 'Asia/Tokyo')
    expect([...m.keys()]).toEqual(['2026-10-27'])
  })
})
