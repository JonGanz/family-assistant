<script setup lang="ts">
import { computed } from 'vue'
import { textColorFor } from '../../lib/contrast'
import { dayKey, formatRange, groupByDay } from '../../lib/time'
import { toEventLike, visibleEvents } from '../filter'
import { S } from '../strings'
import type { PanelProps } from '../types'

const props = defineProps<PanelProps>()
const tz = computed(() => props.data.household.tz)
const today = computed(() => dayKey(props.now, tz.value))
const cols = computed(() => {
  const events = visibleEvents(props.data, props.settings).map(toEventLike)
  return [...groupByDay(events, tz.value, today.value, 7)].map(([key, list]) => {
    const d = new Date(`${key}T12:00:00Z`)
    return {
      key,
      weekday: new Intl.DateTimeFormat(undefined, { weekday: 'short', timeZone: 'UTC' }).format(d),
      dom: d.getUTCDate(),
      today: key === today.value,
      events: list,
    }
  })
})
</script>

<template>
  <section class="week" aria-label="Week">
    <div v-for="c in cols" :key="c.key" class="col" :class="{ today: c.today }" :data-day="c.key">
      <h2>
        <span>{{ c.weekday }}</span> <span class="dom">{{ c.dom }}</span>
      </h2>
      <ul>
        <li
          v-for="(e, i) in c.events"
          :key="e.id ?? i"
          :class="{ busy: e.visibility === 'busy' }"
          :style="
            e.visibility === 'busy'
              ? undefined
              : { background: e.color, color: textColorFor(e.color) }
          "
        >
          <span class="t">{{ formatRange(e, tz) }}</span>
          <span class="n">{{ e.visibility === 'busy' ? S.busy : e.title }}</span>
        </li>
      </ul>
    </div>
  </section>
</template>

<style scoped>
.week {
  display: grid;
  grid-template-columns: repeat(7, minmax(0, 1fr));
  gap: var(--space-2);
  height: 100%;
}
.col {
  background: var(--color-surface);
  border-radius: var(--radius-md);
  padding: var(--space-2);
  min-width: 0;
  overflow: hidden;
}
.col.today {
  outline: 3px solid var(--color-accent);
}
h2 {
  margin: 0 0 var(--space-2);
  font-size: var(--font-size-base);
  font-weight: 600;
}
.dom {
  color: var(--color-muted);
}
ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--space-1);
}
li {
  display: grid;
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-sm);
  overflow-wrap: anywhere;
}
li.busy {
  background: var(--color-border);
  color: var(--color-muted);
}
.t {
  opacity: 0.85;
}
.n {
  font-weight: 600;
}
</style>
