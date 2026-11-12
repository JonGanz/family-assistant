<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { dayKey } from '../../lib/time'
import {
  fireConfetti,
  hasFired,
  markFired,
  memberShades,
  newlyCompleted,
  runKey,
} from '../effects/confettiModel'
import ProgressBar from '../../components/tasks/ProgressBar.vue'
import { shownRuns } from '../tasksModel'
import type { PanelProps } from '../types'

const props = defineProps<PanelProps>()
const emit = defineEmits<{ tick: [run: string, step: string, done: boolean] }>()
const readOnly = computed(() => props.data.completion_enabled === false)
const runs = computed(() =>
  shownRuns(props.data, props.panel, props.now).map((r) => {
    const m = props.data.members.find((x) => x.id === r.member)
    return {
      run: r,
      name: m?.display_name ?? '',
      color: m?.color ?? null,
      done: r.steps.length > 0 && r.steps.every((s) => s.done_at),
    }
  }),
)

/**
 * Confetti when a run turns done (once per run and day, see confettiModel). Runs already done
 * when first seen never fire. Each device can switch it off (`settings.confetti`).
 */
const cards = ref<HTMLElement[]>([])
const prev = new Map<string, boolean>()
watch(
  runs,
  (list) => {
    const states = list.map((x) => ({
      key: runKey(x.run),
      runDate: x.run.run_date,
      done: x.done,
    }))
    const fresh = newlyCompleted(prev, states, hasFired)
    if (props.settings.confetti === false) return
    for (const f of fresh) {
      const x = list.find((y) => runKey(y.run) === f.key)
      const el = cards.value.find((c) => c.dataset.run === x?.run.id)
      markFired(f.key, f.runDate, dayKey(props.now, props.data.household.tz))
      if (el) fireConfetti({ rect: el.getBoundingClientRect(), colors: memberShades(x?.color) })
    }
  },
  { immediate: true, flush: 'post' },
)
</script>

<template>
  <section v-if="runs.length" class="routine" aria-label="Routine">
    <div
      v-for="x in runs"
      ref="cards"
      :key="x.run.id"
      class="col"
      :class="{ complete: x.done }"
      :data-run="x.run.id"
    >
      <h2>
        {{ x.run.routine_name }}<span v-if="x.name" class="who"> · {{ x.name }}</span>
      </h2>
      <ProgressBar
        :done="x.run.steps.filter((s) => s.done_at).length"
        :total="x.run.steps.length"
        :label="x.run.routine_name"
        size="kiosk"
      />
      <ul>
        <li v-for="st in x.run.steps" :key="st.step">
          <button
            type="button"
            class="step"
            :class="{ on: !!st.done_at }"
            :aria-pressed="!!st.done_at"
            :disabled="readOnly"
            @click="emit('tick', x.run.id, st.step, !st.done_at)"
          >
            <span class="box" aria-hidden="true">{{ st.done_at ? '✓' : '' }}</span>
            <span class="text">{{ st.text }}</span>
          </button>
        </li>
      </ul>
    </div>
  </section>
</template>

<style scoped>
.routine {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: minmax(0, 1fr);
  gap: var(--space-3);
  align-content: start;
}
.col {
  display: grid;
  gap: var(--space-2);
  align-content: start;
  min-width: 0;
  padding: var(--space-3);
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}
.col.complete {
  box-shadow: inset 0 0 0 3px var(--color-success);
}
h2 {
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: 600;
}
.who {
  color: var(--color-muted);
  font-weight: 400;
}
ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--space-2);
}
.step {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  width: 100%;
  min-height: 5rem;
  padding: var(--space-2) var(--space-4);
  background: var(--color-bg);
  border: 2px solid var(--color-border);
  border-radius: var(--radius-md);
  color: inherit;
  font: inherit;
  font-size: var(--kiosk-font-base);
  text-align: left;
  cursor: pointer;
}
.step.on {
  border-color: var(--color-success);
  color: var(--color-muted);
}
.step.on .text {
  text-decoration: line-through;
}
.step:disabled {
  cursor: default;
}
.box {
  flex: none;
  display: grid;
  place-items: center;
  position: relative;
  width: 2rem;
  height: 2rem;
  border: 2px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-weight: 700;
}
/* Invisible, wider touch target around the smaller visible box. */
.box::before {
  content: '';
  position: absolute;
  inset: -1.25rem;
}
.on .box {
  background: var(--color-success);
  border-color: var(--color-success);
  color: var(--color-on-accent);
}
@media (prefers-reduced-motion: no-preference) {
  .step {
    transition: border-color 0.3s ease;
  }
}
</style>

<style>
/* Portrait: the routines band runs the full width at the bottom, so routines sit two side by side. */
[data-orientation='portrait'] .routine.routine {
  grid-auto-flow: row;
  grid-auto-columns: auto;
  grid-template-columns: repeat(2, minmax(0, 1fr));
}
</style>
