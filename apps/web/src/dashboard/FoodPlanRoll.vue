<script setup lang="ts">
// The kiosk's rolling meal plan: seven day rows like the Week view, two days back, today (inverted
// header) and four ahead; a column per meal slot. Read-only, and it never scrolls. A meal with a
// recipe opens the full-screen recipe, as on Home.
import { computed, ref, watch } from 'vue'
import { foodKiosk } from '../api/food'
import { entryTitle } from '../components/food/plan/planModel'
import { useMealPlanStore } from '../stores/mealPlan'
import { trimStyle } from '../lib/eventColors'
import { rollingDays } from './rollModel'
import { S } from './strings'
import UiIcon from '../ui/UiIcon.vue'
import RecipeDetail from './details/RecipeDetail.vue'

const props = defineProps<{
  /** `YYYY-MM-DD` in the household's zone. */
  today: string
  slots: { id: string; name: string; time?: string }[]
}>()

const store = useMealPlanStore()
/** The recipe open in the full-screen modal. */
const open = ref<{ id: string; title: string } | null>(null)
const days = computed(() => rollingDays(props.today))
const rows = computed(() =>
  days.value.map((key) => {
    const d = new Date(`${key}T12:00:00Z`)
    const fmt = (o: Intl.DateTimeFormatOptions) =>
      new Intl.DateTimeFormat(undefined, { timeZone: 'UTC', ...o }).format(d)
    return {
      key,
      today: key === props.today,
      weekday: fmt({ weekday: 'short' }),
      month: fmt({ month: 'short' }),
      dom: d.getUTCDate(),
    }
  }),
)
watch(
  days,
  (d) => {
    store.useApi(foodKiosk)
    void store.load(d[0]!, d[d.length - 1]!)
  },
  { immediate: true },
)
</script>

<template>
  <section class="roll" :aria-label="S.foodPlan" data-testid="food-roll">
    <table role="table" :style="{ '--cols': Math.max(slots.length, 1) }">
      <thead role="rowgroup">
        <tr role="row">
          <th scope="col" role="columnheader" class="day"></th>
          <th v-for="s in slots" :key="s.id" scope="col" role="columnheader">{{ s.name }}</th>
        </tr>
      </thead>
      <tbody role="rowgroup">
        <tr v-for="r in rows" :key="r.key" role="row" :class="{ today: r.today }" :data-day="r.key">
          <th scope="row" role="rowheader" class="day">
            <div class="date">
              <span class="dom">{{ r.dom }}</span>
              <span class="wd">{{ r.weekday }}</span>
            </div>
            <div v-if="r.dom === 1" class="mon">{{ r.month }}</div>
          </th>
          <td
            v-for="s in slots"
            :key="s.id"
            role="cell"
            :class="{ past: r.key < today }"
            :data-testid="`roll-cell-${r.key}-${s.name.toLowerCase()}`"
          >
            <ul>
              <li
                v-for="e in store.cell(r.key, s.id)"
                :key="e.id"
                data-testid="roll-entry"
                :style="trimStyle(r.key < today ? 'var(--color-muted)' : 'var(--color-accent)')"
              >
                <button
                  v-if="e.recipe_id"
                  type="button"
                  class="meal"
                  data-testid="roll-recipe"
                  :aria-label="`Open recipe: ${entryTitle(e)}`"
                  @click="open = { id: e.recipe_id, title: entryTitle(e) }"
                >
                  <span class="t">{{ entryTitle(e) }}</span>
                  <UiIcon name="recipe" :size="24" class="ic" />
                </button>
                <span v-else class="t">{{ entryTitle(e) }}</span>
              </li>
            </ul>
          </td>
        </tr>
      </tbody>
    </table>
    <RecipeDetail v-if="open" :id="open.id" :title="open.title" @close="open = null" />
  </section>
</template>

<style scoped>
.roll {
  position: absolute;
  inset: 0;
}
table {
  height: 100%;
  display: flex;
  flex-direction: column;
  width: 100%;
  font-size: var(--font-size-sm);
}
tbody {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-auto-rows: minmax(0, 1fr);
}
tr {
  display: grid;
  grid-template-columns: 7.5em repeat(var(--cols), minmax(0, 1fr));
}
th,
td {
  padding: var(--space-2);
  border: 1px solid var(--color-border);
  vertical-align: top;
  text-align: left;
  min-width: 0;
  overflow: hidden;
}
thead th {
  font-size: var(--font-size-base);
  background: var(--color-surface);
}
.day {
  font-size: var(--font-size-base);
  font-weight: 400;
  background: var(--color-surface);
}
.date {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
}
.dom {
  font-size: 1.9rem;
  line-height: 1.1;
  font-weight: 700;
}
.mon {
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
  padding: var(--space-1) var(--space-2);
  border-left: 6px solid transparent;
  --ev-stripe-w: 6px;
  border-radius: var(--ui-radius-sm);
  background: var(--color-surface);
  /* Seven rows share the screen height: the text grows with it. */
  font-size: clamp(var(--font-size-base), 2.8vh, 2rem);
  line-height: 1.2;
  font-weight: 600;
  overflow-wrap: anywhere;
}
.meal {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  width: 100%;
  min-height: var(--min-touch);
  padding: 0;
  border: 0;
  background: none;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.meal .ic {
  flex: none;
  color: var(--color-accent);
}
.t {
  min-width: 0;
}
td.past {
  color: var(--color-muted);
}
/* Today's header is inverted (light on dark, dark on light) so it stands out. */
tr.today > .day {
  background: var(--color-text);
  color: var(--color-bg);
}
tr.today > .day .mon {
  color: inherit;
}
tr.today > * {
  border-top: 3px solid var(--color-accent);
  border-bottom: 3px solid var(--color-accent);
}
</style>
