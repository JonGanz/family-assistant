<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api } from '../api/client'
import { request } from '../api/http'
import type { components } from '../gen/api'
import SettingsPage from '../app/SettingsPage.vue'
import { useSessionStore } from '../stores/session'
import {
  UiButton,
  UiCard,
  UiDialog,
  UiEmptyState,
  UiErrorState,
  UiSaveBar,
  UiSkeleton,
  UiTextField,
  useFormDraft,
  useSnackbar,
} from '../ui'
import { passwordMin } from './parts/memberForm'
import { describeError } from './shared'

type SessionView = components['schemas']['SessionView']

const session = useSessionStore()
const snackbar = useSnackbar()
const min = computed(() => passwordMin(session.member?.role ?? 'adult'))

// Password: one draft, one Save bar (D-096).
const blank = () => ({ current: '', next: '', again: '' })
const fieldErrors = ref<Partial<Record<'current' | 'next' | 'again', string>>>({})
// After a save the source is replaced by a blank form, which empties the fields.
const source = ref(blank())
const form = useFormDraft(
  () => source.value,
  async (d) => {
    fieldErrors.value = {}
    const id = session.member?.id
    if (!id) throw new Error('You are not signed in.')
    if (d.next.length < min.value) {
      fieldErrors.value = { next: `Use at least ${min.value} characters.` }
      throw new Error('Check the new password.')
    }
    if (d.next !== d.again) {
      fieldErrors.value = { again: 'The new passwords do not match.' }
      throw new Error('Check the new password.')
    }
    try {
      await api.changeOwnPassword(id, { current_password: d.current, new_password: d.next })
    } catch (e) {
      const status = (e as { status?: number }).status
      if (status === 401 || status === 403) {
        fieldErrors.value = { current: 'The current password is wrong.' }
        throw new Error('The current password is wrong.', { cause: e })
      }
      throw new Error(describeError(e, 'Could not change the password.'), { cause: e })
    }
    snackbar.show({ text: 'Password changed. Your other sessions were signed out.' })
    await session.load(true)
  },
)
const d = form.draft

async function submit() {
  if (await form.save()) source.value = blank()
}

// Sessions: a list with revoke (confirm).
const sessions = ref<SessionView[]>([])
const sessionsLoading = ref(true)
const sessionsError = ref('')
const revoking = ref<SessionView | null>(null)

async function loadSessions() {
  sessionsLoading.value = true
  sessionsError.value = ''
  try {
    const page = await request<components['schemas']['SessionPage']>('/api/v1/sessions', {
      bearer: false,
    })
    sessions.value = page.items
  } catch (e) {
    sessionsError.value = describeError(e, 'Could not load the sessions.')
  } finally {
    sessionsLoading.value = false
  }
}
onMounted(loadSessions)

async function revoke() {
  const s = revoking.value
  revoking.value = null
  if (!s) return
  try {
    await request<void>(`/api/v1/sessions/${encodeURIComponent(s.id)}`, {
      method: 'DELETE',
      bearer: false,
    })
    snackbar.show({ text: 'Session ended' })
    await loadSessions()
  } catch (e) {
    snackbar.show({ text: describeError(e, 'Could not end the session.') })
  }
}

const when = (iso: string) =>
  new Date(iso).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })
</script>

<template>
  <SettingsPage title="Account">
    <UiCard
      v-if="session.mustChangePassword"
      variant="outlined"
      role="alert"
      data-testid="must-change"
    >
      Another adult chose your password. Please choose your own before you continue.
    </UiCard>

    <UiCard data-testid="own-password">
      <template #header>Change your password</template>
      <form class="stack" @submit.prevent="submit">
        <UiTextField
          v-model="d.current"
          label="Current password"
          name="current_password"
          type="password"
          autocomplete="current-password"
          :error="fieldErrors.current"
          required
        />
        <UiTextField
          v-model="d.next"
          :label="`New password (${min} characters or more)`"
          name="new_password"
          type="password"
          autocomplete="new-password"
          :error="fieldErrors.next"
          required
        />
        <UiTextField
          v-model="d.again"
          label="New password again"
          name="new_password_again"
          type="password"
          autocomplete="new-password"
          :error="fieldErrors.again"
          required
        />
      </form>
    </UiCard>

    <UiCard data-testid="sessions">
      <template #header>Sessions</template>
      <UiSkeleton v-if="sessionsLoading && !sessions.length" :lines="3" />
      <UiErrorState v-else-if="sessionsError" :message="sessionsError" @retry="loadSessions" />
      <UiEmptyState v-else-if="!sessions.length" icon="lock" title="No other sessions" />
      <ul v-else class="sessions">
        <li v-for="s in sessions" :key="s.id" class="session">
          <div class="session__text">
            <strong>{{ s.user_agent || 'Unknown browser' }}</strong>
            <span class="muted"
              >Signed in {{ when(s.created_at) }}, last seen {{ when(s.last_seen) }}</span
            >
          </div>
          <UiButton variant="text" @click="revoking = s">Sign out</UiButton>
        </li>
      </ul>
    </UiCard>

    <UiDialog
      :model-value="!!revoking"
      title="Sign out this session?"
      @update:model-value="!$event && (revoking = null)"
    >
      <p>If this is the session you are using now, you will be signed out here too.</p>
      <template #actions>
        <UiButton variant="text" @click="revoking = null">Cancel</UiButton>
        <UiButton @click="revoke">Sign out</UiButton>
      </template>
    </UiDialog>

    <UiSaveBar
      :dirty="form.dirty.value"
      :saving="form.saving.value"
      :error="form.error.value"
      @save="submit"
      @discard="form.discard"
    />
  </SettingsPage>
</template>

<style scoped>
.stack {
  display: grid;
  gap: var(--ui-space-3);
}
.sessions {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--ui-space-3);
}
.session {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ui-space-3);
}
.session__text {
  display: grid;
  min-width: 0;
}
.session__text strong {
  overflow-wrap: anywhere;
}
.muted {
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
</style>
