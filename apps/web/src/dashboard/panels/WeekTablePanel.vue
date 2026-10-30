<script setup lang="ts">
import { computed } from 'vue'
import WeatherIcon from '../../components/WeatherIcon.vue'
import { useUnits } from '../../composables/useUnits'
import { stripeStyle } from '../../lib/eventColors'
import { addDays, dayKey, groupByDay } from '../../lib/time'
import { toEventLike, toMemberLike, visibleEvents, visibleMembers } from '../filter'
import DayCell from '../DayCell.vue'
import { tasksOf } from '../tasksModel'
import type { DetailRequest, PanelProps } from '../types'
import { dailyOf } from '../weatherModel'

const DAYS = 7

const props = defineProps<PanelProps>()
const emit = defineEmits<{ detail: [d: DetailRequest] }>()
const tz = computed(() => props.data.household.tz)
const today = computed(() => dayKey(props.now, tz.value))
/** The week's first day (the canvas picks it from the household's first day of the week). */
const start = computed(() =>
  typeof props.panel['start'] === 'string' ? (props.panel['start'] as string) : today.value,
)
const hidePast = computed(() => props.panel['hidePast'] === true)
const likes = computed(() => props.data.members.map(toMemberLike))
const people = computed(() => visibleMembers(props.data, props.settings))
const columns = computed(() =>
  people.value.map((m) => {
    const color = toMemberLike(m).color
    return {
      key: m.id,
      name: m.display_name,
      color,
      stripe: stripeStyle({ color, colors: [color] }),
    }
  }),
)
const { format } = useUnits(computed(() => props.data.household.units))
const forecast = computed(
  () => new Map(dailyOf(props.data.weather ?? ({} as never), DAYS).map((d) => [d.date, d])),
)
const temp = (v: number | null) => (v === null ? '–' : format(v, 'temp'))
/** Over by now: timed events by their end, all-day events once their last day is behind us. */
function ended(e: {
  all_day: boolean
  end_at: string
  start_date?: string | null
  end_date?: string | null
}) {
  if (e.all_day && e.start_date) {
    const endExclusive =
      e.end_date && e.end_date > e.start_date ? e.end_date : addDays(e.start_date, 1)
    return endExclusive <= today.value
  }
  return Date.parse(e.end_at) <= props.now.getTime()
}
const byDay = computed(() =>
  groupByDay(
    visibleEvents(props.data, props.settings)
      .filter((e) => !hidePast.value || !ended(e))
      .map((e) => toEventLike(e, likes.value)),
    tz.value,
    start.value,
    DAYS,
  ),
)
const tasksByDay = computed(() => {
  const out = new Map<string, ReturnType<typeof tasksOf>>()
  for (const t of tasksOf(props.data)) {
    if (!t.due_at) continue
    const k = dayKey(t.due_at, tz.value)
    // With past items hidden, a task goes only once it is completed (open ones stay until done).
    if (hidePast.value && t.status === 'done' && k <= today.value) continue
    out.set(k, [...(out.get(k) ?? []), t])
  }
  return out
})
/** The people with a column that an item belongs to. Household-wide items have no column. */
const owners = (ids: string[]) => ids.filter((id) => people.value.some((m) => m.id === id))
const rows = computed(() =>
  Array.from({ length: DAYS }, (_, i) => {
    const key = addDays(start.value, i)
    const d = new Date(`${key}T12:00:00Z`)
    const events = byDay.value.get(key) ?? []
    const tasks = tasksByDay.value.get(key) ?? []
    return {
      key,
      today: key === today.value,
      weekday: new Intl.DateTimeFormat(undefined, { weekday: 'short', timeZone: 'UTC' }).format(d),
      dom: d.getUTCDate(),
      fc: forecast.value.get(key),
      cells: columns.value.map((c) => ({
        key: c.key,
        name: c.name,
        color: c.color,
        stripe: c.stripe,
        events: events.filter((e) => owners(e.members).includes(c.key)),
        tasks: tasks.filter((t) => owners(t.assignees).includes(c.key)),
      })),
    }
  }),
)
</script>

<template>
  <section class="weektable" aria-label="Week">
    <table role="table" :style="{ '--cols': columns.length }">
      <thead role="rowgroup">
        <tr role="row">
          <th scope="col" role="columnheader" class="day"></th>
          <th v-for="c in columns" :key="c.key" scope="col" role="columnheader">{{ c.name }}</th>
        </tr>
      </thead>
      <tbody role="rowgroup">
        <tr v-for="r in rows" :key="r.key" role="row" :class="{ today: r.today }" :data-day="r.key">
          <th scope="row" role="rowheader" class="day">
            <div class="dom">{{ r.dom }}</div>
            <div class="wd">{{ r.weekday }}</div>
            <div v-if="r.fc" class="fcicon">
              <WeatherIcon :code="r.fc.code" size="2rem" />
            </div>
            <div v-if="r.fc" class="fctemps">
              <span class="hi">{{ temp(r.fc.max) }}</span>
              <span class="sep">|</span>
              <span class="lo">{{ temp(r.fc.min) }}</span>
            </div>
          </th>
          <td v-for="c in r.cells" :key="c.key" role="cell" :data-member="c.key">
            <DayCell
              :events="c.events"
              :tasks="c.tasks"
              :tz="tz"
              :stripe="c.stripe"
              :color="c.color"
              :label="`${c.name}, ${r.weekday} ${r.dom}`"
              @detail="(d: DetailRequest) => emit('detail', d)"
            />
          </td>
        </tr>
      </tbody>
    </table>
  </section>
</template>

<style scoped>
.weektable {
  /* Absolute, so its height is definite (the cell is positioned) and the rows can share it. */
  position: absolute;
  inset: 0;
}
table {
  height: 100%;
  display: flex;
  flex-direction: column;
  width: 100%;
  font-size: var(--font-size-sm);
}
/* Equal rows that share the height exactly; a cell never grows, it folds extra items into "+N". */
tbody {
  flex: 1;
  display: grid;
  grid-auto-rows: minmax(0, 1fr);
}
tr {
  display: grid;
  grid-template-columns: 7.5em repeat(var(--cols), minmax(0, 1fr));
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
td {
  position: relative;
  padding: 0;
}
.day {
  font-size: var(--font-size-base);
  font-weight: 400;
  background: var(--color-surface);
}
.dom {
  font-size: var(--font-size-lg);
  font-weight: 700;
}
.wd {
  font-weight: 400;
}
.fcicon {
  margin-top: var(--space-1);
  color: var(--color-text);
}
.fcicon :deep(.weather-icon) {
  flex: none;
}
.fctemps {
  display: flex;
  gap: var(--space-1);
  white-space: nowrap;
  font-size: var(--font-size-sm);
}
.fctemps .hi {
  font-weight: 700;
}
.lo,
.sep {
  color: var(--color-muted);
}
tr.today > * {
  border-top: 3px solid var(--color-accent);
  border-bottom: 3px solid var(--color-accent);
}
</style>
