<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api } from '../../api/client'
import { useSetupStore } from '../../stores/setup'
import { UiCard } from '../../ui'
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
  <div class="step" data-testid="step-key">
    <UiCard>
      <template #header>Master key backup</template>
      <p v-if="confirmed" class="ok" role="status">The backup is confirmed.</p>
      <MasterKeyBackup v-else @confirmed="confirmed = true" />
    </UiCard>
    <form @submit.prevent="setup.goto(7)">
      <StepButtons :disabled="!confirmed" @back="setup.goto(5)" />
    </form>
  </div>
</template>

<style scoped>
.step {
  display: grid;
  gap: var(--ui-space-4);
}
p {
  margin: 0;
}
.ok {
  color: var(--ui-success);
}
</style>
