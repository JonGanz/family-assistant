<script setup lang="ts">
import { computed, type Component } from 'vue'
import type { DetailRequest, PanelProps } from '../types'
import AgendaPanel from './AgendaPanel.vue'
import ClockPanel from './ClockPanel.vue'
import MemberTasksPanel from './MemberTasksPanel.vue'
import MembersPanel from './MembersPanel.vue'
import NowPanel from './NowPanel.vue'
import RoutinePanel from './RoutinePanel.vue'
import WeatherPanel from './WeatherPanel.vue'
import WeekPanel from './WeekPanel.vue'
import WeekTablePanel from './WeekTablePanel.vue'

const props = defineProps<PanelProps>()
defineEmits<{
  pick: [id: string]
  complete: [id: string]
  tick: [run: string, step: string, done: boolean]
  add: []
  detail: [d: DetailRequest]
}>()

const REGISTRY: Record<string, Component> = {
  clock: ClockPanel,
  weather: WeatherPanel,
  agenda: AgendaPanel,
  week: WeekPanel,
  /** A navigation view, not a configurable panel: it is not in `KNOWN_PANELS`. */
  week_table: WeekTablePanel,
  members: MembersPanel,
  now: NowPanel,
  member_tasks: MemberTasksPanel,
  routine: RoutinePanel,
}
/** Unknown kinds resolve to nothing, so an older client survives a newer layout. */
const comp = computed(() => REGISTRY[props.panel.kind] ?? null)
</script>

<template>
  <component
    :is="comp"
    v-if="comp"
    v-bind="props"
    @pick="(id: string) => $emit('pick', id)"
    @complete="(id: string) => $emit('complete', id)"
    @tick="(r: string, st: string, d: boolean) => $emit('tick', r, st, d)"
    @add="$emit('add')"
    @detail="(d: DetailRequest) => $emit('detail', d)"
  />
</template>
