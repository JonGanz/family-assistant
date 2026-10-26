<script setup lang="ts">
import { onMounted, ref } from 'vue'
import type { WeatherRuleView } from '../api/client'
import ErrorBanner from '../components/ErrorBanner.vue'
import PageHeader from '../components/PageHeader.vue'
import { useRulesStore } from '../stores/rules'
import RuleEditor from './parts/RuleEditor.vue'
import { describeError } from './shared'
import './parts/settings.css'

const rules = useRulesStore()
/** `null` = closed, `'new'` = adding, otherwise the rule id being edited. */
const editing = ref<string | null>(null)
const error = ref('')

onMounted(() => rules.load())

async function act(fn: () => Promise<unknown>) {
  error.value = ''
  try {
    await fn()
  } catch (e) {
    error.value = describeError(e, 'Could not save.')
  }
}

const toggle = (r: WeatherRuleView) =>
  act(() => rules.patch(r, { status: r.status === 'active' ? 'disabled' : 'active' }))
const remove = (r: WeatherRuleView) => {
  if (confirm(`Delete the rule "${r.name}"?`)) void act(() => rules.remove(r.id))
}
</script>

<template>
  <section>
    <PageHeader eyebrow="Settings" title="Weather">
      <template #actions>
        <button v-if="editing !== 'new'" type="button" class="btn-primary" @click="editing = 'new'">
          Add a rule
        </button>
      </template>
    </PageHeader>
    <ErrorBanner v-if="rules.error" :message="rules.error" />
    <ErrorBanner v-if="rules.conflict && editing === null" conflict @reload="rules.load()" />
    <ErrorBanner v-if="error" :message="error" />

    <ul class="card-list">
      <li v-for="r in rules.items" :key="r.id" :data-testid="`rule-${r.name}`">
        <template v-if="editing !== r.id">
          <div class="row">
            <strong>{{ r.name }}</strong>
            <span
              class="badge"
              :class="
                r.severity === 'severe' ? 'error' : r.severity === 'warning' ? 'needs_reauth' : 'ok'
              "
            >
              {{ r.severity }}
            </span>
            <span class="badge">{{ r.status }}</span>
          </div>
          <code>{{ r.condition }}</code>
          <p class="muted">{{ r.message }}</p>
          <div class="row">
            <button type="button" class="btn-sm" @click="toggle(r)">
              {{ r.status === 'active' ? 'Disable' : 'Enable' }}
            </button>
            <button type="button" class="btn-sm" @click="editing = r.id">Edit</button>
            <button type="button" class="btn-danger btn-sm" @click="remove(r)">Delete</button>
          </div>
        </template>
        <RuleEditor v-else :rule="r" @done="editing = null" />
      </li>
    </ul>

    <RuleEditor v-if="editing === 'new'" :rule="null" @done="editing = null" />
  </section>
</template>

<style scoped>
.row {
  margin: var(--space-1) 0;
}
code {
  font-family: ui-monospace, monospace;
}
</style>
