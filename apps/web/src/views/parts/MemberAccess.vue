<script setup lang="ts">
import { ref } from 'vue'
import { api, type Member } from '../../api/client'
import ErrorBanner from '../../components/ErrorBanner.vue'
import { useMembersStore } from '../../stores/members'
import { useSessionStore } from '../../stores/session'
import { describeError } from '../shared'
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
    props.member.has_account ? 'Password changed.' : 'Login created.',
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
  <fieldset class="access" :data-testid="`access-${member.display_name}`">
    <legend>Login and PIN</legend>
    <p class="muted">
      {{ member.has_account ? 'Has a login.' : 'No login yet.' }}
      {{ member.has_pin ? 'Has a PIN.' : 'No PIN.' }}
    </p>
    <template v-if="member.has_account && isSelf()">
      <p class="muted">
        Change your own password under <RouterLink to="/settings/account">Account</RouterLink>.
      </p>
    </template>
    <template v-else>
      <label v-if="!member.has_account" class="field">
        Username
        <input v-model="username" name="login_username" autocomplete="off" />
      </label>
      <label class="field">
        {{ member.has_account ? 'New password' : 'Password' }}
        <input
          v-model="password"
          name="login_password"
          type="password"
          autocomplete="new-password"
        />
      </label>
      <button type="button" class="btn-sm" @click="saveLogin">
        {{ member.has_account ? 'Reset password' : 'Create login' }}
      </button>
    </template>
    <label class="field">
      {{ member.has_pin ? 'New PIN' : 'PIN' }} (4 to 6 digits)
      <input v-model="pin" name="pin" inputmode="numeric" autocomplete="off" maxlength="6" />
    </label>
    <button type="button" class="btn-sm" @click="savePin">Save PIN</button>
    <p v-if="message" class="text-success" role="status">{{ message }}</p>
    <ErrorBanner v-if="error" :message="error" />
  </fieldset>
</template>
