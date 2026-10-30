<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { api } from '../api/client'
import ErrorBanner from '../components/ErrorBanner.vue'
import PageHeader from '../components/PageHeader.vue'
import { useSessionStore } from '../stores/session'
import { passwordMin } from './parts/memberForm'
import { describeError } from './shared'

const session = useSessionStore()
const form = reactive({ current: '', next: '', again: '' })
const message = ref('')
const error = ref('')
const busy = ref(false)
const min = computed(() => passwordMin(session.member?.role ?? 'adult'))

async function save() {
  message.value = ''
  error.value = ''
  const id = session.member?.id
  if (!id) return
  if (form.next.length < min.value) {
    error.value = `Use at least ${min.value} characters.`
    return
  }
  if (form.next !== form.again) {
    error.value = 'The new passwords do not match.'
    return
  }
  busy.value = true
  try {
    await api.changeOwnPassword(id, { current_password: form.current, new_password: form.next })
    Object.assign(form, { current: '', next: '', again: '' })
    message.value = 'Password changed. Your other sessions were signed out.'
    await session.load(true)
  } catch (e) {
    const status = (e as { status?: number }).status
    error.value =
      status === 401 || status === 403
        ? 'The current password is wrong.'
        : describeError(e, 'Could not change the password.')
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <section aria-labelledby="page-title">
    <PageHeader eyebrow="Personal" title="Account" />
    <p v-if="session.mustChangePassword" class="text-danger" role="alert" data-testid="must-change">
      Another adult chose your password. Please choose your own before you continue.
    </p>
    <form class="form" data-testid="own-password" @submit.prevent="save">
      <h2>Change your password</h2>
      <label class="field">
        Current password
        <input
          v-model="form.current"
          name="current_password"
          type="password"
          autocomplete="current-password"
          required
        />
      </label>
      <label class="field">
        New password ({{ min }} characters or more)
        <input
          v-model="form.next"
          name="new_password"
          type="password"
          autocomplete="new-password"
          required
        />
      </label>
      <label class="field">
        New password again
        <input
          v-model="form.again"
          name="new_password_again"
          type="password"
          autocomplete="new-password"
          required
        />
      </label>
      <ErrorBanner v-if="error" :message="error" />
      <p v-if="message" class="text-success" role="status">{{ message }}</p>
      <button type="submit" class="btn-primary" :disabled="busy">Change password</button>
    </form>
  </section>
</template>
