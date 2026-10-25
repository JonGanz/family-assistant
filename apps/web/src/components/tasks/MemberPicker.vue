<script setup lang="ts">
import Avatar from '../Avatar.vue'
import type { MemberRef } from '../../lib/tasks'

defineProps<{
  members: MemberRef[]
  preselected?: string | null
  title?: string
  size?: 'normal' | 'kiosk'
}>()
const emit = defineEmits<{ pick: [memberId: string]; cancel: [] }>()
</script>

<template>
  <div class="picker" :class="size ?? 'normal'" role="dialog" :aria-label="title ?? 'Who did it?'">
    <h2>{{ title ?? 'Who did it?' }}</h2>
    <div class="grid">
      <button
        v-for="m in members"
        :key="m.id"
        type="button"
        class="member"
        :class="{ pre: m.id === preselected }"
        :aria-pressed="m.id === preselected"
        @click="emit('pick', m.id)"
      >
        <Avatar :name="m.name" :color="m.color" :src="m.avatarUrl" size="4em" />
        <span>{{ m.name }}</span>
      </button>
    </div>
    <button type="button" class="cancel" @click="emit('cancel')">Cancel</button>
  </div>
</template>

<style scoped>
.picker {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-5);
  background: var(--color-bg);
  color: var(--color-text);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
}
h2 {
  margin: 0;
  font-size: var(--font-size-xl);
}
.grid {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: var(--space-3);
}
.member {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-2);
  min-width: var(--min-touch, 48px);
  min-height: var(--min-touch, 48px);
  padding: var(--space-3);
  background: var(--color-surface);
  border: 2px solid transparent;
  border-radius: var(--radius-md);
  color: inherit;
  font: inherit;
  cursor: pointer;
}
.member.pre {
  border-color: var(--color-accent);
}
.cancel {
  min-height: var(--min-touch, 48px);
  padding: 0 var(--space-5);
  background: none;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  color: var(--color-muted);
  font: inherit;
  cursor: pointer;
}
.kiosk h2 {
  font-size: var(--kiosk-font-base, 2rem);
}
.kiosk .member {
  min-width: 9rem;
  min-height: 9rem;
  font-size: var(--kiosk-font-small, 1.5rem);
}
.kiosk .cancel {
  min-height: 4rem;
  font-size: var(--kiosk-font-small, 1.5rem);
}
</style>
