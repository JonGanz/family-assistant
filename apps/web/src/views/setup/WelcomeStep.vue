<script setup lang="ts">
import { computed } from 'vue'
import { useSetupStore } from '../../stores/setup'
import StepButtons from './StepButtons.vue'

const setup = useSetupStore()
const s = computed(() => setup.status)
</script>

<template>
  <form class="form" @submit.prevent="setup.goto(setup.step + 1)">
    <h2>Welcome</h2>
    <p>This takes a few minutes. Each step saves when you press Next.</p>
    <ul class="card-list" data-testid="welcome-facts">
      <li>
        Address: <code>{{ s?.origin ?? '' }}</code>
      </li>
      <li>
        Connection:
        {{ s?.secure ? 'HTTPS' : 'plain HTTP (fine on a home network)' }}
      </li>
      <li v-if="s?.via_tailscale">Reached through Tailscale.</li>
    </ul>
    <StepButtons :back="false" next="Start" />
  </form>
</template>
