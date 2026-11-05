<script setup lang="ts">
import { computed } from 'vue'
import { textOn } from '../lib/hues'

/** `kiosk` (default) keeps the square wall-display look; the app passes `variant="app"` (round). */
const props = withDefaults(
  defineProps<{
    name: string
    color: string
    src?: string | null
    size?: string
    variant?: 'kiosk' | 'app'
  }>(),
  { variant: 'kiosk' },
)
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
    :class="{ 'avatar--app': variant === 'app' }"
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
  border-radius: 0;
  overflow: hidden;
  vertical-align: middle;
}
.avatar--app {
  border-width: 2px;
  border-radius: var(--ui-radius-full);
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
