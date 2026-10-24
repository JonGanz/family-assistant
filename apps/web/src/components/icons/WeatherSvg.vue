<script setup lang="ts">
import type { WeatherIconName } from '../../lib/weatherCodes'
defineProps<{ name: WeatherIconName }>()
</script>

<template>
  <svg
    viewBox="0 0 64 64"
    fill="none"
    stroke="currentColor"
    stroke-width="3"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <template v-if="name === 'clear'">
      <circle cx="32" cy="32" r="11" />
      <path d="M32 8v7M32 49v7M8 32h7M49 32h7M15 15l5 5M44 44l5 5M49 15l-5 5M20 44l-5 5" />
    </template>
    <template v-else-if="name === 'clear-night'">
      <path d="M44 38A16 16 0 0 1 26 20a16 16 0 1 0 18 18z" />
    </template>
    <template v-else-if="name === 'partly-cloudy' || name === 'partly-cloudy-night'">
      <path
        v-if="name === 'partly-cloudy'"
        d="M24 10v5M10 24h5M13 13l4 4M35 13l-4 4M24 20a6 6 0 1 0 0 .1"
      />
      <path v-else d="M28 12A9 9 0 0 1 16 24a9 9 0 1 0 12-12z" />
      <path d="M22 50h26a9 9 0 0 0 0-18 12 12 0 0 0-23 3 7.5 7.5 0 0 0-3 15z" />
    </template>
    <template v-else>
      <path d="M18 38h28a9 9 0 0 0 0-18 12 12 0 0 0-23 3 7.5 7.5 0 0 0-5 15z" />
      <path v-if="name === 'fog'" d="M14 46h36M20 53h28" />
      <path v-else-if="name === 'drizzle'" d="M24 46v2M34 46v2M44 46v2M29 53v2M39 53v2" />
      <path v-else-if="name === 'rain'" d="M22 46l-3 8M33 46l-3 8M44 46l-3 8" />
      <path v-else-if="name === 'showers'" d="M24 46l-3 8M38 46l-3 8" />
      <path v-else-if="name === 'freezing-rain'" d="M22 46l-3 8M44 46l-3 8M33 46v8M29 50h8" />
      <path
        v-else-if="name === 'snow'"
        d="M22 48h.1M33 48h.1M44 48h.1M27 55h.1M38 55h.1"
        stroke-width="5"
      />
      <path v-else-if="name === 'snow-showers'" d="M24 48h.1M40 48h.1M32 55h.1" stroke-width="5" />
      <path v-else-if="name === 'thunder'" d="M34 40l-7 10h8l-5 9" />
    </template>
  </svg>
</template>
