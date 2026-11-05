<script setup lang="ts">
import QRCode from 'qrcode'
import { computed, onMounted, ref, watch } from 'vue'
import type { NotifyTrigger } from '../api/client'
import type { ApprovalMode } from '../api/mailSettings'
import SettingsPage from '../app/SettingsPage.vue'
import { useHouseholdStore } from '../stores/household'
import { useMembersStore } from '../stores/members'
import { useNotifyPrefsStore } from '../stores/notifyPrefs'
import { useSessionStore } from '../stores/session'
import { UiButton, UiCard, UiSaveBar, UiSelect, UiSwitch, UiTextField, useFormDraft } from '../ui'
import InstantCheck from './parts/settings-mail/InstantCheck.vue'
import InstantSelect from './parts/settings-mail/InstantSelect.vue'
import PageNotices from './parts/settings-mail/PageNotices.vue'

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

interface Draft {
  dueSoon: string
  nudge: string
  override: boolean
  from: string
  until: string
}
function draftFromPrefs(): Draft {
  const p = store.prefs
  return {
    dueSoon: String(p?.due_soon_minutes ?? 60),
    nudge: String(p?.nudge_minutes ?? 15),
    override: !!(p?.quiet_from && p?.quiet_until),
    from: p?.quiet_from ?? household.quiet.from,
    until: p?.quiet_until ?? household.quiet.until,
  }
}
const form = useFormDraft(draftFromPrefs, async (d) => {
  const ok = await store.save({
    due_soon_minutes: Number(d.dueSoon),
    nudge_minutes: Number(d.nudge),
    quiet_from: d.override ? d.from : null,
    quiet_until: d.override ? d.until : null,
  })
  if (!ok) throw new Error('Changed elsewhere. Reload to see the latest, then try again.')
})
const d = form.draft

async function pick(id: string) {
  selected.value = id
  error.value = ''
  testResult.value = null
  await store.load(id)
  form.discard()
}

onMounted(async () => {
  await Promise.all([members.load(), household.load()])
  if (session.member) await pick(session.member.id)
})
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

const triggerOn = (t: NotifyTrigger) => !!store.prefs?.enabled_triggers.includes(t)
function setTrigger(t: NotifyTrigger, on: boolean) {
  const cur = store.prefs?.enabled_triggers ?? []
  const next = on ? [...cur.filter((x) => x !== t), t] : cur.filter((x) => x !== t)
  return store.save({ enabled_triggers: next })
}
const approvalOptions = APPROVAL_MODES.map((a) => ({ value: a.key, label: a.label }))
const copyOptions = computed(() => [
  { value: '', label: 'Nobody' },
  ...adults.value.map((m) => ({ value: m.id, label: m.display_name })),
])
const memberOptions = computed(() =>
  choices.value.map((m) => ({
    value: m.id,
    label: `${m.display_name}${m.id === session.member?.id ? ' (me)' : ''}`,
  })),
)

async function sendTest() {
  testResult.value = null
  const r = await store.test()
  testResult.value = r.ok
    ? { ok: true, message: 'Test message sent. Check your phone.' }
    : { ok: false, message: r.message }
}
</script>

<template>
  <SettingsPage title="Notifications">
    <UiSelect
      v-if="choices.length > 1"
      :model-value="selected"
      label="Settings for"
      name="member"
      :options="memberOptions"
      @update:model-value="pick"
    />
    <PageNotices
      :load-error="store.error"
      :conflict="store.conflict"
      :error="error"
      @reload="pick(selected)"
    />

    <template v-if="store.prefs">
      <UiCard
        v-if="!store.prefs.server_configured"
        variant="outlined"
        role="status"
        data-testid="not-configured"
      >
        <p>
          Notifications are not configured on this server. Ask whoever runs it to set the ntfy
          address. You can still choose your settings below.
        </p>
      </UiCard>

      <UiCard>
        <template #header>Tell me when</template>
        <InstantCheck
          v-for="t in triggers"
          :key="t.key"
          as="switch"
          :model-value="triggerOn(t.key)"
          :label="t.label"
          :save="(v: boolean) => setTrigger(t.key, v)"
        />
      </UiCard>

      <UiCard>
        <template #header>Reminders</template>
        <UiTextField
          v-model="d.dueSoon"
          label="Minutes before a task is due"
          name="due_soon"
          type="number"
          min="1"
          max="1440"
        />
        <UiTextField
          v-model="d.nudge"
          label="Minutes before a routine deadline to nudge"
          name="nudge"
          type="number"
          min="1"
          max="240"
        />
        <UiSwitch
          v-model="d.override"
          :label="`Use my own quiet hours (the household default is ${household.quiet.from}–${household.quiet.until})`"
        />
        <template v-if="d.override">
          <UiTextField v-model="d.from" label="Quiet from" name="quiet_from" type="time" />
          <UiTextField v-model="d.until" label="Quiet until" name="quiet_until" type="time" />
        </template>
      </UiCard>

      <UiCard>
        <template #header>Delivery</template>
        <InstantSelect
          v-if="!isChild"
          :model-value="store.prefs.approval_mode ?? 'batched'"
          label="Approval reminders"
          :options="approvalOptions"
          :save="(v: string) => store.save({ approval_mode: v as ApprovalMode })"
        />
        <InstantSelect
          :model-value="store.prefs.copy_to_member ?? ''"
          label="Also send a copy to"
          :options="copyOptions"
          :save="(v: string) => store.save({ copy_to_member: v || null })"
        />
      </UiCard>
      <UiSaveBar
        :dirty="form.dirty.value"
        :saving="form.saving.value"
        :error="form.error.value"
        @save="form.save"
        @discard="form.discard"
      />

      <UiCard>
        <template #header>Phone</template>
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
        <p v-else class="meta">No subscribe address is available yet.</p>
        <p
          v-if="testResult"
          :class="testResult.ok ? 'text-success' : 'text-danger'"
          role="status"
          data-testid="test-result"
        >
          {{ testResult.message }}
        </p>
        <template #actions>
          <UiButton variant="tonal" :disabled="!store.prefs.server_configured" @click="sendTest"
            >Send test</UiButton
          >
        </template>
      </UiCard>
    </template>
  </SettingsPage>
</template>

<style scoped>
code {
  word-break: break-all;
}
.meta {
  margin: 0;
  color: var(--ui-on-surface-variant);
}
.text-success {
  color: var(--ui-success);
}
.text-danger {
  color: var(--ui-error);
}
</style>
