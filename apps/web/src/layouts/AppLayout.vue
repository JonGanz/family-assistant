<script setup lang="ts">
import { computed, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useInboxStore } from '../stores/inbox'
import { useSessionStore } from '../stores/session'

const session = useSessionStore()
const router = useRouter()
const route = useRoute()
const inSettings = computed(() => route.path.startsWith('/settings'))
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
      <nav v-if="session.isUser" aria-label="Primary" class="nav-primary">
        <RouterLink to="/">Today</RouterLink>
        <RouterLink to="/calendar">Calendar</RouterLink>
        <RouterLink to="/tasks">Tasks</RouterLink>
        <RouterLink to="/routines">Routines</RouterLink>
        <RouterLink to="/members">Members</RouterLink>
        <template v-if="session.isAdult">
          <RouterLink to="/mail">Mail</RouterLink>
          <RouterLink to="/inbox">
            Inbox
            <span v-if="inbox.total > 0" class="nav-badge" :aria-label="`${inbox.total} waiting`">{{
              inbox.total
            }}</span>
          </RouterLink>
          <RouterLink to="/bills">Bills</RouterLink>
        </template>
      </nav>
      <div v-if="session.isUser" class="header-end">
        <RouterLink
          class="settings-link"
          :to="session.isAdult ? '/settings/household' : '/settings/notifications'"
          :class="{ 'router-link-active': inSettings }"
          >Settings</RouterLink
        >
        <button class="logout" @click="logout">Log out</button>
      </div>
    </header>
    <nav v-if="session.isUser && inSettings" aria-label="Settings" class="nav-sub">
      <template v-if="session.isAdult">
        <RouterLink to="/settings/household">Household</RouterLink>
        <RouterLink to="/settings/calendars">Calendars</RouterLink>
        <RouterLink to="/settings/mail">Mail</RouterLink>
        <RouterLink to="/settings/orgs">Organizations</RouterLink>
        <RouterLink to="/settings/tasks">Tasks</RouterLink>
        <RouterLink to="/settings/weather">Weather</RouterLink>
      </template>
      <RouterLink to="/settings/notifications">Notifications</RouterLink>
      <template v-if="session.isAdult">
        <RouterLink to="/settings/devices">Devices</RouterLink>
        <RouterLink to="/settings/status">Status</RouterLink>
      </template>
    </nav>
    <main class="app-main"><slot /></main>
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
.app-main {
  padding: 1rem;
  max-width: 64rem;
  margin: 0 auto;
}
</style>
