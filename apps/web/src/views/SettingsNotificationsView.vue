<script setup lang="ts">
import QRCode from 'qrcode'
import { computed, onMounted, reactive, ref, watch } from 'vue'
import type { NotifyTrigger } from '../api/client'
import type { ApprovalMode } from '../api/mailSettings'
import ErrorBanner from '../components/ErrorBanner.vue'
import PageHeader from '../components/PageHeader.vue'
import { useHouseholdStore } from '../stores/household'
import { useMembersStore } from '../stores/members'
import { useNotifyPrefsStore } from '../stores/notifyPrefs'
import { useSessionStore } from '../stores/session'
import { describeError } from './shared'

const session = useSessionStore()
const members = useMembersStore()
const household = useHouseholdStore()
const store = useNotifyPrefsStore()

const TRIGGERS: { key: NotifyTrigger; label: string; adultOnly?: boolean }[] = [
  { key: 'assigned', label: 'A task is assigned to me' },
  { key: 'due_soon', label: 'A task is due soon' },
  { key: 'overdue', label: 'A task is overdue' },
  { key: 'digest', label: 'Morning digest' },
  { key: 'routine_nudge', label: 'A routine is running late' },
  { key: 'source_reauth', label: 'A calendar needs to be reconnected', adultOnly: true },
  { key: 'approval_needed', label: 'Something is waiting for my approval', adultOnly: true },
  { key: 'mail_time_sensitive', label: 'A time-sensitive email arrived', adultOnly: true },
]
const APPROVAL_MODES: { key: ApprovalMode; label: string }[] = [
  { key: 'immediate', label: 'Immediately' },
  { key: 'batched', label: 'Batched (a short wait, then one message)' },
  { key: 'digest', label: 'Only in the morning digest' },
  { key: 'off', label: 'Never' },
]

const selected = ref('')
const form = reactive({
  triggers: [] as NotifyTrigger[],
  dueSoon: 60,
  nudge: 15,
  override: false,
  from: '21:00',
  until: '07:00',
  copyTo: '',
  approval: 'batched' as ApprovalMode,
})
const message = ref('')
const error = ref('')
const testResult = ref<{ ok: boolean; message: string } | null>(null)
const qr = ref('')

const choices = computed(() =>
  session.isAdult
    ? members.items.filter((m) => m.id === session.member?.id || m.role === 'child')
    : members.items.filter((m) => m.id === session.member?.id),
)
const adults = computed(() =>
  members.items.filter((m) => m.role === 'adult' && m.id !== selected.value),
)
const isChild = computed(() => members.items.find((m) => m.id === selected.value)?.role === 'child')
const triggers = computed(() => TRIGGERS.filter((t) => !t.adultOnly || !isChild.value))

function fill() {
  const p = store.prefs
  if (!p) return
  form.triggers = [...p.enabled_triggers]
  form.dueSoon = p.due_soon_minutes
  form.nudge = p.nudge_minutes
  form.override = !!(p.quiet_from && p.quiet_until)
  form.from = p.quiet_from ?? household.quiet.from
  form.until = p.quiet_until ?? household.quiet.until
  form.copyTo = p.copy_to_member ?? ''
  form.approval = p.approval_mode ?? 'batched'
}

async function pick(id: string) {
  selected.value = id
  message.value = ''
  error.value = ''
  testResult.value = null
  await store.load(id)
  fill()
}

onMounted(async () => {
  await Promise.all([members.load(), household.load()])
  if (session.member) await pick(session.member.id)
})
watch(() => store.prefs?.version, fill)
watch(
  () => store.prefs?.subscribe_url,
  async (url) => {
    qr.value = ''
    if (!url) return
    try {
      qr.value = await QRCode.toDataURL(url, { margin: 1, width: 192 })
    } catch {
      qr.value = ''
    }
  },
  { immediate: true },
)

function toggle(t: NotifyTrigger) {
  form.triggers = form.triggers.includes(t)
    ? form.triggers.filter((x) => x !== t)
    : [...form.triggers, t]
}

async function save() {
  message.value = ''
  error.value = ''
  try {
    const ok = await store.save({
      enabled_triggers: form.triggers,
      due_soon_minutes: form.dueSoon,
      nudge_minutes: form.nudge,
      quiet_from: form.override ? form.from : null,
      quiet_until: form.override ? form.until : null,
      copy_to_member: form.copyTo || null,
      approval_mode: isChild.value ? undefined : form.approval,
    })
    if (ok) message.value = 'Saved.'
  } catch (e) {
    error.value = describeError(e, 'Save failed.')
  }
}

async function sendTest() {
  testResult.value = null
  const r = await store.test()
  testResult.value = r.ok
    ? { ok: true, message: 'Test message sent. Check your phone.' }
    : { ok: false, message: r.message }
}
</script>

<template>
  <section aria-labelledby="page-title">
    <PageHeader eyebrow="Settings" title="Notifications" />
    <label v-if="choices.length > 1" class="field-inline picker">
      Settings for
      <select
        :value="selected"
        name="member"
        @change="pick(($event.target as HTMLSelectElement).value)"
      >
        <option v-for="m in choices" :key="m.id" :value="m.id">
          {{ m.display_name }}{{ m.id === session.member?.id ? ' (me)' : '' }}
        </option>
      </select>
    </label>
    <ErrorBanner v-if="store.error" :message="store.error" />

    <template v-if="store.prefs">
      <p
        v-if="!store.prefs.server_configured"
        class="muted"
        role="status"
        data-testid="not-configured"
      >
        Notifications are not configured on this server. Ask whoever runs it to set the ntfy
        address. You can still choose your settings below.
      </p>

      <form class="form" @submit.prevent="save">
        <fieldset>
          <legend>Tell me when</legend>
          <label v-for="t in triggers" :key="t.key" class="field-inline">
            <input
              type="checkbox"
              :checked="form.triggers.includes(t.key)"
              @change="toggle(t.key)"
            />
            {{ t.label }}
          </label>
        </fieldset>
        <label class="field">
          Minutes before a task is due
          <input v-model.number="form.dueSoon" name="due_soon" type="number" min="1" max="1440" />
        </label>
        <label class="field">
          Minutes before a routine deadline to nudge
          <input v-model.number="form.nudge" name="nudge" type="number" min="1" max="240" />
        </label>
        <fieldset>
          <legend>Quiet hours</legend>
          <label class="field-inline">
            <input v-model="form.override" type="checkbox" name="quiet_override" />
            Use my own quiet hours (the household default is
            {{ household.quiet.from }}–{{ household.quiet.until }})
          </label>
          <template v-if="form.override">
            <label class="field"
              >From <input v-model="form.from" name="quiet_from" type="time"
            /></label>
            <label class="field"
              >Until <input v-model="form.until" name="quiet_until" type="time"
            /></label>
          </template>
        </fieldset>
        <label v-if="!isChild" class="field">
          Approval reminders
          <select v-model="form.approval" name="approval_mode">
            <option v-for="a in APPROVAL_MODES" :key="a.key" :value="a.key">{{ a.label }}</option>
          </select>
        </label>
        <label class="field">
          Also send a copy to
          <select v-model="form.copyTo" name="copy_to">
            <option value="">Nobody</option>
            <option v-for="m in adults" :key="m.id" :value="m.id">{{ m.display_name }}</option>
          </select>
        </label>
        <ErrorBanner v-if="store.conflict" conflict @reload="pick(selected)" />
        <ErrorBanner v-if="error" :message="error" />
        <p v-if="message" class="text-success" role="status">{{ message }}</p>
        <button type="submit" class="btn-primary">Save</button>
      </form>

      <h2>Phone</h2>
      <div v-if="store.prefs.subscribe_url" class="subscribe">
        <p>
          Open this address in the ntfy app (or scan the code) to subscribe:
          <br /><code data-testid="subscribe-url">{{ store.prefs.subscribe_url }}</code>
        </p>
        <img
          v-if="qr"
          :src="qr"
          width="192"
          height="192"
          alt="QR code for the ntfy subscribe address"
        />
      </div>
      <p v-else class="muted">No subscribe address is available yet.</p>
      <button type="button" :disabled="!store.prefs.server_configured" @click="sendTest">
        Send test
      </button>
      <p
        v-if="testResult"
        :class="testResult.ok ? 'text-success' : 'text-danger'"
        role="status"
        data-testid="test-result"
      >
        {{ testResult.message }}
      </p>
    </template>
  </section>
</template>

<style scoped>
.picker {
  margin-bottom: var(--space-3);
}
code {
  word-break: break-all;
}
</style>
