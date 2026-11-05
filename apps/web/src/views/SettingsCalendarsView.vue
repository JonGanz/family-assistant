<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { api } from '../api/client'
import { ApiError } from '../api/http'
import { mailApi, type CalendarViewExt, type SourceViewExt } from '../api/mailSettings'
import ResponsiveItems from '../app/ResponsiveItems.vue'
import SettingsPage from '../app/SettingsPage.vue'
import { formatTime } from '../lib/time'
import { useHouseholdStore } from '../stores/household'
import { useMembersStore } from '../stores/members'
import { useSourcesStore } from '../stores/sources'
import {
  UiButton,
  UiChip,
  UiEmptyState,
  UiIconButton,
  UiMenu,
  UiSheet,
  UiSkeleton,
  type MenuItem,
} from '../ui'
import AddSourceForm from './parts/AddSourceForm.vue'
import ConfirmDialog from './parts/settings-mail/ConfirmDialog.vue'
import InstantCheck from './parts/settings-mail/InstantCheck.vue'
import PageFab from './parts/settings-mail/PageFab.vue'
import PageNotices from './parts/settings-mail/PageNotices.vue'
import { stateIcon } from './parts/settings-mail/stateIcon'
import { useConfirm } from './parts/settings-mail/useConfirm'
import SourceCredentials from './parts/SourceCredentials.vue'
import { describeError } from './shared'

const sources = useSourcesStore()
const household = useHouseholdStore()
const members = useMembersStore()
const confirm = useConfirm()
const error = ref('')
const note = reactive<Record<string, string>>({})
const needsWriteAccess = reactive<Record<string, boolean>>({})
const editing = ref<SourceViewExt | null>(null)
const adding = ref(false)

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
    return await fn()
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

const patchCal = (c: CalendarViewExt, p: Parameters<typeof sources.patchCalendar>[1]) =>
  sources.patchCalendar(c, p)
function memberPatch(c: CalendarViewExt, id: string, on: boolean) {
  const next = on ? [...c.members, id] : c.members.filter((m) => m !== id)
  return patchCal(c, { members: next })
}
const go = (url: string) => (window.location.href = url)
const connectWrite = () => go(mailApi.googleStartUrl('calendar_write', '/settings/calendars'))

async function setWritable(s: SourceViewExt, v: boolean): Promise<boolean> {
  try {
    return await sources.patchSource(s, { writable: v })
  } catch (e) {
    if (e instanceof ApiError && e.status === 422) {
      needsWriteAccess[s.id] = true
      throw new Error('Reconnect with calendar write access first.', { cause: e })
    }
    throw e
  }
}

const synced = (s: SourceViewExt) =>
  s.last_synced_at ? formatTime(s.last_synced_at, household.tz) : 'never'

function menuItems(s: SourceViewExt): MenuItem[] {
  const items: MenuItem[] = [{ id: 'sync', label: 'Sync now' }]
  if (s.kind === 'google') {
    if (s.state === 'needs_reauth') items.push({ id: 'reconnect', label: 'Reconnect' })
  } else {
    items.push({
      id: 'credentials',
      label: s.state === 'needs_reauth' ? 'Reconnect' : 'Change link or password',
    })
  }
  items.push({ id: 'remove', label: 'Remove', danger: true })
  return items
}
function onMenu(s: SourceViewExt, id: string) {
  if (id === 'sync') void syncNow(s)
  else if (id === 'credentials') editing.value = s
  else if (id === 'reconnect') go(api.googleStartUrl('/settings/calendars'))
  else if (id === 'remove')
    confirm.ask(
      `Remove “${s.name}”?`,
      'Its events are removed from the app too.',
      () => run(() => sources.remove(s.id), 'Could not remove the source.'),
      'Remove',
    )
}
</script>

<template>
  <SettingsPage title="Calendars">
    <PageNotices
      :load-error="sources.error"
      :conflict="sources.conflict"
      :error="error"
      @reload="sources.load()"
    />
    <UiSkeleton v-if="!sources.loaded && !sources.error" :lines="4" />
    <UiEmptyState
      v-else-if="sources.loaded && !sources.sources.length"
      icon="calendar"
      title="No calendar sources yet."
      text="Add a Google, CalDAV or ICS calendar."
    >
      <UiButton icon="add" @click="adding = true">Add a source</UiButton>
    </UiEmptyState>

    <ResponsiveItems :items="sources.sources">
      <template #item="{ item: s }">
        <div class="src" :data-testid="`source-${s.name}`">
          <div class="src__head">
            <strong class="src__name">{{ s.name }}</strong>
            <UiChip :icon="stateIcon(s.state)" data-testid="state">{{ s.state }}</UiChip>
            <UiMenu
              :items="menuItems(s)"
              :label="`Actions for ${s.name}`"
              @select="onMenu(s, $event)"
            >
              <template #activator="{ toggle, attrs }">
                <UiIconButton
                  icon="more"
                  :label="`Actions for ${s.name}`"
                  v-bind="attrs"
                  @click="toggle"
                />
              </template>
            </UiMenu>
          </div>
          <p class="src__meta">
            {{ s.kind }}<template v-if="s.masked_url"> · {{ s.masked_url }}</template> · last sync
            {{ synced(s) }}
          </p>
          <p v-if="s.last_error" class="src__error">{{ s.last_error }}</p>
          <p v-if="note[s.id]" role="status">{{ note[s.id] }}</p>

          <template v-if="s.kind !== 'ics'">
            <InstantCheck
              :model-value="s.writable"
              :label="`Allow writing approved events to ${s.name}`"
              :save="(v: boolean) => setWritable(s, v)"
            />
            <UiButton
              v-if="
                s.kind === 'google' && (needsWriteAccess[s.id] || (!s.writable && googleEnabled))
              "
              variant="tonal"
              @click="connectWrite"
              >Connect for writing</UiButton
            >
            <p v-if="needsWriteAccess[s.id]" role="alert">
              Reconnect with calendar write access first.
            </p>
          </template>

          <section v-for="c in calendarsOf(s)" :key="c.id" class="cal">
            <InstantCheck
              :model-value="c.enabled"
              :label="`Show ${c.name}`"
              :save="(v: boolean) => patchCal(c, { enabled: v })"
            />
            <InstantCheck
              v-if="s.writable"
              :model-value="c.is_default_target"
              :label="`Default target for ${c.name}`"
              :save="(v: boolean) => patchCal(c, { is_default_target: v })"
            />
            <fieldset class="who">
              <legend>Whose calendar (none means the whole household)</legend>
              <InstantCheck
                v-for="m in members.items"
                :key="m.id"
                :model-value="c.members.includes(m.id)"
                :label="m.display_name"
                :save="(v: boolean) => memberPatch(c, m.id, v)"
              />
            </fieldset>
          </section>
          <p v-if="!calendarsOf(s).length" class="src__meta">No calendars found yet.</p>
        </div>
      </template>
    </ResponsiveItems>

    <PageFab label="Add a source" @click="adding = true" />
    <UiSheet v-model="adding" title="Add a calendar source">
      <AddSourceForm
        :google-enabled="googleEnabled"
        :accounts="sources.accounts"
        :tz="household.tz"
        return-to="/settings/calendars"
        @created="adding = false"
      />
    </UiSheet>
    <SourceCredentials
      v-if="editing"
      :model-value="true"
      :source="editing"
      @update:model-value="editing = null"
      @done="editing = null"
    />
    <ConfirmDialog
      :pending="confirm.pending.value"
      @accept="confirm.accept"
      @cancel="confirm.cancel"
    />
  </SettingsPage>
</template>

<style scoped>
.src {
  display: grid;
  gap: var(--ui-space-2);
}
.src__head {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
}
.src__name {
  flex: 1;
  font: var(--ui-type-title-md);
}
.src__meta {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
.src__error {
  margin: 0;
  color: var(--ui-error);
}
.cal {
  display: grid;
  gap: var(--ui-space-1);
  padding-top: var(--ui-space-2);
  border-top: 1px solid var(--ui-outline-variant);
}
.who {
  display: grid;
  gap: var(--ui-space-1);
  border: 0;
  padding: 0;
  margin: 0;
}
.who legend {
  padding: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
</style>
