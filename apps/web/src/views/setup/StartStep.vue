<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { api } from '../../api/client'
import { useSetupStore } from '../../stores/setup'
import { UiButton, UiCard } from '../../ui'
import { describeError } from '../shared'
import StepButtons from './StepButtons.vue'

const setup = useSetupStore()
const router = useRouter()
const error = ref('')
const busy = ref(false)

async function finish(to: string) {
  error.value = ''
  busy.value = true
  try {
    await api.setupComplete()
    await setup.load(true)
    try {
      localStorage.removeItem('fa.setup.step')
    } catch {
      /* private window */
    }
    await router.replace(to)
  } catch (e) {
    error.value = describeError(e, 'Could not finish the setup.')
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <form class="step" data-testid="step-start" @submit.prevent="finish('/')">
    <UiCard>
      <template #header>Start</template>
      <div class="stack">
        <p>Nothing is added for you: you start with an empty calendar and task list.</p>
        <p>Afterwards, connect your calendars and mail, and pair the wall screen:</p>
        <div class="links">
          <UiButton variant="tonal" icon="calendar" @click="finish('/settings/calendars')">
            Calendars
          </UiButton>
          <UiButton variant="tonal" icon="mail" @click="finish('/settings/mail')">Mail</UiButton>
          <UiButton variant="tonal" icon="devices" @click="finish('/settings/devices')">
            Devices
          </UiButton>
        </div>
        <p v-if="error" class="err" role="alert">{{ error }}</p>
      </div>
      <template #actions>
        <StepButtons next="Finish" :busy="busy" @back="setup.goto(6)" />
      </template>
    </UiCard>
  </form>
</template>

<style scoped>
.stack {
  display: grid;
  gap: var(--ui-space-3);
}
.links {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
}
p {
  margin: 0;
}
.err {
  color: var(--ui-error);
}
</style>
