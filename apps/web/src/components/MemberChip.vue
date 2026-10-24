<script setup lang="ts">
import { computed } from 'vue'
import { textColorFor } from '../lib/contrast'

const props = defineProps<{ name: string; color: string }>()
const fg = computed(() => textColorFor(props.color))
</script>

<template>
  <span class="member-chip" :style="{ background: color, color: fg }">{{ name }}</span>
</template>

<style scoped>
.member-chip {
  display: inline-block;
  padding: 0 var(--space-2);
  border-radius: 999px;
  font-size: var(--font-size-sm);
  line-height: 1.6;
  white-space: nowrap;
}
</style>
