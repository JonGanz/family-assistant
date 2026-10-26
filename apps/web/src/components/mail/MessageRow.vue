<script setup lang="ts">
import { computed } from 'vue'
import type { CategoryView, Member, MessageSummary } from '../../api/client'
import { fmt } from './format'

const props = defineProps<{
  msg: MessageSummary
  tz: string
  categories: CategoryView[]
  members: Member[]
}>()

const cat = computed(() => props.categories.find((c) => c.key === props.msg.category))
const people = computed(() =>
  props.members.filter((m) => props.msg.members.includes(m.id)).map((m) => m.display_name),
)
</script>

<template>
  <RouterLink class="row" :to="{ name: 'mail-detail', params: { id: msg.id } }">
    <div class="top">
      <strong class="from">{{ msg.from_name || msg.from_addr }}</strong>
      <time class="settings-note" :datetime="msg.received_at">{{ fmt(msg.received_at, tz) }}</time>
    </div>
    <div class="subject">{{ msg.subject }}</div>
    <div class="meta">
      <span v-if="cat" class="badge">{{ cat.label }}</span>
      <span v-else-if="msg.category" class="badge">{{ msg.category }}</span>
      <span v-for="p in people" :key="p" class="badge">{{ p }}</span>
      <span v-if="msg.time_sensitive" class="badge needs_reauth">Time-sensitive</span>
      <span v-if="msg.is_bill" class="badge">Bill</span>
      <span v-if="msg.has_attachments" class="settings-note">Attachment</span>
    </div>
  </RouterLink>
</template>

<style scoped>
.row {
  display: grid;
  gap: var(--space-1);
  padding: var(--space-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  color: inherit;
  text-decoration: none;
}
.top {
  display: flex;
  justify-content: space-between;
  gap: var(--space-3);
}
.from {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}
</style>
