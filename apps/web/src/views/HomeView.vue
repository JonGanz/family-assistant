<script setup lang="ts">
import { useSessionStore } from '../stores/session'

const session = useSessionStore()
</script>

<template>
  <section>
    <h1>{{ session.household?.name ?? 'Home' }}</h1>
    <p>Welcome. Use the menu to manage members and devices.</p>
  </section>
</template>
