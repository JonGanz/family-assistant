<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useSourcesStore } from '../stores/sources'
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
  <section>
    <h1>Google account</h1>
    <p v-if="error" class="error" role="alert">
      Google did not connect: {{ error }}.
      <RouterLink :to="target">Back</RouterLink>
    </p>
    <p v-else role="status">
      Connected. Taking you back…
      <RouterLink :to="target">Continue now</RouterLink>
    </p>
  </section>
</template>

<style scoped>
.error {
  color: var(--color-danger);
}
</style>
