<script setup lang="ts">
// Scaffold of every Settings child page: top app bar with a back arrow when the list is a
// separate screen (compact, medium), then the page's cards. Wave-3 pages copy this.
import { useRouter } from 'vue-router'
import { UiPage, UiTopAppBar, useBreakpoint } from '../ui'

defineProps<{ title: string }>()
const router = useRouter()
const bp = useBreakpoint()
</script>

<template>
  <UiPage>
    <template #top>
      <UiTopAppBar
        :title="title"
        :back="bp !== 'expanded'"
        back-label="Settings"
        @back="router.push('/settings')"
      >
        <template #actions><slot name="actions" /></template>
      </UiTopAppBar>
    </template>
    <slot />
  </UiPage>
</template>
