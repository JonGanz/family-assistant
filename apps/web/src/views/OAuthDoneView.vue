<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useSourcesStore } from '../stores/sources'
import { UiButton, UiCard, UiIcon, UiPage, UiTopAppBar } from '../ui'
import { safeReturnTo } from './shared'

const route = useRoute()
const router = useRouter()
const sources = useSourcesStore()

const error = computed(() => (typeof route.query.error === 'string' ? route.query.error : ''))

const target = computed(() => safeReturnTo(route.query.return_to))

onMounted(async () => {
  if (error.value) return
  await sources.load()
  setTimeout(() => void router.replace(target.value), 1200)
})
</script>

<template>
  <UiPage>
    <template #top><UiTopAppBar title="Google account" /></template>
    <UiCard v-if="error" variant="outlined" role="alert">
      <template #header>
        <span class="head"><UiIcon name="error" /> Google did not connect</span>
      </template>
      <p>{{ `Google did not connect: ${error}.` }}</p>
      <template #actions>
        <UiButton variant="tonal" @click="router.replace(target)">Back</UiButton>
      </template>
    </UiCard>
    <UiCard v-else role="status">
      <template #header>
        <span class="head"><UiIcon name="success" /> Connected</span>
      </template>
      <p>Taking you back…</p>
      <template #actions>
        <UiButton variant="tonal" @click="router.replace(target)">Continue now</UiButton>
      </template>
    </UiCard>
  </UiPage>
</template>

<style scoped>
.head {
  display: inline-flex;
  align-items: center;
  gap: var(--ui-space-2);
}
p {
  margin: 0;
}
</style>
