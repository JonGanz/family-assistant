<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { api } from '../api/client'
import { mailApi, type CalendarViewExt, type SourceViewExt } from '../api/mailSettings'
import { ApiError } from '../api/http'
import ErrorBanner from '../components/ErrorBanner.vue'
import { formatTime } from '../lib/time'
import { useHouseholdStore } from '../stores/household'
import { useMembersStore } from '../stores/members'
import { useSourcesStore } from '../stores/sources'
import AddSourceForm from './parts/AddSourceForm.vue'
import SourceCredentials from './parts/SourceCredentials.vue'
import { describeError } from './shared'
import { reactive, ref } from 'vue'
import './parts/settings.css'

const sources = useSourcesStore()
const household = useHouseholdStore()
const members = useMembersStore()
const error = ref('')
const note = reactive<Record<string, string>>({})
const needsWriteAccess = reactive<Record<string, boolean>>({})
const editing = ref<string | null>(null)

const googleEnabled = computed(() => household.view?.google_oauth_enabled ?? false)
const calendarsOf = (s: SourceViewExt) => sources.calendars.filter((c) => c.source === s.id)

onMounted(() => {
  void sources.load()
  void members.load()
  if (!household.loaded) void household.load()
})

async function run(fn: () => Promise<unknown>, fallback: string) {
  error.value = ''
  try {
    await fn()
  } catch (e) {
    error.value = describeError(e, fallback)
  }
}

async function syncNow(s: SourceViewExt) {
  await run(async () => {
    await sources.sync(s.id)
    note[s.id] = 'Sync queued.'
  }, 'Could not start a sync.')
}

async function remove(s: SourceViewExt) {
  if (!confirm(`Remove "${s.name}" and its events?`)) return
  await run(() => sources.remove(s.id), 'Could not remove the source.')
}

const toggleEnabled = (c: CalendarViewExt, v: boolean) =>
  run(() => sources.patchCalendar(c, { enabled: v }), 'Could not save.')
const setColor = (c: CalendarViewExt, v: string) =>
  run(() => sources.patchCalendar(c, { color: v }), 'Could not save.')
function toggleMember(c: CalendarViewExt, id: string, on: boolean) {
  const next = on ? [...c.members, id] : c.members.filter((m) => m !== id)
  return run(() => sources.patchCalendar(c, { members: next }), 'Could not save.')
}
const googleReconnect = () => api.googleStartUrl('/settings/calendars')
const connectWrite = () => mailApi.googleStartUrl('calendar_write', '/settings/calendars')
const toggleDefault = (c: CalendarViewExt, v: boolean) =>
  run(() => sources.patchCalendar(c, { is_default_target: v }), 'Could not save.')
async function toggleWritable(s: SourceViewExt, v: boolean) {
  await run(async () => {
    try {
      await sources.patchSource(s, { writable: v })
    } catch (e) {
      if (e instanceof ApiError && e.status === 422) needsWriteAccess[s.id] = true
      else throw e
    }
  }, 'Could not save.')
}
const synced = (s: SourceViewExt) =>
  s.last_synced_at ? formatTime(s.last_synced_at, household.tz) : 'never'
</script>

<template>
  <section>
    <h1>Calendars</h1>
    <ErrorBanner v-if="sources.error" :message="sources.error" />
    <ErrorBanner v-if="sources.conflict" conflict @reload="sources.load()" />
    <ErrorBanner v-if="error" :message="error" />

    <ul class="settings-list">
      <li v-for="s in sources.sources" :key="s.id" :data-testid="`source-${s.name}`">
        <div class="head">
          <strong>{{ s.name }}</strong>
          <span class="badge" :class="s.state" data-testid="state">{{ s.state }}</span>
          <span class="settings-note">
            {{ s.kind }}<template v-if="s.masked_url"> · {{ s.masked_url }}</template> · last sync
            {{ synced(s) }}
          </span>
        </div>
        <p v-if="s.last_error" class="settings-error">{{ s.last_error }}</p>
        <div class="actions">
          <button type="button" @click="syncNow(s)">Sync now</button>
          <a
            v-if="s.kind === 'google' && s.state === 'needs_reauth'"
            class="button"
            :href="googleReconnect()"
          >
            Reconnect
          </a>
          <button
            v-else-if="s.kind !== 'google'"
            type="button"
            @click="editing = editing === s.id ? null : s.id"
          >
            {{ s.state === 'needs_reauth' ? 'Reconnect' : 'Change link or password' }}
          </button>
          <button type="button" @click="remove(s)">Remove</button>
          <span v-if="note[s.id]" class="settings-ok" role="status">{{ note[s.id] }}</span>
        </div>
        <div v-if="s.kind !== 'ics'" class="actions">
          <label class="inline">
            <input
              type="checkbox"
              :checked="s.writable"
              :aria-label="`Allow writing approved events to ${s.name}`"
              @change="toggleWritable(s, ($event.target as HTMLInputElement).checked)"
            />
            Allow writing (approved events)
          </label>
          <a
            v-if="s.kind === 'google' && (needsWriteAccess[s.id] || (!s.writable && googleEnabled))"
            class="button"
            :href="connectWrite()"
          >
            Connect for writing
          </a>
          <span v-if="needsWriteAccess[s.id]" class="settings-error" role="alert">
            Reconnect with calendar write access first.
          </span>
        </div>
        <SourceCredentials v-if="editing === s.id" :source="s" @done="editing = null" />

        <ul class="calendars">
          <li v-for="c in calendarsOf(s)" :key="c.id">
            <label class="inline">
              <input
                type="checkbox"
                :checked="c.enabled"
                :aria-label="`Show ${c.name}`"
                @change="toggleEnabled(c, ($event.target as HTMLInputElement).checked)"
              />
              <strong>{{ c.name }}</strong>
            </label>
            <label v-if="s.writable" class="inline">
              <input
                type="checkbox"
                :checked="c.is_default_target"
                @change="toggleDefault(c, ($event.target as HTMLInputElement).checked)"
              />
              Default target
            </label>
            <label class="inline">
              Colour
              <input
                type="color"
                :value="c.color ?? '#2f6fed'"
                @change="setColor(c, ($event.target as HTMLInputElement).value)"
              />
            </label>
            <fieldset class="who">
              <legend>Whose calendar (none means the whole household)</legend>
              <label v-for="m in members.items" :key="m.id" class="inline">
                <input
                  type="checkbox"
                  :checked="c.members.includes(m.id)"
                  @change="toggleMember(c, m.id, ($event.target as HTMLInputElement).checked)"
                />
                {{ m.display_name }}
              </label>
            </fieldset>
          </li>
          <li v-if="!calendarsOf(s).length" class="settings-note">No calendars found yet.</li>
        </ul>
      </li>
    </ul>
    <p v-if="!sources.sources.length && sources.loaded" class="settings-note">
      No calendar sources yet.
    </p>

    <AddSourceForm
      :google-enabled="googleEnabled"
      :accounts="sources.accounts"
      :tz="household.tz"
      return-to="/settings/calendars"
    />
  </section>
</template>

<style scoped>
.head,
.actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
  margin-bottom: var(--space-2);
}
.calendars {
  list-style: none;
  padding: 0;
  margin: var(--space-3) 0 0;
  display: grid;
  gap: var(--space-2);
}
.calendars > li {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-4);
}
.inline {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.who {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
  border: 0;
  padding: 0;
}
.who legend {
  font-size: var(--font-size-sm);
  color: var(--color-muted);
}
.button {
  padding: 0.1rem 0.5rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  text-decoration: none;
}
</style>
