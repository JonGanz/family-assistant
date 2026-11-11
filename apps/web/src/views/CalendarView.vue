<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import type { EventView } from '../api/client'
import DateRangeNav from '../components/DateRangeNav.vue'
import EventCard from '../components/EventCard.vue'
import WeekGrid from '../components/calendar/WeekGrid.vue'
import { rangeFor, shiftAnchor } from '../composables/useRange'
import { weekStartKey, type WeekStart } from '../dashboard/weekModel'
import { stripeStyle, trimStyle } from '../lib/eventColors'
import { addDays, dayKey, formatDayHeading, groupByDay, startOfDay } from '../lib/time'
import type { EventLike, RangeMode } from '../lib/types'
import { useCalendarStore } from '../stores/calendar'
import { useHouseholdStore } from '../stores/household'
import { useMembersStore } from '../stores/members'
import { chipsByDay, memberRefs, type TaskChip } from '../stores/taskMap'
import { useTasksStore } from '../stores/tasks'
import {
  UiButton,
  UiCheckbox,
  UiFilterChips,
  UiErrorState,
  UiPage,
  UiSheet,
  UiSkeleton,
  UiTabs,
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
// The view a person last used is reopened (D-103); with none stored, Agenda on phones and Week
// on wider screens.
const VIEW_KEY = 'fa.calendar.view'
function readView(): View | null {
  try {
    const v = localStorage.getItem(VIEW_KEY)
    return v === 'agenda' || v === 'week' || v === 'month' ? v : null
  } catch {
    return null
  }
}
const view = ref<View>(readView() ?? (bp.value === 'compact' ? 'agenda' : 'week'))
function setView(v: View) {
  view.value = v
  try {
    localStorage.setItem(VIEW_KEY, v)
  } catch {
    // Blocked storage: the choice is simply not remembered.
  }
}
const anchor = ref(dayKey(new Date(), household.tz))
/** The chosen members (D-105); empty means everyone. */
const chosen = ref<string[]>([])
const detail = ref<EventLike | null>(null)
/** The day whose full list is open ("+N more"); an event opened from it can go back to it. */
const dayOpen = ref<string | null>(null)
const detailOpen = computed({
  get: () => detail.value !== null || dayOpen.value !== null,
  set: (v) => {
    if (!v) {
      detail.value = null
      dayOpen.value = null
    }
  },
})
const dayEvents = computed(() => (dayOpen.value ? (byDay.value.get(dayOpen.value) ?? []) : []))
const sheetTitle = computed(() => {
  if (detail.value) return detail.value.visibility === 'busy' ? 'Busy' : (detail.value.title ?? '')
  return dayOpen.value ? formatDayHeading(dayOpen.value, tz.value, today.value) : ''
})

const VIEWS = [
  { value: 'agenda', label: 'Agenda' },
  { value: 'week', label: 'Week' },
  { value: 'month', label: 'Month' },
]
const navMode = computed<RangeMode>(() => (view.value === 'month' ? 'month' : 'week'))
const weekStart = computed<WeekStart>(() => (household.view?.week_start as WeekStart) ?? 'monday')
/** The household's week (from its first day of the week) or calendar month holding `a`; `to` is exclusive. */
function rangeAt(mode: RangeMode, a: string): { from: string; to: string } {
  if (mode === 'week') {
    const from = weekStartKey(a, weekStart.value)
    return { from, to: addDays(from, 7) }
  }
  return rangeFor(mode, a)
}
const span = computed(() => rangeAt(navMode.value, anchor.value))
/** Month view also shows the leading and trailing days of the boundary weeks. */
const grid = computed(() => {
  if (view.value !== 'month') return span.value
  return {
    from: rangeAt('week', span.value.from).from,
    to: rangeAt('week', addDays(span.value.to, -1)).to,
  }
})
const days = computed(() => {
  const [fy, fm, fd] = grid.value.from.split('-').map(Number)
  const [ty, tm, td] = grid.value.to.split('-').map(Number)
  return Math.round((Date.UTC(ty!, tm! - 1, td!) - Date.UTC(fy!, fm! - 1, fd!)) / 86_400_000)
})
const likes = computed(() => memberLikes(members.items))
// The API filters by one member: each chosen member is fetched and the results merged.
const memberKeys = computed<(string | undefined)[]>(() =>
  chosen.value.length ? chosen.value : [undefined],
)
const events = computed(() => {
  const byId = new Map<string, EventView>()
  for (const m of memberKeys.value)
    for (const e of calendar.eventsIn(grid.value.from, grid.value.to, tz.value, m))
      byId.set(e.id, e)
  return [...byId.values()].sort((a, b) => a.start_at.localeCompare(b.start_at))
})
const memberOptions = computed(() =>
  members.items.map((m) => ({ value: m.id, label: m.display_name })),
)
const eventLikes = computed(() => events.value.map((e) => toEventLike(e, likes.value)))
const byDay = computed(() => groupByDay(eventLikes.value, tz.value, grid.value.from, days.value))
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
      new Date(
        `${addDays(weekStart.value === 'sunday' ? '2026-10-18' : '2026-10-19', i)}T12:00:00Z`,
      ),
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
  for (const m of memberKeys.value)
    void calendar.ensureRange(grid.value.from, grid.value.to, tz.value, m)
  void tasks.loadRange(rangeInstants.value.from, rangeInstants.value.to)
}
onMounted(() => {
  void members.load()
  load()
})
watch([() => grid.value.from, () => grid.value.to, chosen, tz], load)

const dayNumber = (k: string) => Number(k.slice(8))
const loadingFirst = computed(() => calendar.loading && events.value.length === 0)
</script>

<template>
  <UiPage wide>
    <template #top>
      <UiTopAppBar title="Calendar" />
    </template>

    <UiTabs
      :model-value="view"
      :options="VIEWS"
      label="View"
      @update:model-value="setView($event as View)"
    />

    <div class="controls">
      <DateRangeNav
        :label="label"
        @prev="anchor = shiftAnchor(navMode, anchor, -1)"
        @next="anchor = shiftAnchor(navMode, anchor, 1)"
        @today="anchor = dayKey(new Date(), tz)"
      />
      <UiFilterChips v-model="chosen" :options="memberOptions" label="Member" />
    </div>

    <UiErrorState v-if="calendar.error" :message="calendar.error" @retry="load" />
    <UiSkeleton v-else-if="loadingFirst" :lines="5" />

    <template v-else-if="view === 'agenda' || view === 'week'">
      <AgendaList
        v-if="view === 'agenda'"
        :events="events"
        :tz="tz"
        :from="grid.from"
        :days="days"
        :members="likes"
        hide-empty
        selectable
        @select="detail = $event"
      />
      <WeekGrid
        v-else
        :events="eventLikes"
        :members="likes"
        :chosen="chosen"
        :start="grid.from"
        :tz="tz"
        @select="detail = $event"
      />
    </template>

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
            <li v-if="list.length > 3">
              <button type="button" class="more" @click="dayOpen = k">
                +{{ list.length - 3 }} more
              </button>
            </li>
            <li v-for="c in chipsOf(k)" :key="c.id" class="task-chip" :style="trimStyle(c.color)">
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

    <UiSheet v-model="detailOpen" :title="sheetTitle">
      <template v-if="detail">
        <UiButton v-if="dayOpen" class="back" variant="text" icon="back" @click="detail = null">
          Back to day
        </UiButton>
        <EventCard :event="detail" :tz="tz" :members="likes" />
      </template>
      <div v-else class="day-list">
        <EventCard
          v-for="e in dayEvents"
          :key="e.id"
          :event="e"
          :tz="tz"
          :members="likes"
          selectable
          @select="detail = $event"
        />
      </div>
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
  border-left: 4px solid transparent;
  --ev-stripe-w: 4px;
  padding-left: var(--ui-space-2);
  border-radius: var(--ui-radius-sm);
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
  --ev-stripe-w: 4px;
  background-color: var(--ui-surface-container-low);
  border: 0;
  border-left: 4px solid transparent;
  border-radius: var(--ui-radius-sm);
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
  --ev-bg: repeating-linear-gradient(
    45deg,
    transparent,
    transparent 4px,
    var(--ui-outline-variant) 4px,
    var(--ui-outline-variant) 6px
  );
  background-image: var(--ev-bg);
  color: var(--ui-on-surface-variant);
}
.day-list {
  display: grid;
  gap: var(--ui-space-2);
}
.back {
  margin-bottom: var(--ui-space-2);
}
.more {
  background: none;
  border: 0;
  padding: 0;
  cursor: pointer;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
</style>
