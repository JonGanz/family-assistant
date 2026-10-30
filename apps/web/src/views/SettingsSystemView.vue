<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from 'vue'
import { api, type SettingView, type SystemSettingsView } from '../api/client'
import { ApiError, problemCode } from '../api/http'
import ErrorBanner from '../components/ErrorBanner.vue'
import PageHeader from '../components/PageHeader.vue'
import { describeError } from './shared'
import MasterKeyBackup from './system/MasterKeyBackup.vue'
import { buildPatch, connectionText, draftFrom, metaFor, type Draft } from './system/settingsModel'

const view = ref<SystemSettingsView | null>(null)
const drafts = reactive<Record<string, Draft>>({})
const errors = reactive<Record<string, string>>({})
const loadError = ref('')
const error = ref('')
const message = ref('')
const applying = ref(false)
const restartNote = ref(false)
const backupOpen = ref(false)
let timer: ReturnType<typeof setTimeout> | undefined
let gone = false

const settings = computed(() => view.value?.settings ?? [])
const needsBackup = computed(() => !!view.value && !view.value.key_backup_confirmed_at)

function fill(v: SystemSettingsView) {
  view.value = v
  for (const s of v.settings) drafts[s.key] = draftFrom(s)
}

async function load() {
  try {
    fill(await api.systemSettings())
    loadError.value = ''
  } catch (e) {
    loadError.value = describeError(e, 'Could not load the system settings.')
  }
}
onMounted(load)
onUnmounted(() => {
  gone = true
  clearTimeout(timer)
})

const hint = (s: SettingView) =>
  s.locked ? 'Set by the server (an environment variable), so it cannot be changed here.' : ''

/** After a save the server re-reads its settings; wait until it answers /readyz again. */
async function waitReady(tries = 20): Promise<void> {
  for (let i = 0; i < tries && !gone; i++) {
    try {
      await api.readyz()
      return
    } catch {
      await new Promise((r) => (timer = setTimeout(r, 1000)))
    }
  }
}

async function save() {
  error.value = ''
  message.value = ''
  for (const k of Object.keys(errors)) delete errors[k]
  const patch = buildPatch(settings.value, drafts)
  if (Object.keys(patch).length === 0) {
    message.value = 'Nothing changed.'
    return
  }
  applying.value = true
  try {
    const next = await api.patchSystemSettings(patch as never)
    restartNote.value = Object.keys(patch).some((k) => metaFor(k).restart)
    await waitReady()
    fill(next)
    message.value = 'Saved.'
  } catch (e) {
    const code = problemCode(e)
    if (code === 'invalid_setting' && e instanceof ApiError) {
      const field = e.problem.field
      if (typeof field === 'string') errors[field] = e.message
      else error.value = e.message
    } else if (code === 'setting_locked') {
      error.value = 'One of those settings is set by the server and cannot be changed here.'
      await load()
    } else {
      error.value = describeError(e, 'Save failed.')
    }
  } finally {
    applying.value = false
  }
}

function backedUp() {
  backupOpen.value = false
  void load()
}
</script>

<template>
  <section aria-labelledby="page-title">
    <PageHeader eyebrow="System" title="System" />
    <ErrorBanner v-if="loadError" :message="loadError" />

    <template v-if="view">
      <section class="card" aria-label="Connection" data-testid="connection">
        <h2>Connection</h2>
        <p data-testid="connection-text">{{ connectionText(view.connection) }}</p>
      </section>

      <section
        v-if="needsBackup || backupOpen"
        class="card backup"
        aria-label="Master key backup"
        data-testid="backup-warning"
      >
        <h2>Master key backup</h2>
        <template v-if="!backupOpen">
          <p>
            You have not confirmed a backup of the master key yet. Without it, stored secrets are
            lost if this server's disk is.
          </p>
          <button type="button" class="btn-primary" @click="backupOpen = true">Back up now</button>
        </template>
        <MasterKeyBackup v-else @confirmed="backedUp" />
      </section>
      <p v-else class="muted" data-testid="backup-done">
        Master key backup confirmed.
        <button type="button" class="btn-sm" @click="backupOpen = true">Show the key again</button>
      </p>

      <form class="form form-wide" aria-label="Runtime settings" @submit.prevent="save">
        <h2>Runtime settings</h2>
        <div v-for="s in settings" :key="s.key" class="setting" :data-testid="`setting-${s.key}`">
          <template v-if="drafts[s.key]">
            <label v-if="s.kind === 'bool'" class="field-inline">
              <input
                v-model="drafts[s.key]!.value"
                type="checkbox"
                :name="s.key"
                :disabled="s.locked || applying"
              />
              {{ metaFor(s.key).label }}
            </label>
            <label v-else-if="s.kind === 'cidrs'" class="field">
              {{ metaFor(s.key).label }}
              <textarea
                v-model="drafts[s.key]!.value as string"
                :name="s.key"
                rows="3"
                :disabled="s.locked || applying"
              />
            </label>
            <template v-else-if="s.kind === 'secret'">
              <label class="field">
                {{ metaFor(s.key).label }}
                <span class="muted" :data-testid="`secret-state-${s.key}`">{{
                  s.is_set ? 'set' : 'not set'
                }}</span>
                <input
                  v-model="drafts[s.key]!.value as string"
                  :name="s.key"
                  type="password"
                  autocomplete="new-password"
                  :placeholder="s.is_set ? 'Type to replace' : 'Not set'"
                  :disabled="s.locked || applying"
                />
              </label>
              <label v-if="s.is_set && !s.locked" class="field-inline">
                <input v-model="drafts[s.key]!.clear" type="checkbox" :disabled="applying" />
                Clear the stored value
              </label>
            </template>
            <label v-else class="field">
              {{ metaFor(s.key).label }}
              <input
                v-model="drafts[s.key]!.value as string"
                :name="s.key"
                :type="s.kind === 'url' ? 'url' : 'text'"
                :disabled="s.locked || applying"
              />
            </label>
            <p v-if="metaFor(s.key).help" class="muted">{{ metaFor(s.key).help }}</p>
            <p v-if="metaFor(s.key).restart && !s.locked" class="muted">
              A change here needs a server restart to take effect.
            </p>
            <p v-if="s.locked" class="muted" data-testid="locked-note">{{ hint(s) }}</p>
            <p v-if="errors[s.key]" class="text-danger" role="alert">{{ errors[s.key] }}</p>
          </template>
        </div>
        <ErrorBanner v-if="error" :message="error" />
        <p v-if="applying" role="status" data-testid="applying">Applying…</p>
        <p v-else-if="message" class="text-success" role="status">{{ message }}</p>
        <p v-if="restartNote && !applying" class="muted">
          Some of these changes take effect after the server restarts.
        </p>
        <button type="submit" class="btn-primary" :disabled="applying">Save</button>
      </form>
    </template>
  </section>
</template>

<style scoped>
.card {
  margin-bottom: var(--space-4);
}
.setting {
  display: grid;
  gap: var(--space-1);
}
.backup {
  border-color: var(--color-warning);
}
</style>
