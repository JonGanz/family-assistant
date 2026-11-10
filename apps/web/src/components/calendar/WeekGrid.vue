<script setup lang="ts">
import { computed } from 'vue'
import { stripeStyle } from '../../lib/eventColors'
import { formatRange } from '../../lib/time'
import type { EventLike, MemberLike } from '../../lib/types'
import { buildWeekGrid } from './weekGrid'

const props = defineProps<{
  events: EventLike[]
  members: MemberLike[]
  /** Chosen member ids; empty shows every member's column. */
  chosen: string[]
  start: string
  tz: string
}>()
defineEmits<{ select: [event: EventLike] }>()

const grid = computed(() =>
  buildWeekGrid({
    events: props.events,
    members: props.members,
    chosen: props.chosen,
    start: props.start,
    tz: props.tz,
  }),
)
const title = (e: EventLike) => (e.visibility === 'busy' ? 'Busy' : (e.title ?? ''))
</script>

<template>
  <div class="wg-scroll">
    <table class="wg" role="table" aria-label="Week" :style="{ '--cols': grid.columns.length }">
      <thead role="rowgroup">
        <tr role="row">
          <th scope="col" role="columnheader" class="day"></th>
          <th v-for="c in grid.columns" :key="c.key" scope="col" role="columnheader">
            {{ c.name }}
          </th>
        </tr>
      </thead>
      <tbody role="rowgroup">
        <tr
          v-for="r in grid.rows"
          :key="r.key"
          role="row"
          :class="{ today: r.today }"
          :data-day="r.key"
        >
          <th scope="row" role="rowheader" class="day">
            <span class="dom">{{ r.dom }}</span>
            <span class="wd">{{ r.weekday }}</span>
          </th>
          <td v-for="c in r.cells" :key="c.key" role="cell" :data-member="c.key">
            <ul v-if="c.events.length" :aria-label="`${c.name}, ${r.weekday} ${r.dom}`">
              <li v-for="e in c.events" :key="e.id">
                <button
                  type="button"
                  class="ev"
                  :class="{ busy: e.visibility === 'busy' }"
                  :style="stripeStyle(e)"
                  @click="$emit('select', e)"
                >
                  <span class="time">{{ formatRange(e, tz) }}</span>
                  <span class="title">{{ title(e) }}</span>
                </button>
              </li>
            </ul>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
/* On narrow screens the grid scrolls sideways inside this box; the day column stays put. */
.wg-scroll {
  overflow-x: auto;
  border: 1px solid var(--ui-outline-variant);
  border-radius: var(--ui-radius-md);
}
.wg {
  width: 100%;
  min-width: calc(5.5rem + var(--cols) * 9rem);
  border-collapse: collapse;
  table-layout: fixed;
  font: var(--ui-type-body-md);
}
th,
td {
  border: 1px solid var(--ui-outline-variant);
  padding: var(--ui-space-1);
  vertical-align: top;
  text-align: left;
  min-width: 0;
}
thead th {
  background: var(--ui-surface-container-low);
  font: var(--ui-type-label-lg);
}
.day {
  width: 5.5rem;
  position: sticky;
  left: 0;
  z-index: 1;
  background: var(--ui-surface-container-low);
  font-weight: 400;
}
.dom {
  display: block;
  font: var(--ui-type-title-lg);
}
.wd {
  color: var(--ui-on-surface-variant);
}
tr.today > .day {
  background: var(--ui-primary);
  color: var(--ui-on-primary);
}
tr.today > .day .wd {
  color: inherit;
}
tr.today > td {
  background: var(--ui-primary-container);
}
ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--ui-space-1);
}
.ev {
  --ev-stripe-w: 4px;
  display: grid;
  width: 100%;
  text-align: left;
  font: var(--ui-type-body-sm);
  color: var(--ui-on-surface);
  background-color: var(--ui-surface-container-low);
  border: 0;
  border-left: 4px solid var(--ui-primary);
  border-radius: var(--ui-radius-sm);
  padding: var(--ui-space-1) var(--ui-space-2);
  cursor: pointer;
}
.ev:hover {
  background-color: var(--ui-surface-container-high);
}
.ev:focus-visible {
  outline: var(--ui-focus-ring);
}
.time {
  color: var(--ui-on-surface-variant);
}
.title {
  font-weight: 600;
  overflow-wrap: anywhere;
}
.ev.busy {
  --ev-bg: repeating-linear-gradient(
    45deg,
    transparent,
    transparent 4px,
    var(--ui-outline-variant) 4px,
    var(--ui-outline-variant) 6px
  );
  background-image: var(--ev-bg);
  color: var(--ui-on-surface-variant);
}
</style>
