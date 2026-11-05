<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import type { TaskListView } from '../../api/client'
import ResponsiveItems from '../../app/ResponsiveItems.vue'
import { calendarDisplayLabel } from '../../lib/tasks'
import { useListsStore } from '../../stores/lists'
import {
  UiButton,
  UiCheckbox,
  UiChip,
  UiEmptyState,
  UiRadioGroup,
  UiSelect,
  UiSkeleton,
  UiTextField,
} from '../../ui'
import { describeError } from '../shared'
import ConfirmDialog from './settings-mail/ConfirmDialog.vue'
import PageFab from './settings-mail/PageFab.vue'
import PageNotices from './settings-mail/PageNotices.vue'
import SheetForm from './settings-mail/SheetForm.vue'
import { useConfirm } from './settings-mail/useConfirm'
import {
  emptyListForm,
  listDefaults,
  listFormFrom,
  listInput,
  listPatch,
  type ListForm,
  type ListKind,
} from './listForm'

const lists = useListsStore()
const editing = ref<string | null>(null)
const form = reactive<ListForm>(emptyListForm(0))
const error = ref('')
const confirm = useConfirm()
const kindOptions = [
  { value: 'chores', label: 'Chores' },
  { value: 'homework', label: 'Homework' },
  { value: 'general', label: 'General' },
]
const policyOptions = [
  { value: 'assignee', label: "The person it's for (or anyone, if unassigned)" },
  { value: 'adult_only', label: 'Adults only' },
]
const childOptions = [
  { value: 'none', label: 'No' },
  { value: 'own', label: 'Yes, for themselves' },
]
const calendarOptions = [
  { value: 'hidden', label: 'Hidden' },
  { value: 'until_done', label: 'Until done' },
  { value: 'always', label: 'Key date' },
]
const claimOptions = [
  { value: 'end_of_day', label: 'Until the end of the day' },
  { value: 'minutes', label: 'For a number of minutes' },
]

onMounted(() => lists.load())

function open(l: TaskListView | null) {
  Object.assign(form, l ? listFormFrom(l) : emptyListForm(lists.items.length))
  editing.value = l ? l.id : 'new'
  error.value = ''
}
function setKind(k: ListKind) {
  form.kind = k
  Object.assign(form, listDefaults(k))
}
async function save() {
  error.value = ''
  try {
    if (editing.value === 'new') await lists.create(listInput(form))
    else {
      const l = lists.items.find((x) => x.id === editing.value)
      if (l && !(await lists.patch(l, listPatch(form)))) return
    }
    editing.value = null
  } catch (e) {
    error.value = describeError(e, 'Save failed.')
  }
}
const remove = (l: TaskListView) =>
  confirm.ask(`Delete the list “${l.name}”?`, 'Tasks in it are removed too.', async () => {
    try {
      await lists.remove(l.id)
    } catch (e) {
      error.value = describeError(e, 'Could not delete the list.')
    }
  })
</script>

<template>
  <div class="panel">
    <PageNotices
      :load-error="lists.error"
      :conflict="lists.conflict && !editing"
      :error="editing ? '' : error"
      @reload="lists.load()"
    />
    <UiSkeleton v-if="!lists.loaded && !lists.error" :lines="3" />
    <UiEmptyState
      v-else-if="lists.loaded && !lists.items.length"
      icon="tasks"
      title="No lists yet."
    />
    <ResponsiveItems :items="lists.items">
      <template #item="{ item: l }">
        <div class="item" :data-testid="`list-${l.name}`">
          <div class="item__head">
            <strong class="item__name">{{ l.name }}</strong>
            <UiChip>{{ l.kind }}</UiChip>
          </div>
          <p class="meta">
            Children add: {{ l.child_can_add === 'own' ? 'their own' : 'no' }} · Calendar:
            {{ calendarDisplayLabel(l.calendar_default) }} ·
            {{ l.completion_policy === 'adult_only' ? 'adults complete' : 'assignee completes' }}
          </p>
          <div class="item__row">
            <UiButton variant="text" @click="open(l)">Edit</UiButton>
            <UiButton variant="text" @click="remove(l)">Delete</UiButton>
          </div>
        </div>
      </template>
    </ResponsiveItems>

    <PageFab label="Add a list" @click="open(null)" />
    <SheetForm
      :model-value="!!editing"
      :title="editing === 'new' ? 'New list' : 'Edit list'"
      @update:model-value="editing = null"
      @save="save"
    >
      <UiTextField v-model="form.name" label="Name" name="name" required />
      <UiSelect
        :model-value="form.kind"
        label="Kind (sets the defaults below)"
        :options="kindOptions"
        @update:model-value="setKind($event as ListKind)"
      />
      <UiSelect
        :model-value="form.completion_policy"
        label="Who can complete a task"
        :options="policyOptions"
        @update:model-value="form.completion_policy = $event as typeof form.completion_policy"
      />
      <UiSelect
        :model-value="form.child_can_add"
        label="Children can add tasks"
        :options="childOptions"
        @update:model-value="form.child_can_add = $event as typeof form.child_can_add"
      />
      <UiCheckbox v-model="form.dashboard_add" label="Allow adding from the wall dashboard" />
      <UiSelect
        :model-value="form.calendar_default"
        label="On the calendar by default"
        :options="calendarOptions"
        @update:model-value="form.calendar_default = $event as typeof form.calendar_default"
      />
      <UiRadioGroup
        :model-value="form.claim"
        legend="A claimed task is held"
        :options="claimOptions"
        @update:model-value="form.claim = $event as typeof form.claim"
      />
      <UiTextField
        v-if="form.claim === 'minutes'"
        :model-value="String(form.claimMinutes)"
        label="Minutes"
        name="claim_minutes"
        type="number"
        min="1"
        max="10080"
        @update:model-value="form.claimMinutes = Number($event)"
      />
      <UiCheckbox v-model="form.visible_on_dashboard" label="Show on the dashboard" />
      <UiCheckbox v-model="form.useColor" label="Colour" />
      <UiTextField v-if="form.useColor" v-model="form.color" label="Colour value" type="color" />
      <UiTextField
        :model-value="String(form.sort)"
        label="Order"
        name="sort"
        type="number"
        @update:model-value="form.sort = Number($event)"
      />
      <p v-if="error" role="alert">{{ error }}</p>
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
.item {
  display: grid;
  gap: var(--ui-space-3);
}
.item {
  gap: var(--ui-space-2);
}
.item__head,
.item__row {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
}
.item__name {
  flex: 1;
  font: var(--ui-type-title-md);
}
.meta {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
</style>
