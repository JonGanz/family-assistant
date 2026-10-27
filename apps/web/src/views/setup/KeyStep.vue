<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api } from '../../api/client'
import { useSetupStore } from '../../stores/setup'
import MasterKeyBackup from '../system/MasterKeyBackup.vue'
import StepButtons from './StepButtons.vue'

const setup = useSetupStore()
const confirmed = ref(false)

onMounted(async () => {
  try {
    confirmed.value = !!(await api.systemSettings()).key_backup_confirmed_at
  } catch {
    /* the flow below still works */
  }
})
</script>

<template>
  <div data-testid="step-key">
    <h2>Master key backup</h2>
    <p v-if="confirmed" class="text-success" role="status">The backup is confirmed.</p>
    <MasterKeyBackup v-else @confirmed="confirmed = true" />
    <form class="form" @submit.prevent="setup.goto(7)">
      <StepButtons :disabled="!confirmed" @back="setup.goto(5)" />
    </form>
  </div>
</template>
