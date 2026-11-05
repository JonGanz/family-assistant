<script setup lang="ts">
import { ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ApiError } from '../api/http'
import { useSessionStore } from '../stores/session'
import { UiButton, UiCard, UiIconButton, UiTextField } from '../ui'

const session = useSessionStore()
const router = useRouter()
const route = useRoute()
const username = ref('')
const password = ref('')
const error = ref('')
const busy = ref(false)
const reveal = ref(false)

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
  <main class="login" aria-labelledby="page-title">
    <UiCard variant="elevated" class="login__card">
      <template #header><h1 id="page-title" class="login__title">Log in</h1></template>
      <form class="login__form" @submit.prevent="submit">
        <UiTextField
          v-model="username"
          label="Username"
          name="username"
          autocomplete="username"
          required
        />
        <UiTextField
          v-model="password"
          label="Password"
          name="password"
          :type="reveal ? 'text' : 'password'"
          autocomplete="current-password"
          required
        >
          <template #trailing>
            <UiIconButton
              :icon="reveal ? 'visibility_off' : 'visibility'"
              :label="reveal ? 'Hide text' : 'Show text'"
              :pressed="reveal"
              @click="reveal = !reveal"
            />
          </template>
        </UiTextField>
        <p v-if="error" class="login__error" role="alert">{{ error }}</p>
        <UiButton type="submit" :loading="busy">Log in</UiButton>
      </form>
    </UiCard>
  </main>
</template>

<style scoped>
.login {
  min-height: 100dvh;
  display: flex;
  align-items: stretch;
  justify-content: center;
  background: var(--ui-surface);
  color: var(--ui-on-surface);
}
.login__card {
  width: 100%;
}
.login__title {
  margin: 0;
  font: var(--ui-type-headline-sm);
}
.login__form {
  display: grid;
  gap: var(--ui-space-4);
}
.login__error {
  margin: 0;
  color: var(--ui-error);
}
@media (min-width: 600px) {
  .login {
    align-items: center;
    padding: var(--ui-space-4);
  }
  .login__card {
    max-width: 24rem;
  }
}
</style>
