<script setup lang="ts">
import { computed } from 'vue'
import { weatherInfo } from '../lib/weatherCodes'
import WeatherSvg from './icons/WeatherSvg.vue'

const props = defineProps<{ code: number; night?: boolean; size?: string }>()
const info = computed(() => weatherInfo(props.code, props.night))
</script>

<template>
  <span
    class="weather-icon"
    role="img"
    :aria-label="info.label"
    :title="info.label"
    :style="{ width: size ?? '1.5em', height: size ?? '1.5em' }"
  >
    <WeatherSvg :name="info.icon" />
  </span>
</template>

<style scoped>
.weather-icon {
  display: inline-block;
  vertical-align: middle;
}
.weather-icon svg {
  width: 100%;
  height: 100%;
}
</style>
