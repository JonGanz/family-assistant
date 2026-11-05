<script setup lang="ts">
import { computed } from 'vue'
import type { SectionView } from '../../../api/client'

const props = defineProps<{ sections: SectionView[]; childFilter: boolean }>()
const first = computed(() =>
  props.childFilter ? props.sections.filter((s) => s.relevance !== 'other') : props.sections,
)
const rest = computed(() =>
  props.childFilter ? props.sections.filter((s) => s.relevance === 'other') : [],
)
</script>

<template>
  <div class="sections">
    <div v-for="(s, i) in first" :key="i" class="sec">
      <h3 v-if="s.title">
        {{ s.title }}
        <span v-if="s.relevance" class="rel">{{ s.relevance }}</span>
      </h3>
      <p class="text">{{ s.text }}</p>
    </div>
    <details v-if="rest.length">
      <summary>{{ rest.length }} other section{{ rest.length === 1 ? '' : 's' }}</summary>
      <div v-for="(s, i) in rest" :key="i" class="sec">
        <h3 v-if="s.title">{{ s.title }}</h3>
        <p class="text">{{ s.text }}</p>
      </div>
    </details>
  </div>
</template>

<style scoped>
.sections {
  display: grid;
  gap: var(--ui-space-3);
}
h3 {
  margin: 0;
  font: var(--ui-type-title-sm);
}
.rel {
  margin-left: var(--ui-space-2);
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-label-sm);
}
.text {
  margin: 0;
  font: var(--ui-type-body-md);
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
summary {
  min-height: var(--ui-touch);
  display: flex;
  align-items: center;
  cursor: pointer;
  color: var(--ui-primary);
  font: var(--ui-type-label-lg);
}
</style>
