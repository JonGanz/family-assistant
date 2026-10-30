<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watchEffect } from 'vue'
import { applyTheme, loadDeviceChoice, resolveVariant } from '../themes'
import { panelVisible } from './tasksModel'
import { burnInOffset, normalizeTheme, resolveLayout } from './layout'
import PanelHost from './panels/PanelHost.vue'
import { prefersDark, resolveMode } from './theme'
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
const rows = computed(() =>
  Math.max(1, ...areas.value.map((a) => Number(a.row.split('/')[1]?.trim() ?? '2') - 1)),
)
/** Portrait rows share the height by weight, so a long list scrolls inside its panel. */
const WEIGHT: Record<string, number> = { clock: 1, weather: 2, agenda: 4, now: 5 }
const portraitRows = computed(() =>
  areas.value.map((a) => `minmax(0, ${WEIGHT[a.panel.kind] ?? 4}fr)`).join(' '),
)
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
const landscape = computed(() => cfg.value.orientation === 'landscape')
</script>

<template>
  <div
    ref="root"
    class="kiosk canvas"
    :class="cfg.orientation"
    :data-theme="variant.mode"
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
</style>
