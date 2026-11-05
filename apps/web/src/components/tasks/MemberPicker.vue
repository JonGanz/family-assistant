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
  gap: var(--ui-space-4);
  padding: var(--ui-space-5);
  background: var(--ui-surface-container-high);
  color: var(--ui-on-surface);
  border-radius: var(--ui-radius-lg);
}
h2 {
  margin: 0;
  font: var(--ui-type-headline-sm);
}
.grid {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: var(--ui-space-3);
}
.member {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--ui-space-2);
  min-width: var(--ui-touch);
  min-height: var(--ui-touch);
  padding: var(--ui-space-3);
  background: var(--ui-surface-container-low);
  border: 2px solid transparent;
  border-radius: var(--ui-radius-md);
  color: inherit;
  font: var(--ui-type-label-lg);
  cursor: pointer;
}
.member.pre {
  border-color: var(--ui-primary);
}
.cancel {
  min-height: var(--ui-touch);
  padding: 0 var(--ui-space-5);
  background: none;
  border: 1px solid var(--ui-outline);
  border-radius: var(--ui-radius-full);
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-label-lg);
  cursor: pointer;
}
.kiosk h2 {
  font-size: var(--kiosk-font-base);
}
.kiosk .member {
  min-width: 9rem;
  min-height: 9rem;
  font-size: var(--kiosk-font-small);
}
.kiosk .cancel {
  min-height: 4rem;
  font-size: var(--kiosk-font-small);
}
</style>
