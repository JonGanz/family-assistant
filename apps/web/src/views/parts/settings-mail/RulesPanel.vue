<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import ResponsiveItems from '../../../app/ResponsiveItems.vue'
import type { Member } from '../../../api/client'
import { mailApi, type RuleTestResult } from '../../../api/mailSettings'
import { ApiError } from '../../../api/http'
import { useMailSettingsStore } from '../../../stores/mailSettings'
import { useSourcesStore } from '../../../stores/sources'
import {
  UiButton,
  UiCard,
  UiCheckbox,
  UiEmptyState,
  UiIconButton,
  UiSelect,
  UiTextField,
} from '../../../ui'
import { describeError } from '../../shared'
import ConfirmDialog from './ConfirmDialog.vue'
import InstantCheck from './InstantCheck.vue'
import SheetForm from './SheetForm.vue'
import { useConfirm } from './useConfirm'
import {
  DECISION_POINTS,
  FIELDS,
  OPS,
  describeConditions,
  emptyCondition,
  emptyRuleForm,
  fromView,
  isBooleanPoint,
  toInput,
  type RuleForm,
} from './ruleForm'

const props = defineProps<{ members: Member[] }>()
const store = useMailSettingsStore()
const sources = useSourcesStore()

const editing = ref<string | 'new' | null>(null)
const form = reactive<RuleForm>(emptyRuleForm())
const error = ref('')
const confirm = useConfirm()
const sel = (rows: { key: string; label: string }[]) =>
  rows.map((r) => ({ value: r.key, label: r.label }))
const pointOptions = sel(DECISION_POINTS)
const fieldOptions = sel(FIELDS)
const opOptions = sel(OPS)
const yesNo = [
  { value: 'true', label: 'Yes' },
  { value: 'false', label: 'No' },
]
const categoryOptions = computed(() => [
  { value: '', label: 'Choose…', disabled: true },
  ...categories.value.map((c) => ({ value: c.key, label: c.label })),
])
const personOptions = (members: Member[]) => [
  { value: '', label: 'Choose…', disabled: true },
  { value: 'household', label: 'The whole household' },
  ...members.map((m) => ({ value: m.id, label: m.display_name })),
]
const calendarOptions = computed(() => [
  { value: '', label: 'Choose…', disabled: true },
  ...sources.calendars.map((c) => ({ value: c.id, label: c.name })),
])
const testResult = ref<RuleTestResult | null>(null)

const categories = computed(() => store.categories.filter((c) => !c.archived))
const dpLabel = (k: string) => DECISION_POINTS.find((d) => d.key === k)?.label ?? k

function open(id: string | 'new') {
  error.value = ''
  testResult.value = null
  editing.value = id
  const r = store.rules.find((x) => x.id === id)
  Object.assign(form, r ? fromView(r) : emptyRuleForm())
  if (!r) form.value = ''
}

function onPoint(k: string) {
  form.decision_point = k as typeof form.decision_point
  form.value = isBooleanPoint(form.decision_point) ? 'true' : ''
}

/** 422 problems name the offending condition field and message. */
function explain(e: unknown, fallback: string): string {
  if (e instanceof ApiError && e.status === 422) {
    const p = e.problem as { field?: string; message?: string; detail?: string }
    return [p.field, p.message ?? p.detail].filter(Boolean).join(': ') || fallback
  }
  return describeError(e, fallback)
}

async function save() {
  error.value = ''
  try {
    const input = toInput(form)
    const r = store.rules.find((x) => x.id === editing.value)
    const ok = r
      ? await store.patchRule(r, {
          name: input.name,
          priority: input.priority,
          conditions: input.conditions,
          value: input.value,
          confidence: input.confidence,
          enabled: input.enabled,
        })
      : await store.createRule(input)
    if (ok) editing.value = null
  } catch (e) {
    error.value = explain(e, 'Could not save the rule.')
  }
}

async function test() {
  error.value = ''
  testResult.value = null
  try {
    testResult.value = await mailApi.testRule(toInput(form), 200)
  } catch (e) {
    error.value = explain(e, 'The test failed.')
  }
}

function remove(id: string, name: string) {
  confirm.ask(`Delete “${name}”?`, 'The rule stops applying to new mail.', async () => {
    error.value = ''
    try {
      await store.removeRule(id)
    } catch (e) {
      error.value = explain(e, 'Could not delete.')
    }
  })
}

const setEnabled = (id: string, v: boolean) => {
  const r = store.rules.find((x) => x.id === id)
  return r ? store.patchRule(r, { enabled: v }) : Promise.resolve(false)
}
</script>

<template>
  <div class="panel">
    <UiCard v-if="error && editing === null" variant="outlined" role="alert">{{ error }}</UiCard>
    <ResponsiveItems :items="store.rules">
      <template #item="{ item: r }">
        <div class="rule" :data-testid="`rule-${r.name}`">
          <InstantCheck
            as="switch"
            :model-value="r.enabled"
            :label="`Enable ${r.name}`"
            :save="(v: boolean) => setEnabled(r.id, v)"
          />
          <strong>{{ r.name }}</strong>
          <p class="meta">
            {{ dpLabel(r.decision_point) }} = {{ String(r.value) }} when
            {{ describeConditions(r.conditions) }} (priority {{ r.priority }})
          </p>
          <div class="row">
            <UiButton variant="text" @click="open(r.id)">Edit</UiButton>
            <UiButton variant="text" @click="remove(r.id, r.name)">Delete</UiButton>
          </div>
        </div>
      </template>
    </ResponsiveItems>
    <UiEmptyState v-if="!store.rules.length && store.loaded" icon="filter" title="No rules yet." />
    <div><UiButton variant="tonal" icon="add" @click="open('new')">Add a rule</UiButton></div>

    <SheetForm
      :model-value="editing !== null"
      :title="editing === 'new' ? 'New rule' : 'Edit rule'"
      @update:model-value="editing = null"
      @save="save"
    >
      <div data-testid="rule-form" class="fields">
        <UiTextField v-model="form.name" label="Name" name="name" required />
        <UiSelect
          :model-value="form.decision_point"
          label="Decides"
          :options="pointOptions"
          :disabled="editing !== 'new'"
          @update:model-value="onPoint"
        />

        <fieldset class="conds">
          <legend>When all of these match</legend>
          <div v-for="(c, i) in form.conditions" :key="i" class="cond">
            <UiSelect
              :model-value="c.field"
              :label="`Field ${i + 1}`"
              :options="fieldOptions"
              @update:model-value="c.field = $event as typeof c.field"
            />
            <UiTextField
              v-if="c.field === 'header'"
              v-model="c.header"
              :label="`Header name ${i + 1}`"
              placeholder="Header name"
            />
            <UiSelect
              :model-value="c.op"
              :label="`Operator ${i + 1}`"
              :options="opOptions"
              @update:model-value="c.op = $event as typeof c.op"
            />
            <UiTextField
              v-if="c.op !== 'exists'"
              v-model="c.values"
              :label="`Values ${i + 1}`"
              placeholder="comma separated"
            />
            <UiIconButton
              icon="close"
              :label="`Remove condition ${i + 1}`"
              :disabled="form.conditions.length === 1"
              @click="form.conditions.splice(i, 1)"
            />
          </div>
          <UiButton variant="tonal" icon="add" @click="form.conditions.push(emptyCondition())"
            >Add condition</UiButton
          >
        </fieldset>

        <UiSelect
          v-if="form.decision_point === 'email.category'"
          v-model="form.value"
          label="Then the category is"
          :options="categoryOptions"
        />
        <UiSelect
          v-else-if="isBooleanPoint(form.decision_point)"
          v-model="form.value"
          label="Then it is"
          :options="yesNo"
        />
        <UiSelect
          v-else-if="form.decision_point === 'email.member_link'"
          v-model="form.value"
          label="Then it is about"
          :options="personOptions(props.members)"
        />
        <UiSelect
          v-else
          v-model="form.value"
          label="Then write the event to"
          :options="calendarOptions"
        />

        <UiTextField
          :model-value="String(form.priority)"
          label="Priority (lower runs first)"
          name="priority"
          type="number"
          min="0"
          @update:model-value="form.priority = Number($event)"
        />
        <UiTextField
          :model-value="String(form.confidence)"
          label="Confidence (0 to 1)"
          name="confidence"
          type="number"
          min="0"
          max="1"
          step="0.05"
          @update:model-value="form.confidence = Number($event)"
        />
        <UiCheckbox v-model="form.enabled" label="Enabled" />

        <UiCard v-if="error" variant="outlined" role="alert" data-testid="rule-error">{{
          error
        }}</UiCard>
        <div><UiButton variant="tonal" @click="test">Test on recent mail</UiButton></div>
        <UiCard v-if="testResult" variant="outlined" data-testid="rule-test">
          <p role="status">
            Matched {{ testResult.matched.length }} of {{ testResult.scanned }} recent messages.
          </p>
          <ul class="matches">
            <li v-for="m in testResult.matched.slice(0, 10)" :key="m.message">
              {{ m.from_addr }}: {{ m.subject }}
            </li>
          </ul>
        </UiCard>
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
.rule,
.fields,
.cond {
  display: grid;
  gap: var(--ui-space-3);
}
.rule {
  gap: var(--ui-space-2);
}
.meta {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
.row {
  display: flex;
  gap: var(--ui-space-2);
}
.conds {
  display: grid;
  gap: var(--ui-space-3);
  border: 0;
  padding: 0;
  margin: 0;
}
.cond {
  padding-bottom: var(--ui-space-3);
  border-bottom: 1px solid var(--ui-outline-variant);
}
.matches {
  margin: 0;
  padding-left: var(--ui-space-4);
}
</style>
