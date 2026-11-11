<script setup lang="ts">
// Page-level tabs (Frontend.md §3f, UI kit): UiSegmented on compact windows, a tab row otherwise. An option with
// `href` renders as a link (route-based tabs, the parent routes on `update:modelValue`); without it, as a tab button.
import UiSegmented from './UiSegmented.vue'
import { useBreakpoint } from './composables/useBreakpoint'

export interface TabOption {
  value: string
  label: string
  href?: string
}
const props = defineProps<{
  modelValue: string
  options: TabOption[]
  label: string
  /** Optional: each expanded tab gets `data-testid="<prefix>-<value>"`. */
  testidPrefix?: string
}>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
const bp = useBreakpoint()

function tid(value: string) {
  return props.testidPrefix ? `${props.testidPrefix}-${value}` : undefined
}
</script>

<template>
  <div class="ui-tabs">
    <UiSegmented
      v-if="bp === 'compact'"
      class="ui-tabs__seg"
      :label="label"
      :model-value="modelValue"
      :options="options.map((o) => ({ value: o.value, label: o.label }))"
      @update:model-value="emit('update:modelValue', $event)"
    />
    <nav v-else-if="options.every((o) => o.href)" class="ui-tabs__row" :aria-label="label">
      <a
        v-for="o in options"
        :key="o.value"
        :href="o.href"
        class="ui-tabs__tab"
        :class="{ 'is-on': o.value === modelValue }"
        :aria-current="o.value === modelValue ? 'page' : undefined"
        :data-testid="tid(o.value)"
        @click.prevent="emit('update:modelValue', o.value)"
      >
        {{ o.label }}
      </a>
    </nav>
    <div v-else class="ui-tabs__row" role="tablist" :aria-label="label">
      <button
        v-for="o in options"
        :key="o.value"
        type="button"
        role="tab"
        class="ui-tabs__tab"
        :class="{ 'is-on': o.value === modelValue }"
        :aria-selected="o.value === modelValue"
        :data-testid="tid(o.value)"
        @click="emit('update:modelValue', o.value)"
      >
        {{ o.label }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.ui-tabs__seg {
  display: flex;
  width: 100%;
}
.ui-tabs__row {
  display: flex;
  gap: var(--ui-space-1);
  border-bottom: 1px solid var(--ui-outline-variant);
}
.ui-tabs__tab {
  display: inline-flex;
  align-items: center;
  min-height: var(--ui-touch);
  padding: 0 var(--ui-space-4);
  border: 0;
  border-bottom: 3px solid transparent;
  background: none;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-label-lg);
  text-decoration: none;
  cursor: pointer;
}
.ui-tabs__tab:hover {
  background: var(--ui-state-hover);
}
.ui-tabs__tab:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: -3px;
}
.ui-tabs__tab.is-on {
  border-bottom-color: var(--ui-primary);
  color: var(--ui-primary);
}
</style>
