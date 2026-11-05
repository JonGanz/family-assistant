<script setup lang="ts">
import { computed } from 'vue'
import { icons, type IconName } from './icons'

const props = withDefaults(defineProps<{ name: IconName; size?: 20 | 24; label?: string }>(), {
  size: 24,
})
const component = computed(() => icons[props.name])
</script>

<template>
  <component
    :is="component"
    class="ui-icon"
    :width="size"
    :height="size"
    :aria-hidden="label ? undefined : 'true'"
    :aria-label="label"
    :role="label ? 'img' : undefined"
    focusable="false"
  />
</template>

<style scoped>
.ui-icon {
  flex: none;
  fill: currentColor;
  color: inherit;
}
</style>
