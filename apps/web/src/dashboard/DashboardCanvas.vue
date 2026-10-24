<script setup lang="ts">
import { computed } from 'vue'
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
defineEmits<{ dismiss: [id: string]; pick: [id: string] }>()

const cfg = computed(() => props.settings ?? props.data.settings)
const areas = computed(() => resolveLayout(cfg.value))
const rows = computed(() =>
  Math.max(1, ...areas.value.map((a) => Number(a.row.split('/')[1]?.trim() ?? '2') - 1)),
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
        gridTemplateRows: `repeat(${rows}, auto)`,
        transform: `translate(${shift.x}px, ${shift.y}px)`,
      }"
    >
      <div
        v-for="(a, i) in areas"
        :key="i"
        class="cell"
        :class="[`k-${a.panel.kind}`, { fill: landscape && a.column === '5 / 13' }]"
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
  align-content: start;
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
