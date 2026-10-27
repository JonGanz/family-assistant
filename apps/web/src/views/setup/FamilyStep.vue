<script setup lang="ts">
import { useSetupStore } from '../../stores/setup'
import MembersManager from '../parts/MembersManager.vue'
import StepButtons from './StepButtons.vue'

const setup = useSetupStore()
</script>

<template>
  <div data-testid="step-family">
    <h2>Family</h2>
    <p class="muted">
      Add the other adults and the children. You can do this later under Settings, Members.
    </p>
    <MembersManager />
    <form class="form" @submit.prevent="setup.goto(5)">
      <StepButtons skip @back="setup.goto(3)" @skip="setup.goto(5)" />
    </form>
  </div>
</template>
