<script setup lang="ts">
import { computed, onMounted } from 'vue'
import PageHeader from '../components/PageHeader.vue'
import { STEPS, useSetupStore } from '../stores/setup'
import AccessStep from './setup/AccessStep.vue'
import FamilyStep from './setup/FamilyStep.vue'
import HouseholdStep from './setup/HouseholdStep.vue'
import KeyStep from './setup/KeyStep.vue'
import OwnerStep from './setup/OwnerStep.vue'
import StartStep from './setup/StartStep.vue'
import WelcomeStep from './setup/WelcomeStep.vue'

const setup = useSetupStore()
const STEP_COMPONENTS = [
  WelcomeStep,
  HouseholdStep,
  OwnerStep,
  FamilyStep,
  AccessStep,
  KeyStep,
  StartStep,
]
const current = computed(() => STEP_COMPONENTS[setup.step - 1] ?? WelcomeStep)

onMounted(async () => {
  await setup.load(true)
  setup.resume()
})
</script>

<template>
  <section aria-labelledby="page-title">
    <PageHeader title="Set up Family Assistant" />
    <ol class="stepper" aria-label="Setup steps">
      <li
        v-for="(name, i) in STEPS"
        :key="name"
        :aria-current="setup.step === i + 1 ? 'step' : undefined"
        :class="{ on: setup.step === i + 1, done: setup.step > i + 1 }"
      >
        {{ i + 1 }}. {{ name }}
      </li>
    </ol>
    <component :is="current" v-if="setup.status" :key="setup.step" />
    <p v-else class="muted" role="status">Loading…</p>
  </section>
</template>

<style scoped>
.stepper {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  list-style: none;
  padding: 0;
  margin: 0 0 var(--space-4);
  font-size: var(--font-size-sm);
}
.stepper li {
  padding: 0 var(--space-2);
  border: 1px solid var(--color-border);
  border-radius: 999px;
  color: var(--color-muted);
}
.stepper li.on {
  color: var(--color-accent);
  border-color: var(--color-accent);
  font-weight: 600;
}
.stepper li.done {
  color: var(--color-text);
}
</style>
