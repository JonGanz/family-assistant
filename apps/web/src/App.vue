<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import AppLayout from './layouts/AppLayout.vue'
import KioskLayout from './layouts/KioskLayout.vue'

const route = useRoute()
const layout = computed(() => (route.meta.layout === 'kiosk' ? KioskLayout : AppLayout))
</script>

<template>
  <component :is="layout">
    <RouterView />
  </component>
</template>
