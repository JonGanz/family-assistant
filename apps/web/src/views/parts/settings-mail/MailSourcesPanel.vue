<script setup lang="ts">
import { reactive, ref } from 'vue'
import ResponsiveItems from '../../../app/ResponsiveItems.vue'
import type { Member } from '../../../api/client'
import type { MailSourceView, MailVisibility } from '../../../api/mailSettings'
import { formatTime } from '../../../lib/time'
import { useMailSettingsStore } from '../../../stores/mailSettings'
import {
  UiButton,
  UiCard,
  UiCheckbox,
  UiChip,
  UiEmptyState,
  UiIconButton,
  UiMenu,
  UiSelect,
  UiTextField,
  type MenuItem,
} from '../../../ui'
import { describeError } from '../../shared'
import ConfirmDialog from './ConfirmDialog.vue'
import InstantCheck from './InstantCheck.vue'
import SheetForm from './SheetForm.vue'
import { stateIcon } from './stateIcon'
import { toggleIn } from './toggle'
import { useConfirm } from './useConfirm'

const props = defineProps<{ members: Member[]; tz: string }>()
const store = useMailSettingsStore()
const error = ref('')
const note = reactive<Record<string, string>>({})
const editing = ref<MailSourceView | null>(null)
const confirm = useConfirm()
const draft = reactive({
  name: '',
  owners: [] as string[],
  visibility: 'owners' as MailVisibility,
  backfill: 30,
  poll: 5,
  query: '',
  folders: '',
  password: '',
})

const ownerNames = (s: MailSourceView) =>
  s.owners.map((id) => props.members.find((m) => m.id === id)?.display_name ?? '?').join(', ')
const synced = (s: MailSourceView) =>
  s.last_synced_at ? formatTime(s.last_synced_at, props.tz) : 'never'

async function run(fn: () => Promise<unknown>, fallback: string) {
  error.value = ''
  try {
    await fn()
  } catch (e) {
    error.value = describeError(e, fallback)
  }
}

function startEdit(s: MailSourceView) {
  editing.value = s
  draft.name = s.name
  draft.owners = [...s.owners]
  draft.visibility = s.visibility
  draft.backfill = s.backfill_days
  draft.poll = s.poll_minutes
  draft.query = s.query ?? ''
  draft.folders = s.folders.join(', ')
  draft.password = ''
}

async function save() {
  const s = editing.value
  if (!s) return
  await run(async () => {
    const imap =
      s.provider === 'imap' && s.imap_host && s.imap_username
        ? {
            host: s.imap_host,
            port: s.imap_port ?? 993,
            username: s.imap_username,
            password: draft.password || null,
            folders: draft.folders
              .split(',')
              .map((x) => x.trim())
              .filter(Boolean),
            archive_folder: s.archive_folder ?? null,
          }
        : null
    const ok = await store.patchSource(s, {
      name: draft.name,
      owners: draft.owners,
      visibility: draft.visibility,
      backfill_days: draft.backfill,
      poll_minutes: draft.poll,
      query: draft.query.trim() || null,
      imap,
    })
    if (ok) editing.value = null
  }, 'Could not save the source.')
}

const setReadOnly = (s: MailSourceView, v: boolean) => store.patchSource(s, { read_only: v })

async function syncNow(s: MailSourceView) {
  await run(async () => {
    await store.syncSource(s.id)
    note[s.id] = 'Sync queued.'
  }, 'Could not start a sync.')
}

function archive(s: MailSourceView) {
  confirm.ask(
    `Archive “${s.name}”?`,
    'Its mail is removed from the app (nothing changes in the mailbox).',
    () => run(() => store.removeSource(s.id), 'Could not archive the source.'),
    'Archive',
  )
}

const menu = (): MenuItem[] => [
  { id: 'sync', label: 'Sync now' },
  { id: 'edit', label: 'Edit' },
  { id: 'archive', label: 'Archive', danger: true },
]
function onMenu(s: MailSourceView, id: string) {
  if (id === 'sync') void syncNow(s)
  else if (id === 'edit') startEdit(s)
  else if (id === 'archive') archive(s)
}
const visibilityOptions = [
  { value: 'owners', label: 'Owners only' },
  { value: 'household', label: 'All adults' },
]
</script>

<template>
  <div class="panel">
    <UiCard v-if="error" variant="outlined" role="alert">{{ error }}</UiCard>
    <ResponsiveItems :items="store.sources">
      <template #item="{ item: s }">
        <div class="src" :data-testid="`mail-source-${s.name}`">
          <div class="src__head">
            <strong class="src__name">{{ s.name }}</strong>
            <UiChip :icon="stateIcon(s.state)" data-testid="state">{{ s.state }}</UiChip>
            <UiMenu
              v-if="s.can_manage"
              :items="menu()"
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
            {{ s.provider }} · {{ s.address }} · owners {{ ownerNames(s) }} · {{ s.visibility }} ·
            last sync {{ synced(s) }} · {{ s.message_count }} messages
          </p>
          <p v-if="s.last_error" class="src__error">{{ s.last_error }}</p>
          <p v-if="note[s.id]" role="status">{{ note[s.id] }}</p>
          <template v-if="s.can_manage">
            <InstantCheck
              :model-value="s.read_only"
              :label="`Read only (${s.name})`"
              :save="(v: boolean) => setReadOnly(s, v)"
            />
            <div class="src__actions">
              <UiButton v-if="s.state !== 'paused'" variant="tonal" @click="store.pause(s)"
                >Pause</UiButton
              >
              <UiButton v-else variant="tonal" @click="store.resume(s)">Resume</UiButton>
            </div>
          </template>
        </div>
      </template>
    </ResponsiveItems>
    <UiEmptyState
      v-if="!store.sources.length && store.loaded"
      icon="mail"
      title="No mail sources yet."
    />

    <SheetForm
      :model-value="!!editing"
      :title="`Edit ${editing?.name ?? ''}`"
      @update:model-value="editing = null"
      @save="save"
    >
      <UiTextField v-model="draft.name" label="Name" name="name" required />
      <fieldset class="owners">
        <legend>Owners</legend>
        <UiCheckbox
          v-for="m in members.filter((x) => x.role === 'adult')"
          :key="m.id"
          :model-value="draft.owners.includes(m.id)"
          :label="m.display_name"
          @update:model-value="toggleIn(draft.owners, m.id, $event)"
        />
      </fieldset>
      <UiSelect
        :model-value="draft.visibility"
        label="Who can see this mail"
        :options="visibilityOptions"
        @update:model-value="draft.visibility = $event as MailVisibility"
      />
      <UiTextField
        :model-value="String(draft.backfill)"
        label="Days of history"
        type="number"
        min="1"
        max="365"
        @update:model-value="draft.backfill = Number($event)"
      />
      <UiTextField
        :model-value="String(draft.poll)"
        label="Check every (minutes)"
        type="number"
        min="1"
        max="1440"
        @update:model-value="draft.poll = Number($event)"
      />
      <UiTextField v-model="draft.query" label="Filter" name="query" />
      <template v-if="editing?.provider === 'imap'">
        <UiTextField v-model="draft.folders" label="Folders (comma separated)" name="folders" />
        <UiTextField
          v-model="draft.password"
          label="New app password (leave empty to keep)"
          type="password"
          autocomplete="off"
        />
      </template>
    </SheetForm>
    <ConfirmDialog
      :pending="confirm.pending.value"
      @accept="confirm.accept"
      @cancel="confirm.cancel"
    />
  </div>
</template>

<style scoped>
.panel,
.src {
  display: grid;
  gap: var(--ui-space-2);
}
.src__head,
.src__actions {
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
.owners {
  display: grid;
  gap: var(--ui-space-1);
  border: 0;
  padding: 0;
  margin: 0;
}
</style>
