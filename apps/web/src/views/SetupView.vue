<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { STEPS, useSetupStore } from '../stores/setup'
import { UiProgress, UiSkeleton } from '../ui'
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
const total = STEPS.length
const current = computed(() => STEP_COMPONENTS[setup.step - 1] ?? WelcomeStep)

onMounted(async () => {
  await setup.load(true)
  setup.resume()
})
</script>

<template>
  <section class="setup" aria-labelledby="page-title">
    <h1 id="page-title" class="setup__title">Set up Family Assistant</h1>
    <div class="setup__progress" aria-label="Setup steps">
      <p class="setup__step" data-testid="step-count">
        Step {{ setup.step }} of {{ total }}: {{ STEPS[setup.step - 1] }}
      </p>
      <UiProgress :value="(setup.step / total) * 100" :label="`Step ${setup.step} of ${total}`" />
    </div>
    <component :is="current" v-if="setup.status" :key="setup.step" />
    <UiSkeleton v-else :lines="4" />
  </section>
</template>

<style scoped>
.setup {
  display: grid;
  gap: var(--ui-space-4);
}
.setup__title {
  margin: 0;
  font: var(--ui-type-headline-sm);
}
.setup__progress {
  display: grid;
  gap: var(--ui-space-2);
}
.setup__step {
  margin: 0;
  font: var(--ui-type-label-lg);
  color: var(--ui-on-surface-variant);
}
</style>
