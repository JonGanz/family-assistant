<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch, watchEffect } from 'vue'
import { dayKey } from '../lib/time'
import { applyTheme, loadDeviceChoice, resolveVariant } from '../themes'
import { panelVisible } from './tasksModel'
import { burnInOffset, normalizeTheme, resolveLayout } from './layout'
import PanelHost from './panels/PanelHost.vue'
import { prefersDark, resolveMode } from './theme'
import { S } from './strings'
import { weekLabel, weekStartKey, type WeekStart } from './weekModel'
import EventDetail from './details/EventDetail.vue'
import DayDetail from './details/DayDetail.vue'
import WeatherDetail from './details/WeatherDetail.vue'
import type { DashboardData, DashboardSettings, DetailRequest } from './types'

const props = withDefaults(
  defineProps<{
    data: DashboardData
    /** Settings to render; defaults to the snapshot's own. The layout editor passes edited ones. */
    settings?: DashboardSettings
    now: Date
    burnIn?: boolean
    /** Events and tasks of the week the Week view asked for (see the `week` event). */
    weekData?: { from: string; events: DashboardData['events']; tasks: DashboardData['tasks'] }
    /** True on the real /dashboard route: the theme is also applied to the page root. */
    page?: boolean
  }>(),
  { settings: undefined, burnIn: false, page: false, weekData: undefined },
)
const emit = defineEmits<{
  /** The first day of the week the Week view shows, or null when it is not showing. */
  week: [from: string | null]
  pick: [id: string]
  complete: [id: string]
  tick: [run: string, step: string, done: boolean]
  add: []
}>()

const cfg = computed(() => props.settings ?? props.data.settings)
/** Panels with nothing to show (an idle routine) are dropped so the others use the space. */
const areas = computed(() =>
  resolveLayout({
    ...cfg.value,
    theme: normalizeTheme(cfg.value.theme),
    panels: cfg.value.panels.filter((p) => panelVisible(p, props.data, props.now)),
  }),
)
const landscape = computed(() => cfg.value.orientation === 'landscape')
/** Landscape: a left and a right stack. Portrait: one. Each panel takes the height it needs. */
const stacks = computed(() => {
  if (!landscape.value) return [{ key: 'all', span: '1 / 13', areas: areas.value }]
  const left = areas.value.filter((a) => a.column === '1 / 5')
  const right = areas.value.filter((a) => a.column !== '1 / 5')
  return [
    { key: 'left', span: '1 / 5', areas: left },
    { key: 'right', span: '5 / 13', areas: right },
  ].filter((s) => s.areas.length > 0)
})
type View = 'home' | 'week'
const VIEWS: { id: View; label: string }[] = [
  { id: 'home', label: S.navHome },
  { id: 'week', label: S.navWeek },
]
const view = ref<View>('home')
function setView(v: View) {
  detail.value = null
  view.value = v
}
const detail = ref<DetailRequest | null>(null)
const weekOffset = ref(0)
const hidePast = ref(false)
const tz = computed(() => props.data.household.tz)
const weekStart = computed<WeekStart>(() =>
  (props.data.household as { week_start?: string }).week_start === 'sunday' ? 'sunday' : 'monday',
)
const today = computed(() => dayKey(props.now, tz.value))
const shownStart = computed(() => weekStartKey(today.value, weekStart.value, weekOffset.value))
const weekText = computed(() =>
  weekOffset.value === 0 ? S.currentWeek : weekLabel(shownStart.value),
)
const weekPanel = computed(() => ({
  kind: 'week_table',
  start: shownStart.value,
  hidePast: hidePast.value,
}))
/** The fetched week's data; until it arrives the snapshot stands in for the current week. */
const weekView = computed<DashboardData>(() => {
  const w = props.weekData
  if (w && w.from === shownStart.value) return { ...props.data, events: w.events, tasks: w.tasks }
  return weekOffset.value === 0 ? props.data : { ...props.data, events: [] }
})
watch([view, shownStart], () => emit('week', view.value === 'week' ? shownStart.value : null), {
  immediate: true,
})
const choice = computed(() => normalizeTheme(cfg.value.theme))
/** `night` follows sunrise/sunset from the snapshot; without them the browser scheme decides. */
const variant = computed(() => {
  const dark = prefersDark()
  const night =
    resolveMode(
      'auto',
      props.now,
      props.data.weather?.sunrise_today,
      props.data.weather?.sunset_today,
      dark,
    ) === 'dark'
  return resolveVariant(choice.value, { prefersDark: dark, night })
})
const root = ref<HTMLElement | null>(null)
watchEffect(
  () => {
    if (root.value) applyTheme(variant.value, root.value)
    if (props.page) applyTheme(variant.value)
  },
  { flush: 'post' },
)
onBeforeUnmount(() => {
  // Leaving /dashboard: hand the page back to this device's own choice.
  if (props.page) applyTheme(resolveVariant(loadDeviceChoice(), { prefersDark: prefersDark() }))
})
const shift = computed(() =>
  props.burnIn && variant.value.mode === 'dark' ? burnInOffset(props.now) : { x: 0, y: 0 },
)
</script>

<template>
  <div
    ref="root"
    class="kiosk canvas"
    :class="cfg.orientation"
    :data-theme="variant.mode"
    :data-orientation="cfg.orientation"
  >
    <nav class="nav" :aria-label="S.navLabel">
      <button
        v-for="v in VIEWS"
        :key="v.id"
        type="button"
        :class="{ on: view === v.id }"
        :aria-current="view === v.id ? 'page' : undefined"
        :data-view="v.id"
        @click="setView(v.id)"
      >
        {{ v.label }}
      </button>
      <div class="navend"><slot name="nav-end" /></div>
    </nav>
    <main class="main" :class="{ full: view === 'week' }">
      <div class="grid" :style="{ transform: `translate(${shift.x}px, ${shift.y}px)` }">
        <template v-if="view === 'home'">
          <div v-for="st in stacks" :key="st.key" class="stack" :style="{ gridColumn: st.span }">
            <div v-for="(a, i) in st.areas" :key="i" class="cell" :class="`k-${a.panel.kind}`">
              <PanelHost
                :data="data"
                :settings="cfg"
                :panel="a.panel"
                :now="now"
                @pick="(id: string) => $emit('pick', id)"
                @complete="(id: string) => $emit('complete', id)"
                @tick="(r: string, st: string, d: boolean) => $emit('tick', r, st, d)"
                @add="$emit('add')"
                @detail="(d: DetailRequest) => (detail = d)"
              />
            </div>
          </div>
        </template>
        <div v-else class="stack weekview" style="grid-column: 1 / 13">
          <div class="cell k-week_table">
            <PanelHost
              :data="weekView"
              :settings="cfg"
              :panel="weekPanel"
              :now="now"
              @detail="(d: DetailRequest) => (detail = d)"
            />
          </div>
          <div class="weekbar" role="group" :aria-label="S.weekNav">
            <button type="button" :aria-label="S.prevWeek" @click="weekOffset -= 1">‹</button>
            <output class="weektext" data-testid="week-label">{{ weekText }}</output>
            <button type="button" :aria-label="S.nextWeek" @click="weekOffset += 1">›</button>
            <label class="hidepast">
              <input v-model="hidePast" type="checkbox" role="switch" />
              {{ S.hidePast }}
            </label>
          </div>
        </div>
      </div>
    </main>
    <WeatherDetail
      v-if="detail?.kind === 'weather'"
      :data="data"
      :now="now"
      @close="detail = null"
    />
    <DayDetail
      v-else-if="detail?.kind === 'day'"
      :data="weekView"
      :title="detail.title"
      :color="detail.color"
      :events="detail.events"
      :tasks="detail.tasks"
      @close="detail = null"
      @event="(id: string) => (detail = { kind: 'event', id })"
    />
    <EventDetail
      v-else-if="detail?.kind === 'event'"
      :id="detail.id"
      :data="view === 'week' ? weekView : data"
      @close="detail = null"
    />
    <slot />
  </div>
</template>

<style scoped>
.canvas {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: var(--color-bg);
  color: var(--color-text);
  font-family: var(--font-sans);
  line-height: 1.3;
}
.canvas {
  display: flex;
}
.canvas.portrait {
  flex-direction: column;
}
.nav {
  flex: none;
  display: flex;
  gap: var(--space-2);
  padding: var(--space-2);
  background: var(--color-surface);
  border-color: var(--color-border);
}
.landscape .nav {
  flex-direction: column;
  width: 7rem;
  border-right: 1px solid var(--color-border);
}
.portrait .nav {
  border-bottom: 1px solid var(--color-border);
}
.navend {
  margin-top: auto;
  display: grid;
  gap: var(--space-2);
  font-size: var(--font-size-xs);
  color: var(--color-muted);
  overflow-wrap: anywhere;
}
.portrait .navend {
  margin: 0 0 0 auto;
  grid-auto-flow: column;
  align-items: center;
}
.nav button {
  flex: 1;
  min-height: var(--min-touch);
  padding: var(--space-2) var(--space-3);
  border: 2px solid var(--color-border);
  background: var(--color-bg);
  color: inherit;
  font: inherit;
  font-size: var(--font-size-lg);
  cursor: pointer;
}
.landscape .nav button {
  flex: none;
  min-height: 5rem;
}
.nav button.on {
  background: var(--color-accent);
  border-color: var(--color-accent);
  color: var(--color-on-accent);
  font-weight: 600;
}
/* The whole view scrolls, so no panel is cropped when another needs more room. */
.main {
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow-y: auto;
  padding-bottom: 2.5rem;
}
.main.full {
  overflow: hidden;
  padding-bottom: 0;
  display: flex;
}
/* The Week view is exactly as tall as the area beside the menu: the grid's own padding gives the
   same gap at the bottom as at the top, the table takes what the bar leaves. Nothing scrolls: a cell folds what does not fit into "+N". */
.main.full .grid {
  flex: 1;
  /* Stretch, not start: the table is absolutely positioned, so the stack has no content height. */
  align-items: stretch;
  box-sizing: border-box;
  grid-template-rows: minmax(0, 1fr);
  min-height: 0;
}
.weekview {
  grid-template-rows: minmax(0, 1fr) auto;
  min-height: 0;
}
.weekview .cell {
  position: relative;
  min-height: 0;
  overflow: hidden;
}
.weekbar {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}
.weekbar button {
  min-width: var(--min-touch);
  min-height: var(--min-touch);
  border: 2px solid var(--color-border);
  background: var(--color-surface);
  color: inherit;
  font: inherit;
  font-size: var(--font-size-lg);
  cursor: pointer;
}
.weektext {
  min-width: 10rem;
  padding: var(--space-2) var(--space-3);
  border: 2px solid var(--color-border);
  background: var(--color-bg);
  text-align: center;
  font-size: var(--font-size-base);
}
.hidepast {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-left: auto;
  font-size: var(--font-size-base);
}
.hidepast input {
  width: 1.5rem;
  height: 1.5rem;
}
.grid {
  display: grid;
  grid-template-columns: repeat(12, minmax(0, 1fr));
  gap: var(--space-4);
  padding: var(--space-4) var(--space-5);
  align-items: start;
}
.stack {
  display: grid;
  gap: var(--space-4);
  min-width: 0;
}
.cell {
  min-width: 0;
}
</style>
