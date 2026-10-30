<script setup lang="ts">
import { computed } from 'vue'
import { textOn } from '../lib/hues'

const props = defineProps<{ name: string; color: string; src?: string | null; size?: string }>()
const initials = computed(() =>
  props.name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => [...w][0]!.toUpperCase())
    .join(''),
)
const fg = computed(() => textOn(props.color))
</script>

<template>
  <span
    class="avatar"
    :style="{ borderColor: color, width: size ?? '2.5em', height: size ?? '2.5em' }"
    :title="name"
  >
    <img v-if="src" :src="src" :alt="name" />
    <span v-else class="initials" :style="{ background: color, color: fg }" aria-hidden="true">{{
      initials
    }}</span>
  </span>
</template>

<style scoped>
.avatar {
  display: inline-block;
  border: 3px solid;
  border-radius: 50%;
  overflow: hidden;
  vertical-align: middle;
}
.avatar img,
.initials {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
}
</style>
