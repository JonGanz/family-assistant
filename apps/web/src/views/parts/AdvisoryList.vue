<script setup lang="ts">
import type { AdvisoryView } from '../../api/client'
import { UiCard, UiIcon } from '../../ui'

defineProps<{ advisories: AdvisoryView[] }>()
import { trimStyle } from '../../lib/eventColors'
const trimColor = (s: string) =>
  s === 'severe' ? 'var(--ui-error)' : s === 'warning' ? 'var(--ui-warning)' : 'var(--ui-primary)'
const iconFor = (s: string) => (s === 'severe' ? 'error' : s === 'warning' ? 'warning' : 'info')
</script>

<template>
  <ul v-if="advisories.length" class="advisories" aria-label="Advisories">
    <li v-for="a in advisories" :key="a.id">
      <UiCard
        variant="filled"
        class="advisory"
        :class="a.severity"
        :style="trimStyle(trimColor(a.severity))"
      >
        <div class="row">
          <UiIcon :name="iconFor(a.severity)" :size="24" />
          <span>{{ a.text }}</span>
        </div>
      </UiCard>
    </li>
  </ul>
</template>

<style scoped>
.advisories {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: var(--ui-space-2);
}
.advisory {
  background: var(--ui-primary-container);
  color: var(--ui-on-primary-container);
}
.advisory.warning {
  background: color-mix(in srgb, var(--ui-warning) 18%, var(--ui-surface));
  color: var(--ui-on-surface);
  border-left: 6px solid transparent;
  --ev-stripe-w: 6px;
}
.advisory.severe {
  background: var(--ui-error-container);
  color: var(--ui-on-error-container);
  border-left: 6px solid transparent;
  --ev-stripe-w: 6px;
}
.row {
  display: flex;
  align-items: center;
  gap: var(--ui-space-3);
  font: var(--ui-type-body-lg);
}
</style>
