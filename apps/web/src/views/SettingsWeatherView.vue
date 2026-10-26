<script setup lang="ts">
import { onMounted, ref } from 'vue'
import type { WeatherRuleView } from '../api/client'
import ErrorBanner from '../components/ErrorBanner.vue'
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
    <h1>Weather</h1>
    <ErrorBanner v-if="rules.error" :message="rules.error" />
    <ErrorBanner v-if="rules.conflict && editing === null" conflict @reload="rules.load()" />
    <ErrorBanner v-if="error" :message="error" />

    <ul class="settings-list">
      <li v-for="r in rules.items" :key="r.id" :data-testid="`rule-${r.name}`">
        <template v-if="editing !== r.id">
          <div class="head">
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
          <p class="settings-note">{{ r.message }}</p>
          <div class="head">
            <button type="button" @click="toggle(r)">
              {{ r.status === 'active' ? 'Disable' : 'Enable' }}
            </button>
            <button type="button" @click="editing = r.id">Edit</button>
            <button type="button" @click="remove(r)">Delete</button>
          </div>
        </template>
        <RuleEditor v-else :rule="r" @done="editing = null" />
      </li>
    </ul>

    <RuleEditor v-if="editing === 'new'" :rule="null" @done="editing = null" />
    <button v-else type="button" @click="editing = 'new'">Add a rule</button>
  </section>
</template>

<style scoped>
.head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
  margin: var(--space-1) 0;
}
code {
  font-family: ui-monospace, monospace;
}
</style>
