<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watchEffect } from 'vue'
import { applyTheme, loadDeviceChoice, resolveVariant } from '../themes'
import { panelVisible } from './tasksModel'
import { burnInOffset, normalizeTheme, resolveLayout } from './layout'
import PanelHost from './panels/PanelHost.vue'
import { prefersDark, resolveMode } from './theme'
import { S } from './strings'
import type { DashboardData, DashboardSettings } from './types'

const props = withDefaults(
  defineProps<{
    data: DashboardData
    /** Settings to render; defaults to the snapshot's own. The layout editor passes edited ones. */
    settings?: DashboardSettings
    now: Date
    burnIn?: boolean
    /** True on the real /dashboard route: the theme is also applied to the page root. */
    page?: boolean
  }>(),
  { settings: undefined, burnIn: false, page: false },
)
defineEmits<{
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
const weekPanel = { kind: 'week_table' }
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
        @click="view = v.id"
      >
        {{ v.label }}
      </button>
    </nav>
    <main class="main">
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
              />
            </div>
          </div>
        </template>
        <div v-else class="stack" style="grid-column: 1 / 13">
          <div class="cell k-week_table">
            <PanelHost :data="data" :settings="cfg" :panel="weekPanel" :now="now" />
          </div>
        </div>
      </div>
    </main>
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
