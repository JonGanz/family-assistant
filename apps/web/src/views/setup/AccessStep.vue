<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { api, type SettingView } from '../../api/client'
import ErrorBanner from '../../components/ErrorBanner.vue'
import { useSessionStore } from '../../stores/session'
import { useSetupStore } from '../../stores/setup'
import { describeError } from '../shared'
import { buildPatch, draftFrom, metaFor, type Draft } from '../system/settingsModel'
import StepButtons from './StepButtons.vue'

const setup = useSetupStore()
const session = useSessionStore()
const KEYS = ['public_base_url', 'tailnet_base_url', 'ntfy_url']

const all = ref<SettingView[]>([])
const drafts = reactive<Record<string, Draft>>({})
const error = ref('')
const note = ref('')
const busy = ref(false)

const shown = computed(() =>
  all.value.filter(
    (s) =>
      KEYS.includes(s.key) &&
      (s.key !== 'tailnet_base_url' || setup.status?.via_tailscale || s.is_set),
  ),
)

onMounted(async () => {
  try {
    all.value = (await api.systemSettings()).settings
    for (const s of all.value) drafts[s.key] = draftFrom(s)
  } catch (e) {
    error.value = describeError(e, 'Could not load the settings.')
  }
})

async function persist(): Promise<boolean> {
  error.value = ''
  const patch = buildPatch(shown.value, drafts)
  if (!Object.keys(patch).length) return true
  try {
    const next = await api.patchSystemSettings(patch as never)
    all.value = next.settings
    for (const s of next.settings) drafts[s.key] = draftFrom(s)
    return true
  } catch (e) {
    error.value = describeError(e, 'Could not save.')
    return false
  }
}

async function sendTest() {
  note.value = ''
  const id = session.member?.id
  if (!id || !(await persist())) return
  try {
    await api.testNotify(id)
    note.value = 'Test sent. Check your phone.'
  } catch (e) {
    error.value = describeError(e, 'The test notification failed.')
  }
}

async function next() {
  busy.value = true
  if (await persist()) setup.goto(6)
  busy.value = false
}
</script>

<template>
  <form class="form" data-testid="step-access" @submit.prevent="next">
    <h2>Access and notifications</h2>
    <div v-for="s in shown" :key="s.key">
      <label v-if="drafts[s.key]" class="field">
        {{ metaFor(s.key).label }}
        <input
          v-model="drafts[s.key]!.value as string"
          :name="s.key"
          :type="s.kind === 'url' ? 'url' : 'text'"
          :disabled="s.locked"
        />
      </label>
      <p class="muted">{{ metaFor(s.key).help }}</p>
      <p v-if="s.locked" class="muted">Set by the server, so it cannot be changed here.</p>
    </div>
    <p class="muted">Leave the notifications server empty to keep notifications off.</p>
    <div class="row">
      <button type="button" @click="sendTest">Send a test</button>
      <span v-if="note" class="text-success" role="status">{{ note }}</span>
    </div>
    <ErrorBanner v-if="error" :message="error" />
    <StepButtons :busy="busy" @back="setup.goto(4)" />
  </form>
</template>
