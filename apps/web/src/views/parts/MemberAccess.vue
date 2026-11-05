<script setup lang="ts">
import { ref } from 'vue'
import { api, type Member } from '../../api/client'
import { useMembersStore } from '../../stores/members'
import { useSessionStore } from '../../stores/session'
import { describeError } from '../shared'
import { UiButton, UiCard, UiTextField } from '../../ui'
import { isPin, passwordMin } from './memberForm'

const props = defineProps<{ member: Member }>()
const members = useMembersStore()
const session = useSessionStore()

const username = ref('')
const password = ref('')
const pin = ref('')
const message = ref('')
const error = ref('')
const isSelf = () => session.member?.id === props.member.id
const isAdult = () => props.member.role === 'adult'
/** An adult who has claimed their account is theirs alone (D-006); any adult may set up the rest. */
const claimedByOther = () =>
  isAdult() && !isSelf() && props.member.has_account && !props.member.initial_password

async function run(work: () => Promise<void>, done: string) {
  message.value = ''
  error.value = ''
  try {
    await work()
    await members.load()
    message.value = done
    password.value = ''
    pin.value = ''
  } catch (e) {
    error.value = describeError(e, 'That did not work.')
  }
}

function saveLogin() {
  const min = passwordMin(props.member.role)
  if (!props.member.has_account && !username.value.trim()) {
    error.value = 'Enter a username.'
    return
  }
  if (password.value.length < min) {
    error.value = `Use at least ${min} characters.`
    return
  }
  const id = props.member.id
  void run(
    () =>
      props.member.has_account
        ? api.resetPassword(id, password.value)
        : api.createAccount(id, username.value.trim(), password.value),
    props.member.has_account
      ? 'Password changed.'
      : isAdult()
        ? 'Login created. They must change this password at first sign-in.'
        : 'Login created.',
  )
}

function savePin() {
  if (!isPin(pin.value)) {
    error.value = 'A PIN is 4 to 6 digits.'
    return
  }
  void run(() => api.setPin(props.member.id, pin.value), 'PIN saved.')
}
</script>

<template>
  <UiCard variant="outlined" :data-testid="`access-${member.display_name}`">
    <template #header>{{ isAdult() ? 'Login' : 'Login and PIN' }}</template>
    <p v-if="claimedByOther()" class="muted" data-testid="claimed-note">
      This adult has their own login. Only they can change their password, PIN and settings.
    </p>
    <div v-else class="access">
      <p class="muted">
        {{ member.has_account ? 'Has a login.' : 'No login yet.' }}
        {{ member.has_pin ? 'Has a PIN.' : 'No PIN.' }}
      </p>
      <p v-if="isAdult() && !isSelf()" class="muted">
        They set their own PIN after signing in.
        <template v-if="member.has_account">
          They must change this password at first sign-in.
        </template>
      </p>
      <p v-if="member.has_account && isSelf()" class="muted">
        Change your own password under <RouterLink to="/settings/account">Account</RouterLink>.
      </p>
      <template v-else>
        <UiTextField
          v-if="!member.has_account"
          v-model="username"
          label="Username"
          name="login_username"
          autocomplete="off"
        />
        <UiTextField
          v-model="password"
          :label="member.has_account ? 'New password' : 'Password'"
          name="login_password"
          type="password"
          autocomplete="new-password"
        />
        <div class="actions">
          <UiButton variant="tonal" @click="saveLogin">
            {{ member.has_account ? 'Reset password' : 'Create login' }}
          </UiButton>
        </div>
      </template>
      <template v-if="!isAdult() || isSelf()">
        <UiTextField
          v-model="pin"
          :label="`${member.has_pin ? 'New PIN' : 'PIN'} (4 to 6 digits)`"
          name="pin"
          inputmode="numeric"
          autocomplete="off"
          maxlength="6"
        />
        <div class="actions"><UiButton variant="tonal" @click="savePin">Save PIN</UiButton></div>
      </template>
      <p v-if="message" class="ok" role="status">{{ message }}</p>
      <p v-if="error" class="err" role="alert">{{ error }}</p>
    </div>
  </UiCard>
</template>

<style scoped>
.access {
  display: grid;
  gap: var(--ui-space-3);
}
.actions {
  display: flex;
  gap: var(--ui-space-2);
}
p {
  margin: 0;
}
.muted {
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-md);
}
.ok {
  color: var(--ui-success);
}
.err {
  color: var(--ui-error);
}
</style>
