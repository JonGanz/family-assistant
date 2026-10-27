<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { api } from '../../api/client'
import ErrorBanner from '../../components/ErrorBanner.vue'
import { useHouseholdStore } from '../../stores/household'
import { useMembersStore } from '../../stores/members'
import { useSetupStore } from '../../stores/setup'
import { describeError } from '../shared'
import HouseholdFile from '../system/HouseholdFile.vue'
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

function imported() {
  void useHouseholdStore().load()
  void useMembersStore().load()
}
</script>

<template>
  <form class="form" data-testid="step-start" @submit.prevent="finish('/')">
    <h2>Start</h2>
    <p>
      Nothing is added for you: start with an empty calendar and task list, or bring a household
      file you exported from another install.
    </p>
    <fieldset>
      <legend>Import a household file (optional)</legend>
      <HouseholdFile :exportable="false" @applied="imported" />
    </fieldset>
    <p>Afterwards, connect your calendars and mail, and pair the wall screen:</p>
    <ul class="row">
      <li>
        <button type="button" class="btn-ghost" @click="finish('/settings/calendars')">
          Calendars
        </button>
      </li>
      <li>
        <button type="button" class="btn-ghost" @click="finish('/settings/mail')">Mail</button>
      </li>
      <li>
        <button type="button" class="btn-ghost" @click="finish('/settings/devices')">
          Devices
        </button>
      </li>
    </ul>
    <ErrorBanner v-if="error" :message="error" />
    <StepButtons next="Finish" :busy="busy" @back="setup.goto(6)" />
  </form>
</template>
