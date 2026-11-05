<script setup lang="ts">
// System settings (D-052, D-075). Runtime values are one draft with one Save bar: saving makes the
// server re-read its settings, so the page waits for /readyz and then shows the stored state.
import { computed, onMounted, onUnmounted, reactive, ref } from 'vue'
import { api, type SettingView, type SystemSettingsView } from '../api/client'
import { ApiError, problemCode } from '../api/http'
import SettingsPage from '../app/SettingsPage.vue'
import {
  UiButton,
  UiCard,
  UiCheckbox,
  UiChip,
  UiErrorState,
  UiIcon,
  UiSaveBar,
  UiSkeleton,
  UiTextField,
  useFormDraft,
} from '../ui'
import { describeError } from './shared'
import MasterKeyBackup from './system/MasterKeyBackup.vue'
import {
  buildPatch,
  connectionText,
  draftFrom,
  groupSettings,
  metaFor,
  type Draft,
} from './system/settingsModel'

const view = ref<SystemSettingsView | null>(null)
const errors = reactive<Record<string, string>>({})
const loadError = ref('')
const message = ref('')
const restartNote = ref(false)
const backupOpen = ref(false)
let timer: ReturnType<typeof setTimeout> | undefined
let gone = false

const settings = computed(() => view.value?.settings ?? [])
const groups = computed(() => groupSettings(settings.value))
const needsBackup = computed(() => !!view.value && !view.value.key_backup_confirmed_at)

const form = useFormDraft<Record<string, Draft>>(
  () => Object.fromEntries(settings.value.map((s) => [s.key, draftFrom(s)])),
  async (drafts) => {
    message.value = ''
    for (const k of Object.keys(errors)) delete errors[k]
    const patch = buildPatch(settings.value, drafts)
    try {
      const next = await api.patchSystemSettings(patch as never)
      restartNote.value = Object.keys(patch).some((k) => metaFor(k).restart)
      await waitReady()
      view.value = next
      message.value = 'Saved.'
    } catch (e) {
      const code = problemCode(e)
      if (code === 'invalid_setting' && e instanceof ApiError) {
        const field = e.problem.field
        if (typeof field === 'string') errors[field] = e.message
        throw new Error(e.message, { cause: e })
      }
      if (code === 'setting_locked') {
        await load()
        throw new Error('One of those settings is set by the server and cannot be changed here.', {
          cause: e,
        })
      }
      throw new Error(describeError(e, 'Save failed.'), { cause: e })
    }
  },
)
const drafts = form.draft

async function load() {
  try {
    view.value = await api.systemSettings()
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
const field = (key: string) => drafts.value[key]!

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

function backedUp() {
  backupOpen.value = false
  void load()
}
</script>

<template>
  <SettingsPage title="System">
    <UiErrorState v-if="loadError" :message="loadError" @retry="load" />
    <UiSkeleton v-else-if="!view" :lines="5" />

    <template v-if="view">
      <UiCard variant="outlined" aria-label="Connection" data-testid="connection">
        <template #header>Connection</template>
        <p data-testid="connection-text">{{ connectionText(view.connection) }}</p>
      </UiCard>

      <UiCard
        v-if="needsBackup || backupOpen"
        variant="outlined"
        aria-label="Master key backup"
        data-testid="backup-warning"
      >
        <template #header
          ><span class="warn"><UiIcon name="warning" /> Master key backup</span></template
        >
        <template v-if="!backupOpen">
          <p>
            You have not confirmed a backup of the master key yet. Without it, stored secrets are
            lost if this server's disk is.
          </p>
        </template>
        <MasterKeyBackup v-else @confirmed="backedUp" />
        <template v-if="!backupOpen" #actions>
          <UiButton @click="backupOpen = true">Back up now</UiButton>
        </template>
      </UiCard>
      <UiCard v-else data-testid="backup-done">
        <p>Master key backup confirmed.</p>
        <template #actions>
          <UiButton variant="text" @click="backupOpen = true">Show the key again</UiButton>
        </template>
      </UiCard>

      <form class="runtime" aria-label="Runtime settings" @submit.prevent="form.save()">
        <UiCard v-for="g in groups" :key="g.title">
          <template #header>{{ g.title }}</template>
          <div v-for="s in g.items" :key="s.key" class="setting" :data-testid="`setting-${s.key}`">
            <template v-if="drafts[s.key]">
              <UiCheckbox
                v-if="s.kind === 'bool'"
                :model-value="field(s.key).value as boolean"
                :label="metaFor(s.key).label"
                :disabled="s.locked || form.saving.value"
                @update:model-value="field(s.key).value = $event"
              />
              <UiTextField
                v-else-if="s.kind === 'cidrs'"
                :model-value="field(s.key).value as string"
                :label="metaFor(s.key).label"
                :name="s.key"
                multiline
                :disabled="s.locked || form.saving.value"
                :error="errors[s.key]"
                @update:model-value="field(s.key).value = $event"
              />
              <template v-else-if="s.kind === 'secret'">
                <div class="secret">
                  <span>{{ metaFor(s.key).label }}</span>
                  <UiChip
                    :icon="s.is_set ? 'success' : 'info'"
                    :data-testid="`secret-state-${s.key}`"
                    >{{ s.is_set ? 'set' : 'not set' }}</UiChip
                  >
                </div>
                <UiTextField
                  :model-value="field(s.key).value as string"
                  :label="s.is_set ? 'Replace with' : 'Value'"
                  :name="s.key"
                  type="password"
                  autocomplete="new-password"
                  :placeholder="s.is_set ? 'Type to replace' : 'Not set'"
                  :disabled="s.locked || form.saving.value"
                  :error="errors[s.key]"
                  @update:model-value="field(s.key).value = $event"
                />
                <UiCheckbox
                  v-if="s.is_set && !s.locked"
                  :model-value="field(s.key).clear"
                  label="Clear the stored value"
                  :disabled="form.saving.value"
                  @update:model-value="field(s.key).clear = $event"
                />
              </template>
              <UiTextField
                v-else
                :model-value="field(s.key).value as string"
                :label="metaFor(s.key).label"
                :name="s.key"
                :type="s.kind === 'url' ? 'url' : 'text'"
                :disabled="s.locked || form.saving.value"
                :error="errors[s.key]"
                @update:model-value="field(s.key).value = $event"
              />
              <p v-if="metaFor(s.key).help" class="note">{{ metaFor(s.key).help }}</p>
              <p v-if="metaFor(s.key).restart && !s.locked" class="note">
                A change here needs a server restart to take effect.
              </p>
              <p v-if="s.locked" class="note" data-testid="locked-note">
                <UiIcon name="lock" :size="20" /> {{ hint(s) }}
              </p>
            </template>
          </div>
        </UiCard>
        <p v-if="form.saving.value" role="status" data-testid="applying">Applying…</p>
        <p v-else-if="message" role="status">{{ message }}</p>
        <p v-if="restartNote && !form.saving.value" class="note">
          Some of these changes take effect after the server restarts.
        </p>
        <UiSaveBar
          :dirty="form.dirty.value"
          :saving="form.saving.value"
          :error="form.error.value"
          @save="form.save()"
          @discard="form.discard()"
        />
      </form>
    </template>
  </SettingsPage>
</template>

<style scoped>
.runtime {
  display: grid;
  gap: var(--ui-space-3);
}
.setting {
  display: grid;
  gap: var(--ui-space-2);
}
.secret {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
}
.note {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
.warn {
  display: inline-flex;
  align-items: center;
  gap: var(--ui-space-2);
  color: var(--ui-warning);
}
</style>
