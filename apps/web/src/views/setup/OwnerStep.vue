<script setup lang="ts">
import { ref } from 'vue'
import { api } from '../../api/client'
import { problemCode } from '../../api/http'
import HuePicker from '../../components/HuePicker.vue'
import { useSessionStore } from '../../stores/session'
import { useSetupStore } from '../../stores/setup'
import { UiCard, UiTextField } from '../../ui'
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
    class="step"
    data-testid="step-owner"
    @submit.prevent="next"
  >
    <UiCard>
      <template #header>You</template>
      <div class="stack">
        <p class="muted">
          The first adult. You will be signed in and finish the rest as this person.
        </p>
        <UiTextField
          v-model="f.display_name"
          label="Your name"
          name="display_name"
          autocomplete="name"
          :error="errors.display_name"
          required
        />
        <HuePicker v-model="f.color" name="color" />
        <UiTextField
          v-model="f.username"
          label="Username"
          name="username"
          autocomplete="username"
          :error="errors.username"
          required
        />
        <UiTextField
          v-model="f.password"
          label="Password (12 characters or more)"
          name="password"
          type="password"
          autocomplete="new-password"
          :error="errors.password"
          required
        />
        <UiTextField
          v-model="f.again"
          label="Password again"
          name="password_again"
          type="password"
          autocomplete="new-password"
          :error="errors.again"
          required
        />
        <UiTextField
          v-model="f.pin"
          label="PIN for the wall screen (optional, 4 to 6 digits)"
          name="pin"
          inputmode="numeric"
          autocomplete="off"
          maxlength="6"
          :error="errors.pin"
        />
        <p v-if="error" class="err" role="alert">{{ error }}</p>
      </div>
      <template #actions>
        <StepButtons :busy="busy" :back="!!setup.status?.needed" @back="setup.goto(2)" />
      </template>
    </UiCard>
  </form>
  <form v-else class="step" @submit.prevent="setup.goto(4)">
    <UiCard>
      <template #header>You</template>
      <p class="muted">Your account is ready.</p>
      <template #actions><StepButtons :back="false" /></template>
    </UiCard>
  </form>
</template>

<style scoped>
.stack {
  display: grid;
  gap: var(--ui-space-3);
}
p {
  margin: 0;
}
.muted {
  color: var(--ui-on-surface-variant);
}
.err {
  color: var(--ui-error);
}
</style>
