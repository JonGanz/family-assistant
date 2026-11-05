<script setup lang="ts">
import { computed, ref } from 'vue'
import { UiChip, UiSheet, useBreakpoint } from '../../ui'

const props = defineProps<{ confidence?: number | null }>()

const bp = useBreakpoint()
const compact = computed(() => bp.value === 'compact')
const pct = computed(() =>
  props.confidence == null ? null : Math.round(Math.min(1, Math.max(0, props.confidence)) * 100),
)
const label = computed(() => (pct.value == null ? 'AI' : `AI · ${pct.value}%`))
const sentence = computed(() =>
  pct.value == null
    ? 'Suggested by the local language model'
    : `Suggested by the local language model, ${pct.value}% confident`,
)
const sheet = ref(false)
function tap() {
  if (compact.value) sheet.value = true
}
</script>

<template>
  <span class="ai-chip" :aria-label="sentence" role="group">
    <UiChip icon="ai" @click="tap">{{ label }}</UiChip>
    <span v-if="!compact" class="tip" role="tooltip">{{ sentence }}</span>
    <UiSheet v-model="sheet" title="AI suggestion">
      <p>{{ sentence }}</p>
    </UiSheet>
  </span>
</template>

<style scoped>
.ai-chip {
  position: relative;
  display: inline-flex;
}
.ai-chip :deep(.ui-chip) {
  background: var(--ui-primary-container);
  color: var(--ui-on-primary-container);
  border-color: transparent;
}
.tip {
  position: absolute;
  top: 100%;
  right: 0;
  z-index: 2;
  margin-top: var(--ui-space-1);
  padding: var(--ui-space-1) var(--ui-space-2);
  border-radius: var(--ui-radius-sm, 4px);
  background: var(--ui-inverse-surface);
  color: var(--ui-inverse-on-surface);
  font: var(--ui-type-body-sm, var(--ui-type-body-md));
  white-space: nowrap;
  visibility: hidden;
  pointer-events: none;
}
.ai-chip:hover .tip,
.ai-chip:focus-within .tip {
  visibility: visible;
}
</style>
