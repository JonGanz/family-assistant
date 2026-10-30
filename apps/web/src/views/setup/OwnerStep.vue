<script setup lang="ts">
import { ref } from 'vue'
import { api } from '../../api/client'
import { problemCode } from '../../api/http'
import HuePicker from '../../components/HuePicker.vue'
import ErrorBanner from '../../components/ErrorBanner.vue'
import { useSessionStore } from '../../stores/session'
import { useSetupStore } from '../../stores/setup'
import { describeError } from '../shared'
import StepButtons from './StepButtons.vue'
import { validateOwner } from './validate'

const setup = useSetupStore()
const session = useSessionStore()
const f = setup.owner
const errors = ref<Record<string, string>>({})
const error = ref('')
const busy = ref(false)

async function next() {
  error.value = ''
  if (!setup.status?.owner_needed) {
    setup.goto(4)
    return
  }
  errors.value = validateOwner(f)
  if (Object.keys(errors.value).length) return
  busy.value = true
  try {
    await api.setupOwner({
      display_name: f.display_name.trim(),
      color: f.color,
      username: f.username.trim(),
      password: f.password,
      pin: f.pin || null,
    })
    f.password = ''
    f.again = ''
    await session.load(true)
    await setup.load(true)
    setup.goto(4)
  } catch (e) {
    if (problemCode(e) === 'setup_closed') await setup.load(true)
    error.value = describeError(e, 'Could not create your account.')
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <form
    v-if="setup.status?.owner_needed"
    class="form"
    data-testid="step-owner"
    @submit.prevent="next"
  >
    <h2>You</h2>
    <p class="muted">The first adult. You will be signed in and finish the rest as this person.</p>
    <label class="field">
      Your name
      <input v-model="f.display_name" name="display_name" autocomplete="name" required />
    </label>
    <p v-if="errors.display_name" class="text-danger" role="alert">{{ errors.display_name }}</p>
    <HuePicker v-model="f.color" name="color" />
    <label class="field">
      Username
      <input v-model="f.username" name="username" autocomplete="username" required />
    </label>
    <p v-if="errors.username" class="text-danger" role="alert">{{ errors.username }}</p>
    <label class="field">
      Password (12 characters or more)
      <input
        v-model="f.password"
        name="password"
        type="password"
        autocomplete="new-password"
        required
      />
    </label>
    <p v-if="errors.password" class="text-danger" role="alert">{{ errors.password }}</p>
    <label class="field">
      Password again
      <input
        v-model="f.again"
        name="password_again"
        type="password"
        autocomplete="new-password"
        required
      />
    </label>
    <p v-if="errors.again" class="text-danger" role="alert">{{ errors.again }}</p>
    <label class="field">
      PIN for the wall screen (optional, 4 to 6 digits)
      <input v-model="f.pin" name="pin" inputmode="numeric" autocomplete="off" maxlength="6" />
    </label>
    <p v-if="errors.pin" class="text-danger" role="alert">{{ errors.pin }}</p>
    <ErrorBanner v-if="error" :message="error" />
    <StepButtons :busy="busy" :back="!!setup.status?.needed" @back="setup.goto(2)" />
  </form>
  <form v-else class="form" @submit.prevent="setup.goto(4)">
    <h2>You</h2>
    <p>Your account is ready.</p>
    <StepButtons :back="false" />
  </form>
</template>
