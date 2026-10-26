<script setup lang="ts">
import { reactive, ref } from 'vue'
import type { Member } from '../../../api/client'
import type { MailSourceView, MailVisibility } from '../../../api/mailSettings'
import { formatTime } from '../../../lib/time'
import { useMailSettingsStore } from '../../../stores/mailSettings'
import { describeError } from '../../shared'

const props = defineProps<{ members: Member[]; tz: string }>()
const store = useMailSettingsStore()
const error = ref('')
const note = reactive<Record<string, string>>({})
const editing = ref<string | null>(null)
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
  editing.value = editing.value === s.id ? null : s.id
  draft.name = s.name
  draft.owners = [...s.owners]
  draft.visibility = s.visibility
  draft.backfill = s.backfill_days
  draft.poll = s.poll_minutes
  draft.query = s.query ?? ''
  draft.folders = s.folders.join(', ')
  draft.password = ''
}

async function save(s: MailSourceView) {
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

const toggleReadOnly = (s: MailSourceView, v: boolean) =>
  run(() => store.patchSource(s, { read_only: v }), 'Could not save.')

async function syncNow(s: MailSourceView) {
  await run(async () => {
    await store.syncSource(s.id)
    note[s.id] = 'Sync queued.'
  }, 'Could not start a sync.')
}

async function archive(s: MailSourceView) {
  if (
    !confirm(
      `Archive "${s.name}"? Its mail is removed from the app (nothing changes in the mailbox).`,
    )
  )
    return
  await run(() => store.removeSource(s.id), 'Could not archive the source.')
}
</script>

<template>
  <div>
    <p v-if="error" class="settings-error" role="alert">{{ error }}</p>
    <ul class="settings-list">
      <li v-for="s in store.sources" :key="s.id" :data-testid="`mail-source-${s.name}`">
        <div class="head">
          <strong>{{ s.name }}</strong>
          <span class="badge" :class="s.state" data-testid="state">{{ s.state }}</span>
          <span class="settings-note">
            {{ s.provider }} · {{ s.address }} · owners {{ ownerNames(s) }} · {{ s.visibility }} ·
            last sync {{ synced(s) }} · {{ s.message_count }} messages
          </span>
        </div>
        <p v-if="s.last_error" class="settings-error">{{ s.last_error }}</p>
        <div v-if="s.can_manage" class="actions">
          <label class="inline">
            <input
              type="checkbox"
              :checked="s.read_only"
              @change="toggleReadOnly(s, ($event.target as HTMLInputElement).checked)"
            />
            Read only
          </label>
          <button type="button" @click="syncNow(s)">Sync now</button>
          <button v-if="s.state !== 'paused'" type="button" @click="store.pause(s)">Pause</button>
          <button v-else type="button" @click="store.resume(s)">Resume</button>
          <button type="button" @click="startEdit(s)">Edit</button>
          <button type="button" @click="archive(s)">Archive</button>
          <span v-if="note[s.id]" class="settings-ok" role="status">{{ note[s.id] }}</span>
        </div>
        <form v-if="editing === s.id" class="settings-form" @submit.prevent="save(s)">
          <label>Name <input v-model="draft.name" name="name" required /></label>
          <fieldset>
            <legend>Owners</legend>
            <label
              v-for="m in members.filter((x) => x.role === 'adult')"
              :key="m.id"
              class="inline"
            >
              <input v-model="draft.owners" type="checkbox" :value="m.id" />
              {{ m.display_name }}
            </label>
          </fieldset>
          <label>
            Who can see this mail
            <select v-model="draft.visibility" name="visibility">
              <option value="owners">Owners only</option>
              <option value="household">All adults</option>
            </select>
          </label>
          <label>
            Days of history
            <input v-model.number="draft.backfill" type="number" min="1" max="365" />
          </label>
          <label>
            Check every (minutes)
            <input v-model.number="draft.poll" type="number" min="1" max="1440" />
          </label>
          <label>Filter <input v-model="draft.query" name="query" /></label>
          <template v-if="s.provider === 'imap'">
            <label
              >Folders (comma separated) <input v-model="draft.folders" name="folders"
            /></label>
            <label>
              New app password (leave empty to keep)
              <input v-model="draft.password" type="password" autocomplete="off" />
            </label>
          </template>
          <div>
            <button type="submit">Save</button>
            <button type="button" @click="editing = null">Cancel</button>
          </div>
        </form>
      </li>
    </ul>
    <p v-if="!store.sources.length && store.loaded" class="settings-note">No mail sources yet.</p>
  </div>
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
.inline {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
</style>
