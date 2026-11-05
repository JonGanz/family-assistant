<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import ResponsiveItems from '../../../app/ResponsiveItems.vue'
import type {
  AutoAction,
  CategoryBehaviour,
  CategoryView,
  EmailTasksMode,
} from '../../../api/mailSettings'
import { useMailSettingsStore } from '../../../stores/mailSettings'
import { UiButton, UiCard, UiCheckbox, UiEmptyState, UiSelect, UiTextField } from '../../../ui'
import { describeError } from '../../shared'
import ConfirmDialog from './ConfirmDialog.vue'
import InstantCheck from './InstantCheck.vue'
import InstantSelect from './InstantSelect.vue'
import SheetForm from './SheetForm.vue'
import { DEFAULT_CATEGORY_COLOR } from './categoryDefaults'
import { toggleIn } from './toggle'
import { useConfirm } from './useConfirm'

const store = useMailSettingsStore()
const error = ref('')
const ACTIONS: { key: AutoAction; label: string }[] = [
  { key: 'label', label: 'Label in Gmail' },
  { key: 'archive', label: 'Archive' },
  { key: 'mark_read', label: 'Mark read' },
]
const BEHAVIOURS: { key: CategoryBehaviour; label: string }[] = [
  { key: 'ignore', label: 'Ignore (no items made)' },
  { key: 'keep', label: 'Keep (searchable only)' },
  { key: 'extract', label: 'Extract events, bills and tasks' },
]
const TASK_MODES: { key: EmailTasksMode; label: string }[] = [
  { key: 'suggest', label: 'Suggest (you approve)' },
  { key: 'create', label: 'Create directly' },
  { key: 'off', label: 'Off' },
]

const fresh = () => ({
  key: '',
  label: '',
  behaviour: 'keep' as CategoryBehaviour,
  notify: false,
  actions: ['label'] as AutoAction[],
  tasks: 'suggest' as EmailTasksMode,
  color: DEFAULT_CATEGORY_COLOR,
})
const form = reactive(fresh())
const confirm = useConfirm()
const adding = ref(false)
const active = computed(() => store.categories.filter((x) => !x.archived))
const behaviourOptions = BEHAVIOURS.map((b) => ({ value: b.key, label: b.label }))
const taskOptions = TASK_MODES.map((t) => ({ value: t.key, label: t.label }))
const archive = (c: CategoryView) =>
  confirm.ask(
    `Archive “${c.label}”?`,
    'Its old mail stays labelled.',
    () => run(() => store.archiveCategory(c), 'Could not archive.'),
    'Archive',
  )

async function run(fn: () => Promise<unknown>, fallback: string) {
  error.value = ''
  try {
    await fn()
  } catch (e) {
    error.value = describeError(e, fallback)
  }
}

const patch = (c: CategoryView, p: Parameters<typeof store.patchCategory>[1]) =>
  store.patchCategory(c, p)

function toggleAction(c: CategoryView, a: AutoAction, on: boolean) {
  const next = on ? [...c.auto_actions, a] : c.auto_actions.filter((x) => x !== a)
  return patch(c, { auto_actions: next })
}

async function add() {
  await run(async () => {
    const ok = await store.createCategory({
      key: form.key.trim(),
      label: form.label.trim(),
      behaviour: form.behaviour,
      notify_time_sensitive: form.notify,
      auto_actions: form.actions,
      email_tasks: form.tasks,
      color: form.color,
    })
    if (ok) {
      Object.assign(form, fresh())
      adding.value = false
    }
  }, 'Could not add the category.')
}
</script>

<template>
  <div class="panel">
    <UiCard v-if="error" variant="outlined" role="alert">{{ error }}</UiCard>
    <ResponsiveItems :items="active">
      <template #item="{ item: c }">
        <div class="cat">
          <div class="cat__head">
            <span
              class="cat__dot"
              :style="{ background: c.color ?? 'var(--ui-outline-variant)' }"
            />
            <strong class="cat__name">{{ c.label }}</strong>
            <code>{{ c.key }}</code>
          </div>
          <InstantSelect
            :model-value="c.behaviour"
            :label="`Behaviour for ${c.label}`"
            :options="behaviourOptions"
            :save="(v: string) => patch(c, { behaviour: v as CategoryBehaviour })"
          />
          <InstantCheck
            as="switch"
            :model-value="c.notify_time_sensitive"
            :label="`Notify for ${c.label} if time-sensitive`"
            :save="(v: boolean) => patch(c, { notify_time_sensitive: v })"
          />
          <fieldset class="cat__group">
            <legend>Automatic actions</legend>
            <InstantCheck
              v-for="a in ACTIONS"
              :key="a.key"
              :model-value="c.auto_actions.includes(a.key)"
              :label="`${a.label} (${c.label})`"
              :save="(v: boolean) => toggleAction(c, a.key, v)"
            />
          </fieldset>
          <InstantSelect
            :model-value="c.email_tasks"
            :label="`Tasks for ${c.label}`"
            :options="taskOptions"
            :save="(v: string) => patch(c, { email_tasks: v as EmailTasksMode })"
          />
          <div class="cat__actions">
            <UiButton variant="text" @click="archive(c)">Archive</UiButton>
          </div>
        </div>
      </template>
    </ResponsiveItems>
    <UiEmptyState v-if="store.loaded && !active.length" icon="label" title="No categories yet." />
    <p class="note">
      Auto actions only run on sources that are not read only. Archiving a category keeps its old
      mail labelled.
    </p>
    <div><UiButton variant="tonal" icon="add" @click="adding = true">Add a category</UiButton></div>

    <SheetForm v-model="adding" title="Add a category" save-label="Add category" @save="add">
      <div data-testid="add-category" class="sheet-fields">
        <UiTextField
          v-model="form.key"
          label="Key (lower case letters, digits, underscore)"
          name="key"
          pattern="[a-z][a-z0-9_]{0,31}"
          required
        />
        <UiTextField v-model="form.label" label="Label" name="label" required />
        <UiSelect
          :model-value="form.behaviour"
          label="What to do"
          :options="behaviourOptions"
          @update:model-value="form.behaviour = $event as CategoryBehaviour"
        />
        <UiCheckbox v-model="form.notify" label="Notify if time-sensitive" />
        <fieldset class="cat__group">
          <legend>Automatic actions</legend>
          <UiCheckbox
            v-for="a in ACTIONS"
            :key="a.key"
            :model-value="form.actions.includes(a.key)"
            :label="a.label"
            @update:model-value="toggleIn(form.actions, a.key, $event)"
          />
        </fieldset>
        <UiSelect
          :model-value="form.tasks"
          label="Tasks from email"
          :options="taskOptions"
          @update:model-value="form.tasks = $event as EmailTasksMode"
        />
        <UiTextField v-model="form.color" label="Colour" type="color" />
      </div>
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
.cat,
.sheet-fields {
  display: grid;
  gap: var(--ui-space-3);
}
.cat__head,
.cat__actions {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
}
.cat__name {
  font: var(--ui-type-title-md);
}
.cat__dot {
  width: 0.75rem;
  height: 0.75rem;
  border-radius: var(--ui-radius-full);
}
.cat__group {
  display: grid;
  gap: var(--ui-space-1);
  border: 0;
  padding: 0;
  margin: 0;
}
.cat__group legend {
  padding: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
.note {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
</style>
