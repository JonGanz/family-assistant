<script setup lang="ts">
import { computed, inject, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import type { EventView } from '../api/client'
import DateRangeNav from '../components/DateRangeNav.vue'
import DayListSheet, { type DayOpen } from '../components/calendar/DayListSheet.vue'
import AgendaScroller from '../components/calendar/AgendaScroller.vue'
import WeekGrid from '../components/calendar/WeekGrid.vue'
import { rangeFor, shiftAnchor } from '../composables/useRange'
import { weekStartKey, type WeekStart } from '../dashboard/weekModel'
import { capacity, visibleCount } from '../components/calendar/monthGrid'
import { stripeStyle } from '../lib/eventColors'
import { addDays, dayKey, formatDayHeading, groupByDay } from '../lib/time'
import type { EventLike, RangeMode } from '../lib/types'
import { useCalendarStore } from '../stores/calendar'
import { useHouseholdStore } from '../stores/household'
import { useMembersStore } from '../stores/members'
import {
  UiFilterChips,
  UiIconButton,
  UiErrorState,
  UiPage,
  UiSkeleton,
  UiTabs,
  useBreakpoint,
} from '../ui'
import { navControlKey } from '../ui/nav'
import { memberLikes, toEventLike } from './shared'

type View = 'agenda' | 'week' | 'month'

const household = useHouseholdStore()
const calendar = useCalendarStore()
const members = useMembersStore()
const bp = useBreakpoint()
// The page has no title bar (the menu shows where you are); on a phone its menu button lives here.
const navControl = inject(navControlKey, null)

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
const dayOpen = ref<DayOpen | null>(null)

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
// Agenda: an endless list from `agendaStart`; the arrows jump its start a week, Today resets it.
const agendaStart = ref(dayKey(new Date(), household.tz))
const agendaNonce = ref(0)
function nav(dir: -1 | 0 | 1) {
  if (view.value === 'agenda') {
    agendaStart.value = dir ? addDays(agendaStart.value, dir * 7) : dayKey(new Date(), tz.value)
    agendaNonce.value++
    return
  }
  anchor.value = dir ? shiftAnchor(navMode.value, anchor.value, dir) : dayKey(new Date(), tz.value)
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
const loadingFirstRaw = computed(() => calendar.loading && events.value.length === 0)
const today = computed(() => dayKey(new Date(), tz.value))
const label = computed(() => {
  if (view.value === 'agenda')
    return `From ${formatDayHeading(agendaStart.value, tz.value, today.value)}`
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

function load() {
  if (view.value === 'agenda') return // the Agenda loads its own chunks
  for (const m of memberKeys.value)
    void calendar.ensureRange(grid.value.from, grid.value.to, tz.value, m)
}
// Month: the grid fills the space below it, the week rows share the height equally; each cell's capacity comes from its measured height.
const FALLBACK_CAP = 3
const monthEl = ref<HTMLElement | null>(null)
const cap = ref(FALLBACK_CAP)
const weeks = computed(() => Math.max(1, Math.round(days.value / 7)))
let ro: ResizeObserver | null = null
function measure() {
  const el = monthEl.value
  if (!el) return
  const rootPx = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16
  // The grid fills the viewport from its top edge down.
  const avail = Math.floor(window.innerHeight - el.getBoundingClientRect().top - rootPx)
  let h = Math.max(avail, 16 * rootPx)
  el.style.height = `${h}px`
  // Whatever still pushes the page past the screen (its bottom padding, a safe area) comes off too.
  const over = document.documentElement.scrollHeight - window.innerHeight
  if (over > 0 && h - over >= 16 * rootPx) {
    h -= over
    el.style.height = `${h}px`
  }
  const cell = el.querySelector<HTMLElement>('.cell')
  const num = el.querySelector<HTMLElement>('.num')
  if (!cell || cell.clientHeight === 0) return
  const pad = parseFloat(getComputedStyle(cell).paddingTop) * 2 || 0
  // Item height (the CSS --month-line, in rem) plus the 1px list gap.
  const raw = getComputedStyle(el).getPropertyValue('--month-line').trim()
  const lineRem = raw.endsWith('rem') ? parseFloat(raw) : NaN
  const line = (Number.isFinite(lineRem) ? lineRem : 0.85) * rootPx + 1
  cap.value = capacity(cell.clientHeight - pad, line, (num?.offsetHeight ?? 0) + 1)
}
function observe() {
  ro?.disconnect()
  if (typeof ResizeObserver === 'undefined' || !monthEl.value) return
  ro = new ResizeObserver(measure)
  ro.observe(monthEl.value)
  ro.observe(document.body)
}
watch(
  () => [view.value, weeks.value, calendar.error, loadingFirstRaw.value],
  async () => {
    await nextTick()
    measure()
    observe()
  },
)
onMounted(() => {
  void members.load()
  load()
  window.addEventListener('resize', measure)
  void nextTick(() => {
    measure()
    observe()
  })
})
onBeforeUnmount(() => {
  window.removeEventListener('resize', measure)
  ro?.disconnect()
})
watch([() => grid.value.from, () => grid.value.to, chosen, tz, view], load)

const dayNumber = (k: string) => Number(k.slice(8))
const loadingFirst = loadingFirstRaw
</script>

<template>
  <UiPage wide>
    <div class="top-row">
      <UiIconButton
        v-if="navControl && bp === 'compact'"
        icon="menu"
        label="Open navigation"
        @click="navControl.openNav()"
      />
      <UiTabs
        :model-value="view"
        :options="VIEWS"
        label="View"
        @update:model-value="setView($event as View)"
      />
    </div>

    <div class="controls">
      <DateRangeNav :label="label" @prev="nav(-1)" @next="nav(1)" @today="nav(0)" />
      <UiFilterChips v-model="chosen" :options="memberOptions" label="Member" />
    </div>

    <UiErrorState v-if="calendar.error" :message="calendar.error" @retry="load" />
    <UiSkeleton v-else-if="loadingFirst && view !== 'agenda'" :lines="5" />

    <template v-else-if="view === 'agenda' || view === 'week'">
      <AgendaScroller
        v-if="view === 'agenda'"
        :tz="tz"
        :start="agendaStart"
        :nonce="agendaNonce"
        :members="likes"
        :member-keys="memberKeys"
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
        @more="dayOpen = $event"
      />
    </template>

    <div v-else class="month-wrap">
      <div
        ref="monthEl"
        class="month"
        :style="{
          gridTemplateRows: `auto repeat(${weeks}, minmax(0, 1fr))`,
        }"
      >
        <div v-for="w in weekdays" :key="w" class="dow">{{ w }}</div>
        <div
          v-for="[k, list] in byDay"
          :key="k"
          class="cell"
          :class="{ today: k === today, outside: k < span.from || k >= span.to }"
        >
          <div class="num">{{ dayNumber(k) }}</div>
          <ul>
            <li v-for="e in list.slice(0, visibleCount(list.length, cap))" :key="e.id">
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
            <li v-if="list.length > visibleCount(list.length, cap)">
              <button type="button" class="more" @click="dayOpen = { day: k, events: list }">
                +{{ list.length - visibleCount(list.length, cap) }}
              </button>
            </li>
          </ul>
        </div>
      </div>
    </div>

    <DayListSheet
      v-model:day="dayOpen"
      v-model:detail="detail"
      :tz="tz"
      :members="likes"
      :today="today"
    />
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
.top-row {
  display: flex;
  align-items: center;
  gap: var(--ui-space-1);
}
.top-row > :first-child:not(:last-child) {
  flex: none;
}
.top-row > :last-child {
  flex: 1;
  min-width: 0;
}
.month {
  /* A day number and four one-line events. */
  --month-font: 0.75rem;
  --month-line: 0.85rem;
  display: grid;
  overflow: hidden;
  grid-template-columns: repeat(7, minmax(0, 1fr));
  gap: 1px;
  background: var(--ui-outline-variant);
  border: 1px solid var(--ui-outline-variant);
  border-radius: var(--ui-radius-md);
}
.dow {
  background: var(--ui-surface-container-low);
  padding: var(--ui-space-1) var(--ui-space-2);
  font: var(--ui-type-label-md);
  text-align: center;
}
.cell {
  background: var(--ui-surface);
  padding: 1px;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
.cell.outside {
  background: var(--ui-surface-container-low);
  color: var(--ui-on-surface-variant);
}
/* Today is inverted: the whole cell, events included (their colour stripe stays). */
.cell.today {
  background: var(--ui-on-surface);
  color: var(--ui-surface);
}
.cell.today .num {
  font-weight: 700;
}
.cell.today .ev.busy {
  --ev-bg: repeating-linear-gradient(
    45deg,
    transparent,
    transparent 4px,
    color-mix(in srgb, var(--ui-surface) 35%, transparent) 4px,
    color-mix(in srgb, var(--ui-surface) 35%, transparent) 6px
  );
  color: inherit;
}
.cell.today .more {
  color: inherit;
}
.num {
  display: inline-block;
  font: 500 var(--month-font) / var(--month-line) var(--font-sans);
  min-width: 1.4em;
  text-align: center;
  padding: 0 3px;
}
.cell ul {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: 1px;
  min-width: 0;
}
.cell li {
  min-width: 0;
}
.ev {
  display: block;
  width: 100%;
  text-align: left;
  font: 500 var(--month-font) / var(--month-line) var(--font-sans);
  color: inherit;
  --ev-stripe-w: 3px;
  background: none;
  border: 0;
  border-left: 3px solid transparent;
  border-radius: var(--ui-radius-sm);
  height: var(--month-line);
  min-height: 0;
  box-sizing: border-box;
  line-height: var(--month-line);
  padding: 0 3px;
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
.more {
  background: none;
  border: 0;
  padding: 0;
  cursor: pointer;
  color: var(--ui-on-surface-variant);
  font: 500 var(--month-font) / var(--month-line) var(--font-sans);
  height: var(--month-line);
  min-height: 0;
  padding: 0 3px;
}
@media (max-width: 599px) {
  .month {
    --month-font: 0.625rem;
    --month-line: 0.7rem;
  }
  .dow {
    padding: 0 1px;
    font-size: 0.625rem;
  }
  .num {
    min-width: 0;
    padding: 0 1px;
  }
  .cell {
    padding: 0;
  }
  .ev {
    padding: 0 2px;
    border-left-width: 2px;
  }
}
</style>
