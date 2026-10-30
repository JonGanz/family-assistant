<script setup lang="ts">
import { watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useInboxStore } from '../stores/inbox'
import { useSessionStore } from '../stores/session'

const session = useSessionStore()
const router = useRouter()
const route = useRoute()
const inbox = useInboxStore()

/** A nav item is active for its own path and everything below it (`/tasks/:id` marks Tasks). */
const under = (p: string) =>
  p === '/' ? route.path === '/' : route.path === p || route.path.startsWith(`${p}/`)

// Adults only: the inbox badge loads once and then follows SSE `inbox.updated` (stores/live.ts).
watch(
  () => session.isAdult,
  (adult) => {
    if (adult) void inbox.load()
  },
  { immediate: true },
)

async function logout() {
  await session.logout()
  await router.replace({ name: 'login' })
}
</script>

<template>
  <div class="app-layout">
    <a class="skip-link" href="#main">Skip to content</a>
    <header class="app-header">
      <strong>Family Assistant</strong>
      <nav v-if="session.isUser" aria-label="Primary" class="nav-primary">
        <RouterLink to="/" :class="{ 'router-link-active': under('/') }">Today</RouterLink>
        <RouterLink to="/calendar" :class="{ 'router-link-active': under('/calendar') }"
          >Calendar</RouterLink
        >
        <RouterLink to="/tasks" :class="{ 'router-link-active': under('/tasks') }"
          >Tasks</RouterLink
        >
        <RouterLink to="/routines" :class="{ 'router-link-active': under('/routines') }"
          >Routines</RouterLink
        >
        <template v-if="session.isAdult">
          <RouterLink to="/mail" :class="{ 'router-link-active': under('/mail') }">Mail</RouterLink>
          <RouterLink to="/inbox" :class="{ 'router-link-active': under('/inbox') }">
            Inbox
            <span v-if="inbox.total > 0" class="nav-badge" :aria-label="`${inbox.total} waiting`">{{
              inbox.total
            }}</span>
          </RouterLink>
          <RouterLink to="/bills" :class="{ 'router-link-active': under('/bills') }"
            >Bills</RouterLink
          >
        </template>
      </nav>
      <div v-if="session.isUser" class="header-end">
        <RouterLink
          class="settings-link"
          to="/settings"
          :class="{ 'router-link-active': under('/settings') }"
          >Settings</RouterLink
        >
        <button class="logout" @click="logout">Log out</button>
      </div>
    </header>
    <main id="main" class="app-main"><slot /></main>
  </div>
</template>

<style scoped>
.app-header {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem 1.5rem;
  align-items: center;
  padding: 0.5rem 1rem;
  border-bottom: 1px solid var(--color-border);
}
.header-end {
  display: flex;
  gap: 0.75rem;
  align-items: center;
  margin-left: auto;
}
.settings-link {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 0 0.9rem;
  text-decoration: none;
}
.settings-link.router-link-active {
  border-color: var(--color-accent);
}
.header-end button {
  min-height: 40px;
}
@media (max-width: 40rem) {
  .app-header {
    gap: 0.25rem 0.75rem;
  }
  .header-end {
    order: 1;
  }
  .nav-primary {
    order: 2;
    flex: 1 0 100%;
    flex-wrap: nowrap;
    overflow-x: auto;
    white-space: nowrap;
    scrollbar-width: thin;
  }
}
.nav-badge {
  display: inline-block;
  min-width: 1.4em;
  padding: 0 0.4em;
  border-radius: 0;
  background: var(--color-accent);
  color: var(--color-bg);
  font-size: 0.8em;
  text-align: center;
}
.app-main {
  padding: 1rem;
  max-width: 64rem;
  margin: 0 auto;
}
</style>
