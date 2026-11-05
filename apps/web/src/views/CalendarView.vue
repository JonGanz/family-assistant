<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import DateRangeNav from '../components/DateRangeNav.vue'
import EventCard from '../components/EventCard.vue'
import { rangeFor, shiftAnchor } from '../composables/useRange'
import { stripeStyle } from '../lib/eventColors'
import { addDays, dayKey, formatDayHeading, groupByDay, startOfDay } from '../lib/time'
import type { EventLike, RangeMode } from '../lib/types'
import { useCalendarStore } from '../stores/calendar'
import { useHouseholdStore } from '../stores/household'
import { useMembersStore } from '../stores/members'
import { chipsByDay, memberRefs, type TaskChip } from '../stores/taskMap'
import { useTasksStore } from '../stores/tasks'
import {
  UiCheckbox,
  UiChip,
  UiErrorState,
  UiPage,
  UiSegmented,
  UiSheet,
  UiSkeleton,
  UiTopAppBar,
  useBreakpoint,
  useSnackbar,
} from '../ui'
import AgendaList from './parts/AgendaList.vue'
import { describeError, memberLikes, toEventLike } from './shared'

type View = 'agenda' | 'week' | 'month'

const household = useHouseholdStore()
const calendar = useCalendarStore()
const members = useMembersStore()
const tasks = useTasksStore()
const snackbar = useSnackbar()
const bp = useBreakpoint()

const tz = computed(() => household.tz)
// Agenda on phones, week on wider screens; the segmented control overrides.
const view = ref<View>(bp.value === 'compact' ? 'agenda' : 'week')
const anchor = ref(dayKey(new Date(), household.tz))
const member = ref('')
const detail = ref<EventLike | null>(null)
const detailOpen = computed({
  get: () => detail.value !== null,
  set: (v) => {
    if (!v) detail.value = null
  },
})

const VIEWS = [
  { value: 'agenda', label: 'Agenda' },
  { value: 'week', label: 'Week' },
  { value: 'month', label: 'Month' },
]
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
// Completing a task chip is instant (D-096); the store arms a 10 s undo that the snackbar offers.
async function toggleChip(c: TaskChip) {
  try {
    if (c.done) await tasks.reopenDone(c.taskId)
    else await tasks.complete({ id: c.taskId, title: c.title })
  } catch (e) {
    snackbar.show({ text: describeError(e, 'That did not work.') })
  }
}
const UNDO_MS = 10_000
watch(
  () => tasks.undo,
  (u) => {
    if (!u) return
    snackbar.show({
      text: `Completed “${u.title}”`,
      timeoutMs: UNDO_MS,
      action: {
        label: 'Undo',
        run: async () => {
          try {
            await tasks.undoLast()
          } catch (e) {
            snackbar.show({ text: describeError(e, 'That did not work.') })
          }
        },
      },
    })
    setTimeout(() => {
      if (tasks.undo?.id === u.id) tasks.expireUndo()
    }, UNDO_MS)
  },
)

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
const loadingFirst = computed(() => calendar.loading && events.value.length === 0)
</script>

<template>
  <UiPage wide>
    <template #top>
      <UiTopAppBar title="Calendar" />
    </template>

    <div class="controls">
      <DateRangeNav
        :label="label"
        @prev="anchor = shiftAnchor(navMode, anchor, -1)"
        @next="anchor = shiftAnchor(navMode, anchor, 1)"
        @today="anchor = dayKey(new Date(), tz)"
      />
      <UiSegmented
        :model-value="view"
        :options="VIEWS"
        label="View"
        @update:model-value="view = $event as View"
      />
      <div class="members" role="group" aria-label="Member">
        <UiChip variant="filter" :selected="member === ''" @click="member = ''">Everyone</UiChip>
        <UiChip
          v-for="m in members.items"
          :key="m.id"
          variant="filter"
          :selected="member === m.id"
          @click="member = member === m.id ? '' : m.id"
        >
          {{ m.display_name }}
        </UiChip>
      </div>
    </div>

    <UiErrorState v-if="calendar.error" :message="calendar.error" @retry="load" />
    <UiSkeleton v-else-if="loadingFirst" :lines="5" />

    <template v-else-if="view === 'agenda'">
      <AgendaList
        :events="events"
        :tz="tz"
        :from="grid.from"
        :days="days"
        :members="likes"
        hide-empty
        selectable
        @select="detail = $event"
      />
      <section v-if="allChips.length" class="agenda-tasks" aria-label="Tasks">
        <h2>Tasks</h2>
        <div v-for="[k, list] in allChips" :key="k">
          <strong>{{ formatDayHeading(k, tz, today) }}</strong>
          <ul class="chips">
            <li v-for="c in list" :key="c.id" :style="{ borderLeftColor: c.color }">
              <UiCheckbox
                :model-value="c.done"
                :label="c.title"
                @update:model-value="toggleChip(c)"
              />
            </li>
          </ul>
        </div>
      </section>
    </template>

    <div v-else-if="view === 'week'" class="week">
      <section
        v-for="[k, list] in byDay"
        :key="k"
        class="day"
        :class="{ today: k === today }"
        :data-day="k"
      >
        <h2>{{ formatDayHeading(k, tz, today) }}</h2>
        <div class="list">
          <EventCard
            v-for="e in list"
            :key="e.id"
            :event="e"
            :tz="tz"
            :members="likes"
            square
            selectable
            @select="detail = $event"
          />
          <ul v-if="chipsOf(k).length" class="chips">
            <li v-for="c in chipsOf(k)" :key="c.id" :style="{ borderLeftColor: c.color }">
              <UiCheckbox
                :model-value="c.done"
                :label="c.title"
                @update:model-value="toggleChip(c)"
              />
            </li>
          </ul>
        </div>
      </section>
    </div>

    <div v-else class="month-scroll">
      <div class="month">
        <div v-for="w in weekdays" :key="w" class="dow">{{ w }}</div>
        <div
          v-for="[k, list] in byDay"
          :key="k"
          class="cell"
          :class="{ today: k === today, outside: k < span.from || k >= span.to }"
        >
          <div class="num">{{ dayNumber(k) }}</div>
          <ul>
            <li v-for="e in list.slice(0, 3)" :key="e.id">
              <button
                type="button"
                class="ev"
                :class="{ busy: e.visibility === 'busy' }"
                :style="stripeStyle(e)"
                @click="detail = e"
              >
                {{ e.visibility === 'busy' ? 'Busy' : e.title }}
              </button>
            </li>
            <li v-if="list.length > 3" class="more">+{{ list.length - 3 }} more</li>
            <li
              v-for="c in chipsOf(k)"
              :key="c.id"
              class="task-chip"
              :style="{ borderLeftColor: c.color }"
            >
              <UiCheckbox
                :model-value="c.done"
                :label="c.title"
                @update:model-value="toggleChip(c)"
              />
            </li>
          </ul>
        </div>
      </div>
    </div>

    <UiSheet
      v-model="detailOpen"
      :title="detail?.visibility === 'busy' ? 'Busy' : (detail?.title ?? '')"
    >
      <EventCard v-if="detail" :event="detail" :tz="tz" :members="likes" />
    </UiSheet>
  </UiPage>
</template>

<style scoped>
.controls {
  display: grid;
  gap: var(--ui-space-3);
  justify-items: start;
}
.controls > :first-child {
  width: 100%;
}
.members {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
}
h2 {
  margin: 0 0 var(--ui-space-2);
  font: var(--ui-type-title-md);
}
.chips {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: var(--ui-space-1);
}
.chips li,
.task-chip {
  border-left: 4px solid var(--ui-primary);
  padding-left: var(--ui-space-2);
  border-radius: var(--ui-radius-none);
}
.week {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(14rem, 1fr));
  gap: var(--ui-space-3);
}
.week .list {
  display: grid;
  gap: var(--ui-space-2);
}
.day.today > h2 {
  color: var(--ui-primary);
}
.month-scroll {
  overflow-x: auto;
}
.month {
  display: grid;
  grid-template-columns: repeat(7, minmax(5rem, 1fr));
  gap: 1px;
  background: var(--ui-outline-variant);
  border: 1px solid var(--ui-outline-variant);
}
.dow {
  background: var(--ui-surface-container-low);
  padding: var(--ui-space-1) var(--ui-space-2);
  font: var(--ui-type-label-md);
  text-align: center;
}
.cell {
  background: var(--ui-surface);
  min-height: 5.5rem;
  padding: var(--ui-space-1);
  min-width: 0;
}
.cell.outside {
  background: var(--ui-surface-container-low);
  color: var(--ui-on-surface-variant);
}
.cell.today .num {
  color: var(--ui-primary);
  font-weight: 700;
}
.num {
  font: var(--ui-type-label-md);
}
.cell ul {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: 2px;
}
.ev {
  display: block;
  width: 100%;
  text-align: left;
  font: var(--ui-type-body-sm);
  color: inherit;
  background: var(--ui-surface-container-low);
  border: 0;
  border-left: 4px solid var(--ui-primary);
  border-radius: var(--ui-radius-none);
  padding: 0 var(--ui-space-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  cursor: pointer;
}
.ev:focus-visible {
  outline: var(--ui-focus-ring);
}
.ev.busy {
  background: repeating-linear-gradient(
    45deg,
    transparent,
    transparent 4px,
    var(--ui-outline-variant) 4px,
    var(--ui-outline-variant) 6px
  );
  color: var(--ui-on-surface-variant);
}
.more {
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
</style>
