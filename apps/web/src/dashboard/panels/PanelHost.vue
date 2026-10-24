<script setup lang="ts">
import { computed, type Component } from 'vue'
import type { PanelProps } from '../types'
import AdvisoriesPanel from './AdvisoriesPanel.vue'
import AgendaPanel from './AgendaPanel.vue'
import ClockPanel from './ClockPanel.vue'
import MembersPanel from './MembersPanel.vue'
import WeatherPanel from './WeatherPanel.vue'
import WeekPanel from './WeekPanel.vue'

const props = defineProps<PanelProps>()
defineEmits<{ dismiss: [id: string]; pick: [id: string] }>()

const REGISTRY: Record<string, Component> = {
  clock: ClockPanel,
  weather: WeatherPanel,
  advisories: AdvisoriesPanel,
  agenda: AgendaPanel,
  week: WeekPanel,
  members: MembersPanel,
}
/** Unknown kinds resolve to nothing, so an older client survives a newer layout. */
const comp = computed(() => REGISTRY[props.panel.kind] ?? null)
</script>

<template>
  <component
    :is="comp"
    v-if="comp"
    v-bind="props"
    @dismiss="(id: string) => $emit('dismiss', id)"
    @pick="(id: string) => $emit('pick', id)"
  />
</template>
