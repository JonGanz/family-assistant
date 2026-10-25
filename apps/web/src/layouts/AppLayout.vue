<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useSessionStore } from '../stores/session'

const session = useSessionStore()
const router = useRouter()

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
.logout {
  margin-left: auto;
}
.app-main {
  padding: 1rem;
  max-width: 64rem;
  margin: 0 auto;
}
</style>
