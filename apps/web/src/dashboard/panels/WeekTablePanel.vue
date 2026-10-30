<script setup lang="ts">
import { computed } from 'vue'
import { textOn } from '../../lib/hues'
import { addDays, dayKey, formatRange, groupByDay } from '../../lib/time'
import { toEventLike, toMemberLike, visibleEvents, visibleMembers } from '../filter'
import { S } from '../strings'
import { tasksOf } from '../tasksModel'
import type { PanelProps } from '../types'

const DAYS = 7
const EVERYONE = 'everyone'

const props = defineProps<PanelProps>()
const tz = computed(() => props.data.household.tz)
const today = computed(() => dayKey(props.now, tz.value))
const likes = computed(() => props.data.members.map(toMemberLike))
const people = computed(() => visibleMembers(props.data, props.settings))
const columns = computed(() => [
  ...people.value.map((m) => ({ key: m.id, name: m.display_name })),
  { key: EVERYONE, name: S.everyone },
])
const byDay = computed(() =>
  groupByDay(
    visibleEvents(props.data, props.settings).map((e) => toEventLike(e, likes.value)),
    tz.value,
    today.value,
    DAYS,
  ),
)
const tasksByDay = computed(() => {
  const out = new Map<string, ReturnType<typeof tasksOf>>()
  for (const t of tasksOf(props.data)) {
    if (!t.due_at) continue
    const k = dayKey(t.due_at, tz.value)
    out.set(k, [...(out.get(k) ?? []), t])
  }
  return out
})
/** Whose column an item belongs in: each assignee/attendee, or "everyone" when it names nobody. */
const owners = (ids: string[]) => {
  const known = ids.filter((id) => people.value.some((m) => m.id === id))
  return known.length ? known : ids.length ? [] : [EVERYONE]
}
const rows = computed(() =>
  Array.from({ length: DAYS }, (_, i) => {
    const key = addDays(today.value, i)
    const d = new Date(`${key}T12:00:00Z`)
    const events = byDay.value.get(key) ?? []
    const tasks = tasksByDay.value.get(key) ?? []
    return {
      key,
      today: key === today.value,
      weekday: new Intl.DateTimeFormat(undefined, { weekday: 'short', timeZone: 'UTC' }).format(d),
      dom: d.getUTCDate(),
      cells: columns.value.map((c) => ({
        key: c.key,
        events: events.filter((e) => owners(e.members).includes(c.key)),
        tasks: tasks.filter((t) => owners(t.assignees).includes(c.key)),
      })),
    }
  }),
)
</script>

<template>
  <section class="weektable" aria-label="Week">
    <table>
      <thead>
        <tr>
          <th scope="col" class="day"></th>
          <th v-for="c in columns" :key="c.key" scope="col">{{ c.name }}</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="r in rows" :key="r.key" :class="{ today: r.today }" :data-day="r.key">
          <th scope="row" class="day">
            {{ r.weekday }} <span class="dom">{{ r.dom }}</span>
          </th>
          <td v-for="c in r.cells" :key="c.key" :data-member="c.key">
            <ul>
              <li
                v-for="t in c.tasks"
                :key="t.id"
                class="chip"
                :class="{ done: t.status === 'done' }"
                data-testid="task-chip"
              >
                ⚑ {{ t.title }}
              </li>
              <li
                v-for="(e, i) in c.events"
                :key="e.id ?? i"
                :class="{ busy: e.visibility === 'busy' }"
                :style="
                  e.visibility === 'busy'
                    ? undefined
                    : { background: e.color, color: textOn(e.color) }
                "
              >
                <span class="t">{{ formatRange(e, tz) }}</span>
                {{ e.visibility === 'busy' ? S.busy : e.title }}
              </li>
            </ul>
          </td>
        </tr>
      </tbody>
    </table>
  </section>
</template>

<style scoped>
table {
  width: 100%;
  border-collapse: collapse;
  table-layout: fixed;
  font-size: var(--font-size-sm);
}
th,
td {
  padding: var(--space-2);
  border: 1px solid var(--color-border);
  vertical-align: top;
  text-align: left;
  min-width: 0;
}
thead th {
  font-size: var(--font-size-base);
  background: var(--color-surface);
}
.day {
  width: 5em;
  font-size: var(--font-size-base);
  background: var(--color-surface);
}
.dom,
.t {
  color: var(--color-muted);
  font-weight: 400;
}
tr.today > * {
  border-top: 3px solid var(--color-accent);
  border-bottom: 3px solid var(--color-accent);
}
ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--space-1);
}
li {
  padding: var(--space-1) var(--space-2);
  overflow-wrap: anywhere;
}
li .t {
  display: block;
  color: inherit;
  opacity: 0.85;
}
li.chip {
  border: 2px dashed var(--color-accent);
}
li.chip.done {
  text-decoration: line-through;
  color: var(--color-muted);
}
li.busy {
  background: var(--color-border);
  color: var(--color-muted);
}
</style>
