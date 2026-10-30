<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import Avatar from '../../components/Avatar.vue'
import { hueVar } from '../../lib/hues'
import { avatarSrc } from '../avatars'
import { approvalCounts, visibleMembers } from '../filter'
import { S } from '../strings'
import type { PanelProps } from '../types'

const props = defineProps<PanelProps>()
defineEmits<{ pick: [id: string] }>()
const members = computed(() => visibleMembers(props.data, props.settings))
const counts = computed(() => approvalCounts(props.data))
const srcs = ref<Record<string, string | null>>({})
watch(
  members,
  (list) => {
    for (const m of list) {
      void avatarSrc(m.avatar_url).then((u) => (srcs.value = { ...srcs.value, [m.id]: u }))
    }
  },
  { immediate: true },
)
</script>

<template>
  <section class="members" aria-label="Members">
    <ul>
      <li v-for="m in members" :key="m.id">
        <button type="button" :data-member="m.id" @click="$emit('pick', m.id)">
          <Avatar :name="m.display_name" :color="hueVar(m.color)" :src="srcs[m.id]" size="2.2em" />
          <span>{{ m.display_name }}</span>
          <span
            v-if="counts[m.id]"
            class="approvals"
            :data-testid="`approvals-${m.id}`"
            :aria-label="S.approvalsWaiting(counts[m.id]!)"
            >{{ counts[m.id] }}</span
          >
        </button>
      </li>
    </ul>
  </section>
</template>

<style scoped>
ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
}
.approvals {
  min-width: 1.6em;
  padding: 0 var(--space-2);
  border-radius: 999px;
  background: var(--color-accent);
  color: var(--color-bg);
  font-size: var(--font-size-sm);
  text-align: center;
}
button {
  display: grid;
  justify-items: center;
  gap: var(--space-1);
  min-width: var(--min-touch, 48px);
  min-height: var(--min-touch, 48px);
  background: transparent;
  border: 0;
  color: inherit;
  font-size: var(--font-size-sm);
  cursor: pointer;
}
</style>
