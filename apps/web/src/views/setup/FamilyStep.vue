<script setup lang="ts">
import { useSetupStore } from '../../stores/setup'
import { UiCard } from '../../ui'
import MembersManager from '../parts/MembersManager.vue'
import StepButtons from './StepButtons.vue'

const setup = useSetupStore()
</script>

<template>
  <div class="step" data-testid="step-family">
    <UiCard>
      <template #header>Family</template>
      <p>Add the other adults and the children. You can do this later under Settings, Members.</p>
    </UiCard>
    <MembersManager :fab="false" />
    <form @submit.prevent="setup.goto(5)">
      <StepButtons skip @back="setup.goto(3)" @skip="setup.goto(5)" />
    </form>
  </div>
</template>

<style scoped>
.step {
  display: grid;
  gap: var(--ui-space-4);
}
p {
  margin: 0;
  color: var(--ui-on-surface-variant);
}
</style>
