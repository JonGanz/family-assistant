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
  <div>
    <div v-for="(s, i) in first" :key="i" class="sec">
      <h3 v-if="s.title">
        {{ s.title }}
        <span v-if="s.relevance" class="badge">{{ s.relevance }}</span>
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
.text {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
</style>
