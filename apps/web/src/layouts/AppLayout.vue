<script setup lang="ts">
import { watch } from 'vue'
import { useRouter } from 'vue-router'
import { useInboxStore } from '../stores/inbox'
import { useSessionStore } from '../stores/session'

const session = useSessionStore()
const router = useRouter()
const inbox = useInboxStore()

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
    <header class="app-header">
      <strong>Family Assistant</strong>
      <nav v-if="session.isUser">
        <RouterLink to="/">Today</RouterLink>
        <RouterLink to="/calendar">Calendar</RouterLink>
        <RouterLink to="/tasks">Tasks</RouterLink>
        <RouterLink to="/routines">Routines</RouterLink>
        <RouterLink to="/members">Members</RouterLink>
        <RouterLink to="/settings/notifications">Notifications</RouterLink>
        <template v-if="session.isAdult">
          <RouterLink to="/mail">Mail</RouterLink>
          <RouterLink to="/inbox">
            Inbox
            <span v-if="inbox.total > 0" class="nav-badge" :aria-label="`${inbox.total} waiting`">{{
              inbox.total
            }}</span>
          </RouterLink>
          <RouterLink to="/bills">Bills</RouterLink>
          <RouterLink to="/settings/mail">Mail settings</RouterLink>
          <RouterLink to="/settings/orgs">Organizations</RouterLink>
          <RouterLink to="/settings/household">Household</RouterLink>
          <RouterLink to="/settings/tasks">Task settings</RouterLink>
          <RouterLink to="/settings/calendars">Calendars</RouterLink>
          <RouterLink to="/settings/weather">Weather</RouterLink>
          <RouterLink to="/settings/devices">Devices</RouterLink>
          <RouterLink to="/settings/status">Status</RouterLink>
        </template>
      </nav>
      <button v-if="session.isUser" class="logout" @click="logout">Log out</button>
    </header>
    <main class="app-main"><slot /></main>
  </div>
</template>

<style scoped>
.app-header {
  display: flex;
  gap: 1.5rem;
  align-items: center;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid var(--color-border);
}
.app-header nav {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
}
.nav-badge {
  display: inline-block;
  min-width: 1.4em;
  padding: 0 0.4em;
  border-radius: 999px;
  background: var(--color-accent);
  color: var(--color-bg);
  font-size: 0.8em;
  text-align: center;
}
.logout {
  margin-left: auto;
}
.app-main {
  padding: 1rem;
  max-width: 64rem;
  margin: 0 auto;
}
</style>
