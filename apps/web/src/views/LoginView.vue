<script setup lang="ts">
import ErrorBanner from '../components/ErrorBanner.vue'
import { ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ApiError } from '../api/http'
import { useSessionStore } from '../stores/session'

const session = useSessionStore()
const router = useRouter()
const route = useRoute()
const username = ref('')
const password = ref('')
const error = ref('')
const busy = ref(false)

function describe(e: unknown): string {
  if (e instanceof ApiError) {
    if (e.status === 401) return 'Wrong username or password.'
    if (e.status === 423) return `This account is locked. ${e.problem.detail ?? ''}`.trim()
    if (e.status === 429) return 'Too many attempts. Wait a moment and try again.'
    return e.message
  }
  return 'Could not reach the server.'
}

async function submit() {
  error.value = ''
  busy.value = true
  try {
    await session.login(username.value, password.value)
    const r = route.query.redirect
    await router.replace(typeof r === 'string' && r.startsWith('/') ? r : '/')
  } catch (e) {
    error.value = describe(e)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <section class="login">
    <h1>Log in</h1>
    <form @submit.prevent="submit">
      <label>
        Username
        <input v-model="username" name="username" autocomplete="username" required />
      </label>
      <label>
        Password
        <input
          v-model="password"
          name="password"
          type="password"
          autocomplete="current-password"
          required
        />
      </label>
      <ErrorBanner v-if="error" :message="error" />
      <button type="submit" :disabled="busy">Log in</button>
    </form>
  </section>
</template>

<style scoped>
.login {
  max-width: 22rem;
  margin: 3rem auto;
}
form {
  display: grid;
  gap: 1rem;
}
label {
  display: grid;
  gap: 0.25rem;
}
</style>
