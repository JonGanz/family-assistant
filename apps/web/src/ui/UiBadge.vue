<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{ count?: number; max?: number; dot?: boolean }>(), {
  max: 99,
})
const text = computed(() =>
  props.count && props.count > props.max ? `${props.max}+` : String(props.count ?? ''),
)
</script>

<template>
  <span
    v-if="dot || (count !== undefined && count > 0)"
    class="ui-badge"
    :class="{ 'ui-badge--dot': dot }"
    :aria-label="dot ? undefined : `${count} new`"
    >{{ dot ? '' : text }}</span
  >
</template>

<style scoped>
.ui-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  border-radius: var(--ui-radius-full);
  background: var(--ui-error);
  color: var(--ui-on-error);
  font: var(--ui-type-label-sm);
}
.ui-badge--dot {
  min-width: 8px;
  width: 8px;
  height: 8px;
  padding: 0;
}
</style>
