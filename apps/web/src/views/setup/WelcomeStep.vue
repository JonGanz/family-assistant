<script setup lang="ts">
import { computed } from 'vue'
import { useSetupStore } from '../../stores/setup'
import { UiCard } from '../../ui'
import StepButtons from './StepButtons.vue'

const setup = useSetupStore()
const s = computed(() => setup.status)
</script>

<template>
  <form class="step" @submit.prevent="setup.goto(setup.step + 1)">
    <UiCard>
      <template #header>Welcome</template>
      <p>This takes a few minutes. Each step saves when you press Next.</p>
      <ul class="facts" data-testid="welcome-facts">
        <li>
          Address: <code>{{ s?.origin ?? '' }}</code>
        </li>
        <li>
          Connection:
          {{ s?.secure ? 'HTTPS' : 'plain HTTP (fine on a home network)' }}
        </li>
        <li v-if="s?.via_tailscale">Reached through Tailscale.</li>
      </ul>
      <template #actions><StepButtons :back="false" next="Start" /></template>
    </UiCard>
  </form>
</template>

<style scoped>
p {
  margin: 0 0 var(--ui-space-2);
}
.facts {
  margin: 0;
  padding-left: var(--ui-space-5);
}
code {
  overflow-wrap: anywhere;
}
</style>
