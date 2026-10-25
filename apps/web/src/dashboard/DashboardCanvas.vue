<script setup lang="ts">
import { computed } from 'vue'
import { panelVisible } from './tasksModel'
import { burnInOffset, resolveLayout } from './layout'
import PanelHost from './panels/PanelHost.vue'
import { prefersDark, resolveTheme } from './theme'
import type { DashboardData, DashboardSettings } from './types'

const props = withDefaults(
  defineProps<{
    data: DashboardData
    /** Settings to render; defaults to the snapshot's own. The layout editor passes edited ones. */
    settings?: DashboardSettings
    now: Date
    burnIn?: boolean
  }>(),
  { settings: undefined, burnIn: false },
)
defineEmits<{
  dismiss: [id: string]
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
    panels: cfg.value.panels.filter((p) => panelVisible(p, props.data, props.now)),
  }),
)
const rows = computed(() =>
  Math.max(1, ...areas.value.map((a) => Number(a.row.split('/')[1]?.trim() ?? '2') - 1)),
)
/** Portrait rows share the height by weight, so a long list scrolls inside its panel. */
const WEIGHT: Record<string, number> = { clock: 1, weather: 2, advisories: 1, agenda: 4, now: 5 }
const portraitRows = computed(() =>
  areas.value.map((a) => `minmax(0, ${WEIGHT[a.panel.kind] ?? 4}fr)`).join(' '),
)
const theme = computed(() =>
  resolveTheme(
    cfg.value.theme,
    props.now,
    props.data.weather?.sunrise_today,
    props.data.weather?.sunset_today,
    prefersDark(),
  ),
)
const shift = computed(() =>
  props.burnIn && theme.value === 'dark' ? burnInOffset(props.now) : { x: 0, y: 0 },
)
const landscape = computed(() => cfg.value.orientation === 'landscape')
</script>

<template>
  <div
    class="kiosk canvas"
    :class="cfg.orientation"
    :data-theme="theme"
    :data-orientation="cfg.orientation"
  >
    <div
      class="grid"
      :style="{
        gridTemplateRows: landscape ? `repeat(${rows}, auto)` : portraitRows,
        transform: `translate(${shift.x}px, ${shift.y}px)`,
      }"
    >
      <div
        v-for="(a, i) in areas"
        :key="i"
        class="cell"
        :class="[`k-${a.panel.kind}`, { fill: !landscape || a.column === '5 / 13' }]"
        :style="{ gridColumn: a.column, gridRow: a.row }"
      >
        <div class="inner">
          <PanelHost
            :data="data"
            :settings="cfg"
            :panel="a.panel"
            :now="now"
            @dismiss="(id: string) => $emit('dismiss', id)"
            @pick="(id: string) => $emit('pick', id)"
            @complete="(id: string) => $emit('complete', id)"
            @tick="(r: string, st: string, d: boolean) => $emit('tick', r, st, d)"
            @add="$emit('add')"
          />
        </div>
      </div>
    </div>
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
.grid {
  display: grid;
  grid-template-columns: repeat(12, minmax(0, 1fr));
  gap: var(--space-4);
  height: 100%;
  padding: var(--space-4) var(--space-5);
  align-content: stretch;
}
.portrait .grid {
  align-content: stretch;
}
.cell {
  min-width: 0;
  min-height: 0;
}
.cell.fill {
  position: relative;
}
.cell.fill .inner {
  position: absolute;
  inset: 0;
  overflow: hidden;
}
.cell.fill .inner > :deep(*) {
  max-height: 100%;
  overflow: hidden;
}

/* Local theme tokens so the editor preview can differ from the page theme. */
.canvas[data-theme='dark'] {
  color-scheme: dark;
  --color-bg: #0f1218;
  --color-surface: #181d26;
  --color-text: #eef1f6;
  --color-muted: #9aa4b5;
  --color-border: #2a3140;
  --color-accent: #6b9bff;
  --color-danger: #ff8a80;
  --color-warning: #ffb74d;
}
.canvas[data-theme='light'] {
  color-scheme: light;
  --color-bg: #ffffff;
  --color-surface: #f4f6fa;
  --color-text: #1b1f27;
  --color-muted: #5b6472;
  --color-border: #d8dde6;
  --color-accent: #2f6fed;
  --color-danger: #c62828;
  --color-warning: #b26a00;
}
</style>
