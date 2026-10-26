<script setup lang="ts">
import ErrorBanner from '../components/ErrorBanner.vue'
import { computed, onMounted, ref, watch } from 'vue'
import DateRangeNav from '../components/DateRangeNav.vue'
import EventCard from '../components/EventCard.vue'
import UndoToast from '../components/tasks/UndoToast.vue'
import { rangeFor, shiftAnchor } from '../composables/useRange'
import { addDays, dayKey, formatDayHeading, groupByDay, startOfDay } from '../lib/time'
import type { RangeMode } from '../lib/types'
import { useCalendarStore } from '../stores/calendar'
import { useHouseholdStore } from '../stores/household'
import { useMembersStore } from '../stores/members'
import { chipsByDay, memberRefs, type TaskChip } from '../stores/taskMap'
import { useTasksStore } from '../stores/tasks'
import AgendaList from './parts/AgendaList.vue'
import { memberLikes, toEventLike } from './shared'

type View = 'agenda' | 'week' | 'month'

const household = useHouseholdStore()
const calendar = useCalendarStore()
const members = useMembersStore()
const tasks = useTasksStore()

const tz = computed(() => household.tz)
const narrow = typeof matchMedia === 'function' && matchMedia('(max-width: 700px)').matches
const view = ref<View>(narrow ? 'agenda' : 'week')
const anchor = ref(dayKey(new Date(), household.tz))
const member = ref('')

const navMode = computed<RangeMode>(() => (view.value === 'month' ? 'month' : 'week'))
const span = computed(() => rangeFor(navMode.value, anchor.value))
/** Month view also shows the leading and trailing days of the boundary weeks. */
const grid = computed(() => {
  if (view.value !== 'month') return span.value
  return {
    from: rangeFor('week', span.value.from).from,
    to: rangeFor('week', addDays(span.value.to, -1)).to,
  }
})
const days = computed(() => {
  const [fy, fm, fd] = grid.value.from.split('-').map(Number)
  const [ty, tm, td] = grid.value.to.split('-').map(Number)
  return Math.round((Date.UTC(ty!, tm! - 1, td!) - Date.UTC(fy!, fm! - 1, fd!)) / 86_400_000)
})
const likes = computed(() => memberLikes(members.items))
const events = computed(() =>
  calendar.eventsIn(grid.value.from, grid.value.to, tz.value, member.value || undefined),
)
const byDay = computed(() =>
  groupByDay(
    events.value.map((e) => toEventLike(e, likes.value)),
    tz.value,
    grid.value.from,
    days.value,
  ),
)
const today = computed(() => dayKey(new Date(), tz.value))
const label = computed(() => {
  if (view.value === 'month') {
    return new Intl.DateTimeFormat(undefined, {
      timeZone: 'UTC',
      month: 'long',
      year: 'numeric',
    }).format(new Date(`${span.value.from}T12:00:00Z`))
  }
  return `${formatDayHeading(span.value.from, tz.value)} – ${formatDayHeading(addDays(span.value.to, -1), tz.value)}`
})
const weekdays = computed(() =>
  Array.from({ length: 7 }, (_, i) =>
    new Intl.DateTimeFormat(undefined, { timeZone: 'UTC', weekday: 'short' }).format(
      new Date(`${addDays('2026-10-19', i)}T12:00:00Z`),
    ),
  ),
)

const rangeInstants = computed(() => ({
  from: startOfDay(tz.value, grid.value.from).toISOString(),
  to: startOfDay(tz.value, grid.value.to).toISOString(),
}))
const chips = computed(() =>
  chipsByDay(
    tasks.rangeTasks(rangeInstants.value.from, rangeInstants.value.to),
    memberRefs(members.items),
    tz.value,
  ),
)
const chipsOf = (k: string): TaskChip[] => chips.value.get(k) ?? []
const allChips = computed(() => [...chips.value.entries()].sort(([a], [b]) => a.localeCompare(b)))
async function toggleChip(c: TaskChip) {
  try {
    if (c.done) await tasks.reopenDone(c.taskId)
    else await tasks.complete({ id: c.taskId, title: c.title })
  } catch {
    /* the store keeps its last error; the chip stays as it was */
  }
}

function load() {
  void calendar.ensureRange(grid.value.from, grid.value.to, tz.value, member.value || undefined)
  void tasks.loadRange(rangeInstants.value.from, rangeInstants.value.to)
}
onMounted(() => {
  void members.load()
  load()
})
watch([() => grid.value.from, () => grid.value.to, member, tz], load)

const dayNumber = (k: string) => Number(k.slice(8))
</script>

<template>
  <section>
    <h1>Calendar</h1>
    <div class="controls">
      <DateRangeNav
        :label="label"
        @prev="anchor = shiftAnchor(navMode, anchor, -1)"
        @next="anchor = shiftAnchor(navMode, anchor, 1)"
        @today="anchor = dayKey(new Date(), tz)"
      />
      <span class="modes" role="group" aria-label="View">
        <button
          v-for="v in ['agenda', 'week', 'month'] as View[]"
          :key="v"
          type="button"
          :aria-pressed="view === v"
          @click="view = v"
        >
          {{ v }}
        </button>
      </span>
      <label class="filter">
        Member
        <select v-model="member" name="member">
          <option value="">Everyone</option>
          <option v-for="m in members.items" :key="m.id" :value="m.id">{{ m.display_name }}</option>
        </select>
      </label>
    </div>
    <ErrorBanner v-if="calendar.error" :message="calendar.error" />

    <AgendaList
      v-if="view === 'agenda'"
      :events="events"
      :tz="tz"
      :from="grid.from"
      :days="days"
      :members="likes"
      hide-empty
    />
    <section v-if="view === 'agenda' && allChips.length" class="agenda-tasks">
      <h3>Tasks</h3>
      <div v-for="[k, list] in allChips" :key="k">
        <strong>{{ formatDayHeading(k, tz, today) }}</strong>
        <ul class="chips">
          <li v-for="c in list" :key="c.id">
            <label :class="{ done: c.done }" :style="{ borderLeftColor: c.color }">
              <input type="checkbox" :checked="c.done" @change="toggleChip(c)" />
              {{ c.title }}
            </label>
          </li>
        </ul>
      </div>
    </section>

    <div v-else-if="view === 'week'" class="week">
      <section v-for="[k, list] in byDay" :key="k" :class="{ today: k === today }">
        <h3>{{ formatDayHeading(k, tz, today) }}</h3>
        <div class="list">
          <EventCard v-for="e in list" :key="e.id" :event="e" :tz="tz" :members="likes" />
          <ul v-if="chipsOf(k).length" class="chips">
            <li v-for="c in chipsOf(k)" :key="c.id">
              <label :class="{ done: c.done }" :style="{ borderLeftColor: c.color }">
                <input type="checkbox" :checked="c.done" @change="toggleChip(c)" />
                {{ c.title }}
              </label>
            </li>
          </ul>
        </div>
      </section>
    </div>

    <div v-else class="month">
      <div v-for="w in weekdays" :key="w" class="dow">{{ w }}</div>
      <div
        v-for="[k, list] in byDay"
        :key="k"
        class="cell"
        :class="{ today: k === today, outside: k < span.from || k >= span.to }"
      >
        <div class="num">{{ dayNumber(k) }}</div>
        <ul>
          <li
            v-for="e in list.slice(0, 3)"
            :key="e.id"
            :class="{ busy: e.visibility === 'busy' }"
            :style="{ borderLeftColor: e.color }"
          >
            {{ e.visibility === 'busy' ? 'Busy' : e.title }}
          </li>
          <li v-if="list.length > 3" class="more">+{{ list.length - 3 }} more</li>
          <li
            v-for="c in chipsOf(k)"
            :key="c.id"
            class="task-chip"
            :class="{ done: c.done }"
            :style="{ borderLeftColor: c.color }"
          >
            <label>
              <input type="checkbox" :checked="c.done" @change="toggleChip(c)" />
              {{ c.title }}
            </label>
          </li>
        </ul>
      </div>
    </div>
    <UndoToast
      v-if="tasks.undo"
      :key="tasks.undo.id"
      :message="`Completed “${tasks.undo.title}”`"
      @undo="tasks.undoLast()"
      @expire="tasks.expireUndo()"
    />
  </section>
</template>

<style scoped>
.chips {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: 2px;
}
.chips label,
.task-chip label {
  display: flex;
  gap: var(--space-1);
  align-items: center;
  border-left: 4px solid var(--color-accent);
  padding-left: var(--space-1);
  font-size: var(--font-size-sm);
}
.task-chip {
  font-size: var(--font-size-xs);
}
label.done,
.task-chip.done {
  text-decoration: line-through;
  color: var(--color-muted);
}
.controls {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-3);
  margin-bottom: var(--space-3);
}
.modes {
  display: flex;
  gap: var(--space-1);
}
button[aria-pressed='true'] {
  background: var(--color-accent);
  color: var(--color-bg);
}
.filter {
  margin-left: auto;
  display: flex;
  gap: var(--space-2);
  align-items: center;
}
.week {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(14rem, 1fr));
  gap: var(--space-3);
}
.week .list {
  display: grid;
  gap: var(--space-2);
}
.week h3 {
  margin: 0 0 var(--space-2);
  font-size: var(--font-size-base);
}
.today > h3 {
  color: var(--color-accent);
}
.month {
  display: grid;
  grid-template-columns: repeat(7, minmax(0, 1fr));
  gap: 1px;
  background: var(--color-border);
  border: 1px solid var(--color-border);
}
.dow {
  background: var(--color-surface);
  padding: var(--space-1) var(--space-2);
  font-size: var(--font-size-sm);
  text-align: center;
}
.cell {
  background: var(--color-bg);
  min-height: 5.5rem;
  padding: var(--space-1);
  min-width: 0;
}
.cell.outside {
  background: var(--color-surface);
  color: var(--color-muted);
}
.cell.today .num {
  color: var(--color-accent);
  font-weight: 700;
}
.num {
  font-size: var(--font-size-sm);
}
.cell ul {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: 2px;
}
.cell li {
  font-size: var(--font-size-xs);
  border-left: 4px solid var(--color-accent);
  padding-left: var(--space-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.cell li.busy {
  background: repeating-linear-gradient(
    45deg,
    transparent,
    transparent 4px,
    var(--color-border) 4px,
    var(--color-border) 6px
  );
  color: var(--color-muted);
}
.cell li.more {
  border-left: 0;
  color: var(--color-muted);
}
</style>
