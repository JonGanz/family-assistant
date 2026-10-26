<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import type { Member } from '../../../api/client'
import { mailApi, type RuleTestResult } from '../../../api/mailSettings'
import { ApiError } from '../../../api/http'
import { useMailSettingsStore } from '../../../stores/mailSettings'
import { useSourcesStore } from '../../../stores/sources'
import { describeError } from '../../shared'
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

defineProps<{ members: Member[] }>()
const store = useMailSettingsStore()
const sources = useSourcesStore()

const editing = ref<string | 'new' | null>(null)
const form = reactive<RuleForm>(emptyRuleForm())
const error = ref('')
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

function onPoint() {
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

async function remove(id: string) {
  if (!confirm('Delete this rule?')) return
  error.value = ''
  try {
    await store.removeRule(id)
  } catch (e) {
    error.value = explain(e, 'Could not delete.')
  }
}

const toggleEnabled = async (id: string, v: boolean) => {
  const r = store.rules.find((x) => x.id === id)
  if (!r) return
  try {
    await store.patchRule(r, { enabled: v })
  } catch (e) {
    error.value = explain(e, 'Could not save.')
  }
}
</script>

<template>
  <div>
    <p v-if="error && editing === null" class="settings-error" role="alert">{{ error }}</p>
    <ul class="settings-list">
      <li v-for="r in store.rules" :key="r.id" :data-testid="`rule-${r.name}`">
        <div class="head">
          <label class="inline">
            <input
              type="checkbox"
              :checked="r.enabled"
              :aria-label="`Enable ${r.name}`"
              @change="toggleEnabled(r.id, ($event.target as HTMLInputElement).checked)"
            />
            <strong>{{ r.name }}</strong>
          </label>
          <span class="settings-note">
            {{ dpLabel(r.decision_point) }} = {{ String(r.value) }} when
            {{ describeConditions(r.conditions) }} (priority {{ r.priority }})
          </span>
          <button type="button" @click="open(r.id)">Edit</button>
          <button type="button" @click="remove(r.id)">Delete</button>
        </div>
      </li>
    </ul>
    <p v-if="!store.rules.length && store.loaded" class="settings-note">No rules yet.</p>
    <button v-if="editing === null" type="button" @click="open('new')">Add a rule</button>

    <form
      v-if="editing !== null"
      class="settings-form"
      data-testid="rule-form"
      @submit.prevent="save"
    >
      <h3>{{ editing === 'new' ? 'New rule' : 'Edit rule' }}</h3>
      <label>Name <input v-model="form.name" name="name" required /></label>
      <label>
        Decides
        <select
          v-model="form.decision_point"
          name="decision_point"
          :disabled="editing !== 'new'"
          @change="onPoint"
        >
          <option v-for="d in DECISION_POINTS" :key="d.key" :value="d.key">{{ d.label }}</option>
        </select>
      </label>

      <fieldset>
        <legend>When all of these match</legend>
        <div v-for="(c, i) in form.conditions" :key="i" class="cond">
          <select v-model="c.field" :aria-label="`Field ${i + 1}`">
            <option v-for="f in FIELDS" :key="f.key" :value="f.key">{{ f.label }}</option>
          </select>
          <input
            v-if="c.field === 'header'"
            v-model="c.header"
            :aria-label="`Header name ${i + 1}`"
            placeholder="Header name"
          />
          <select v-model="c.op" :aria-label="`Operator ${i + 1}`">
            <option v-for="o in OPS" :key="o.key" :value="o.key">{{ o.label }}</option>
          </select>
          <input
            v-if="c.op !== 'exists'"
            v-model="c.values"
            :aria-label="`Values ${i + 1}`"
            placeholder="comma separated"
          />
          <button
            type="button"
            :disabled="form.conditions.length === 1"
            :aria-label="`Remove condition ${i + 1}`"
            @click="form.conditions.splice(i, 1)"
          >
            ✕
          </button>
        </div>
        <button type="button" @click="form.conditions.push(emptyCondition())">Add condition</button>
      </fieldset>

      <label v-if="form.decision_point === 'email.category'">
        Then the category is
        <select v-model="form.value" name="value" required>
          <option value="" disabled>Choose…</option>
          <option v-for="c in categories" :key="c.key" :value="c.key">{{ c.label }}</option>
        </select>
      </label>
      <label v-else-if="isBooleanPoint(form.decision_point)">
        Then it is
        <select v-model="form.value" name="value">
          <option value="true">Yes</option>
          <option value="false">No</option>
        </select>
      </label>
      <label v-else-if="form.decision_point === 'email.member_link'">
        Then it is about
        <select v-model="form.value" name="value" required>
          <option value="" disabled>Choose…</option>
          <option value="household">The whole household</option>
          <option v-for="m in members" :key="m.id" :value="m.id">{{ m.display_name }}</option>
        </select>
      </label>
      <label v-else>
        Then write the event to
        <select v-model="form.value" name="value" required>
          <option value="" disabled>Choose…</option>
          <option v-for="c in sources.calendars" :key="c.id" :value="c.id">{{ c.name }}</option>
        </select>
      </label>

      <label>
        Priority (lower runs first)
        <input v-model.number="form.priority" name="priority" type="number" min="0" />
      </label>
      <label>
        Confidence (0 to 1)
        <input
          v-model.number="form.confidence"
          name="confidence"
          type="number"
          min="0"
          max="1"
          step="0.05"
        />
      </label>
      <label class="inline"><input v-model="form.enabled" type="checkbox" /> Enabled</label>

      <p v-if="error" class="settings-error" role="alert" data-testid="rule-error">{{ error }}</p>
      <div>
        <button type="button" @click="test">Test on recent mail</button>
        <button type="submit">Save</button>
        <button type="button" @click="editing = null">Cancel</button>
      </div>
      <div v-if="testResult" data-testid="rule-test">
        <p role="status">
          Matched {{ testResult.matched.length }} of {{ testResult.scanned }} recent messages.
        </p>
        <ul class="matches">
          <li v-for="m in testResult.matched.slice(0, 10)" :key="m.message">
            {{ m.from_addr }}: {{ m.subject }}
          </li>
        </ul>
      </div>
    </form>
  </div>
</template>

<style scoped>
.head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
}
.inline {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.cond {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}
.matches {
  margin: 0;
  padding-left: var(--space-4);
}
</style>
