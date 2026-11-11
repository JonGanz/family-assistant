<script setup lang="ts">
import MemberChip from '../components/MemberChip.vue'

/** Rounded member chips shared by task rows and event items on the kiosk. */
defineProps<{ members: { id: string; name: string; color: string }[] }>()
</script>

<template>
  <span v-if="members.length" class="member-pills">
    <MemberChip v-for="m in members" :key="m.id" :name="m.name" :color="m.color" variant="app" />
  </span>
</template>

<style scoped>
.member-pills {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}
.member-pills :deep(.member-chip) {
  padding: 0 var(--space-3);
  font-size: var(--kiosk-font-small);
  line-height: 1.6;
}
</style>
