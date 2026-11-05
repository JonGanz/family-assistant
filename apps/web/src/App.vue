<script setup lang="ts">
import { computed, watch } from 'vue'
import { useRoute } from 'vue-router'
import { applyDensity } from './app/density'
import AppLayout from './layouts/AppLayout.vue'
import BareLayout from './layouts/BareLayout.vue'
import KioskLayout from './layouts/KioskLayout.vue'
import SetupLayout from './layouts/SetupLayout.vue'
import { useDeviceTheme } from './themes'
import { useHouseholdStore } from './stores/household'
import { useLiveStore } from './stores/live'
import { useSessionStore } from './stores/session'

const route = useRoute()
const LAYOUTS = { kiosk: KioskLayout, setup: SetupLayout, app: AppLayout, bare: BareLayout }
const layout = computed(() => LAYOUTS[route.meta.layout ?? 'app'])

// The kiosk shell's theme comes from device settings, so the bootstrap leaves /dashboard alone.
useDeviceTheme(() => route.meta.layout !== 'kiosk')
applyDensity()

const session = useSessionStore()
const household = useHouseholdStore()
const live = useLiveStore()

// One app-level subscription while a user is signed in; stores refetch on events.
watch(
  () => session.isUser,
  (signedIn) => {
    if (signedIn) {
      void household.load()
      live.start()
    } else {
      live.stop()
    }
  },
  { immediate: true },
)
</script>

<template>
  <component :is="layout">
    <RouterView />
  </component>
</template>
