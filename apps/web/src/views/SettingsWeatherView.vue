<script setup lang="ts">
import { onMounted, ref } from 'vue'
import type { WeatherRuleView } from '../api/client'
import ResponsiveItems from '../app/ResponsiveItems.vue'
import SettingsPage from '../app/SettingsPage.vue'
import { useRulesStore } from '../stores/rules'
import { UiButton, UiChip, UiEmptyState, UiSheet, UiSkeleton } from '../ui'
import ConfirmDialog from './parts/settings-mail/ConfirmDialog.vue'
import InstantCheck from './parts/settings-mail/InstantCheck.vue'
import PageFab from './parts/settings-mail/PageFab.vue'
import PageNotices from './parts/settings-mail/PageNotices.vue'
import { stateIcon } from './parts/settings-mail/stateIcon'
import { useConfirm } from './parts/settings-mail/useConfirm'
import RuleEditor from './parts/RuleEditor.vue'
import { describeError } from './shared'

const rules = useRulesStore()
const confirm = useConfirm()
/** `null` = closed, `'new'` = adding, otherwise the rule id being edited. */
const editing = ref<string | null>(null)
const error = ref('')

onMounted(() => rules.load())

const setActive = (r: WeatherRuleView, on: boolean) =>
  rules.patch(r, { status: on ? 'active' : 'disabled' })
const remove = (r: WeatherRuleView) =>
  confirm.ask(`Delete the rule “${r.name}”?`, 'Tasks it already made stay.', async () => {
    error.value = ''
    try {
      await rules.remove(r.id)
    } catch (e) {
      error.value = describeError(e, 'Could not delete.')
    }
  })
const severityIcon = (s: string) =>
  stateIcon(s === 'severe' ? 'error' : s === 'warning' ? 'warning' : 'ok')
const current = () => rules.items.find((r) => r.id === editing.value) ?? null
</script>

<template>
  <SettingsPage title="Weather">
    <PageNotices
      :load-error="rules.error"
      :conflict="rules.conflict && editing === null"
      :error="error"
      @reload="rules.load()"
    />
    <UiSkeleton v-if="!rules.loaded && !rules.error" :lines="3" />
    <UiEmptyState
      v-else-if="rules.loaded && !rules.items.length"
      icon="weather"
      title="No weather rules yet."
      text="A rule shows an advisory or makes a task when the forecast matches its condition."
    >
      <UiButton icon="add" @click="editing = 'new'">Add a rule</UiButton>
    </UiEmptyState>

    <ResponsiveItems :items="rules.items">
      <template #item="{ item: r }">
        <div class="rule" :data-testid="`rule-${r.name}`">
          <div class="rule__head">
            <strong class="rule__name">{{ r.name }}</strong>
            <UiChip :icon="severityIcon(r.severity)">{{ r.severity }}</UiChip>
          </div>
          <code>{{ r.condition }}</code>
          <p class="rule__msg">{{ r.message }}</p>
          <InstantCheck
            as="switch"
            :model-value="r.status === 'active'"
            :label="`Rule ${r.name} is on`"
            :save="(v: boolean) => setActive(r, v)"
          />
          <div class="rule__row">
            <UiButton variant="text" @click="editing = r.id">Edit</UiButton>
            <UiButton variant="text" @click="remove(r)">Delete</UiButton>
          </div>
        </div>
      </template>
    </ResponsiveItems>

    <PageFab label="Add a rule" @click="editing = 'new'" />
    <UiSheet
      :model-value="editing !== null"
      :title="editing === 'new' ? 'New weather rule' : 'Edit weather rule'"
      @update:model-value="editing = null"
    >
      <RuleEditor v-if="editing !== null" :rule="current()" @done="editing = null" />
    </UiSheet>
    <ConfirmDialog
      :pending="confirm.pending.value"
      @accept="confirm.accept"
      @cancel="confirm.cancel"
    />
  </SettingsPage>
</template>

<style scoped>
.rule {
  display: grid;
  gap: var(--ui-space-2);
}
.rule__head,
.rule__row {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
}
.rule__name {
  flex: 1;
  font: var(--ui-type-title-md);
}
.rule__msg {
  margin: 0;
  color: var(--ui-on-surface-variant);
}
code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  word-break: break-word;
}
</style>
