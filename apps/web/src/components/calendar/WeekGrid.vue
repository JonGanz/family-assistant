<script setup lang="ts">
import { computed } from 'vue'
import { useBreakpoint } from '../../ui'
import { stripeStyle } from '../../lib/eventColors'
import { formatRange } from '../../lib/time'
import type { EventLike, MemberLike } from '../../lib/types'
import { buildWeekGrid, weekVisible } from './weekGrid'

const props = defineProps<{
  events: EventLike[]
  members: MemberLike[]
  /** Chosen member ids; empty shows every member's column. */
  chosen: string[]
  start: string
  tz: string
}>()
const emit = defineEmits<{
  select: [event: EventLike]
  /** "+N events" was clicked: that day's events in that cell's column. */
  more: [v: { day: string; events: EventLike[] }]
}>()
const bp = useBreakpoint()
const phone = computed(() => bp.value === 'compact')

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
            <div class="cellbox">
              <ul v-if="c.events.length" :aria-label="`${c.name}, ${r.weekday} ${r.dom}`">
                <li v-for="e in c.events.slice(0, weekVisible(c.events.length, phone))" :key="e.id">
                  <button
                    type="button"
                    class="ev"
                    :class="{ busy: e.visibility === 'busy' }"
                    :style="stripeStyle(e)"
                    @click="$emit('select', e)"
                  >
                    <span class="title">{{ title(e) }}</span>
                    <span class="time">{{ formatRange(e, tz) }}</span>
                  </button>
                </li>
                <li v-if="c.events.length > weekVisible(c.events.length, phone)">
                  <button
                    type="button"
                    class="more"
                    @click="emit('more', { day: r.key, events: c.events })"
                  >
                    +{{ c.events.length - weekVisible(c.events.length, phone) }} events
                  </button>
                </li>
              </ul>
            </div>
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
  --wg-day-w: 5.5rem;
  border-collapse: collapse;
  table-layout: fixed;
  font: var(--ui-type-body-md);
  /* Every day row has this exact height: 2 two-line items, a "+N events" line and the gaps. */
  --wg-row-h: 9.75rem;
}
tbody tr {
  height: var(--wg-row-h);
}
th,
td {
  border: 1px solid var(--ui-outline-variant);
  padding: var(--ui-space-1);
  vertical-align: top;
  text-align: left;
  min-width: 0;
}
tbody td {
  height: var(--wg-row-h);
  overflow: hidden;
}
/* A table row only ever grows past its height, so the content sits in a box of fixed height. */
.cellbox {
  height: calc(var(--wg-row-h) - 2 * var(--wg-pad, var(--ui-space-1)) - 2px);
  overflow: hidden;
}
thead th {
  background: var(--ui-surface-container-low);
  font: var(--ui-type-label-lg);
}
.day {
  width: var(--wg-day-w);
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
  min-width: 0;
  text-align: left;
  font: var(--ui-type-body-sm);
  color: var(--ui-on-surface);
  background-color: var(--ui-surface-container-low);
  border: 0;
  border-left: 4px solid transparent;
  --ev-stripe-w: 4px;
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
  font-weight: 700;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.time {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.more {
  background: none;
  border: 0;
  padding: 0 var(--ui-space-2);
  cursor: pointer;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
.more:focus-visible {
  outline: var(--ui-focus-ring);
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
/* Phones: the grid fits the screen like Month: a very narrow date column, equal member columns,
   one-line ellipsized items, smaller type, no sideways scroll. */
@media (max-width: 599px) {
  .wg {
    min-width: 0;
    --wg-day-w: 1.25rem;
    --wg-row-h: 4rem;
    --wg-pad: 1px;
    font-size: 0.75625rem;
  }
  .wg-scroll {
    overflow-x: hidden;
  }
  th,
  td {
    padding: 1px;
  }
  thead th {
    font-size: 0.625rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: center;
  }
  .day {
    position: static;
    padding: 1px 0;
    text-align: center;
    overflow: hidden;
  }
  .dom {
    font-size: 0.6875rem;
    font-weight: 600;
    line-height: 1;
  }
  .wd {
    display: block;
    overflow: hidden;
    font-size: 0.5625rem;
    line-height: 1.1;
  }
  ul {
    gap: 1px;
  }
  .cellbox {
    display: grid;
    align-content: center;
  }
  /* The global min touch height would make each one-line item several lines tall. */
  .ev,
  .more {
    min-height: 0;
  }
  .ev {
    font-size: 0.75625rem;
    line-height: 0.85rem;
    padding: 2px 2px 2px 5px;
    border-left-width: 2px;
  }
  .time {
    display: none;
  }
  .more {
    font-size: 0.75625rem;
    line-height: 0.85rem;
    padding: 0 2px;
  }
}
</style>
