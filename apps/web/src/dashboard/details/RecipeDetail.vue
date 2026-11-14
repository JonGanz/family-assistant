<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { foodKiosk, type RecipeView } from '../../api/food'
import { getDeviceToken } from '../../kiosk/deviceToken'
import { S } from '../strings'
import {
  emptyChecks,
  loadChecks,
  pruneChecks,
  saveChecks,
  toggleCheck,
  type Checks,
} from './recipeChecks'

// Local strings: strings.ts belongs to another task.
const T = {
  ingredients: 'Ingredients',
  steps: 'Steps',
  failed: 'The recipe could not be loaded',
  details: 'Details',
  prep: 'Prep',
  cook: 'Cook',
  total: 'Total',
  servings: 'Servings',
  yield: 'Yield',
  source: 'Source',
  note: 'Note',
  reset: 'Reset ticks',
}

const props = defineProps<{ id: string; title: string }>()
const emit = defineEmits<{ close: [] }>()

const recipe = ref<RecipeView | null>(null)
const failed = ref(false)
const imageSrc = ref<string | null>(null)
const checks = ref<Checks>(emptyChecks())
const lines = computed(() => (recipe.value?.lines ?? []).map((l) => l.raw_text))

const mins = (n: number) =>
  n >= 60 ? `${Math.floor(n / 60)} h${n % 60 ? ` ${n % 60} min` : ''}` : `${n} min`
function host(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, '')
  } catch {
    return url
  }
}
const facts = computed(() => {
  const r = recipe.value
  if (!r) return []
  const out: { k: string; v: string }[] = []
  if (r.prep_minutes) out.push({ k: T.prep, v: mins(r.prep_minutes) })
  if (r.cook_minutes) out.push({ k: T.cook, v: mins(r.cook_minutes) })
  if (r.total_minutes) out.push({ k: T.total, v: mins(r.total_minutes) })
  if (r.servings) out.push({ k: T.servings, v: String(r.servings) })
  if (r.yield_text) out.push({ k: T.yield, v: r.yield_text })
  if (r.source_url) out.push({ k: T.source, v: host(r.source_url) })
  if (r.note) out.push({ k: T.note, v: r.note })
  return out
})
const anyChecked = computed(() => checks.value.lines.length + checks.value.steps.length > 0)

function toggle(kind: 'lines' | 'steps', i: number) {
  checks.value = toggleCheck(checks.value, kind, i)
  saveChecks(props.id, checks.value)
}
function reset() {
  checks.value = emptyChecks()
  saveChecks(props.id, checks.value)
}

// No idle auto-close: the recipe stays until the user closes it (the screensaver may cover it).
onMounted(async () => {
  pruneChecks()
  checks.value = loadChecks(props.id)
  try {
    recipe.value = await foodKiosk.recipe(props.id)
  } catch {
    failed.value = true
    return
  }
  if (!recipe.value.has_image) return
  // An <img src> cannot send the device token, so fetch the picture and show a blob URL.
  try {
    const token = getDeviceToken()
    const res = await fetch(foodKiosk.recipeImageUrl(props.id), {
      headers: token ? { Authorization: `Bearer ${token}` } : {},
      credentials: 'include',
    })
    if (res.ok) imageSrc.value = URL.createObjectURL(await res.blob())
  } catch {
    /* no picture is fine */
  }
})
onBeforeUnmount(() => {
  if (imageSrc.value) URL.revokeObjectURL(imageSrc.value)
})
</script>

<template>
  <Teleport to="body">
    <div
      class="full"
      role="dialog"
      aria-modal="true"
      :aria-label="recipe?.title ?? title"
      data-testid="recipe-detail"
    >
      <header>
        <h2>{{ recipe?.title ?? title }}</h2>
        <button
          v-if="anyChecked"
          type="button"
          class="reset"
          data-testid="recipe-reset"
          @click="reset"
        >
          {{ T.reset }}
        </button>
        <button type="button" class="close" :aria-label="S.close" @click="emit('close')">✕</button>
      </header>
      <div class="body">
        <p v-if="failed" class="none" data-testid="recipe-error">{{ T.failed }}</p>
        <p v-else-if="!recipe" class="none">{{ S.loading }}</p>
        <template v-else>
          <div class="top" :class="{ solo: !imageSrc || !facts.length }">
            <div v-if="imageSrc" class="card imgcard">
              <img :src="imageSrc" alt="" class="photo" data-testid="recipe-image" />
            </div>
            <section v-if="facts.length" class="card facts" data-testid="recipe-facts">
              <h3>{{ T.details }}</h3>
              <dl>
                <template v-for="f in facts" :key="f.k">
                  <dt>{{ f.k }}</dt>
                  <dd>{{ f.v }}</dd>
                </template>
              </dl>
            </section>
          </div>
          <section class="card">
            <h3>{{ T.ingredients }}</h3>
            <ul data-testid="recipe-lines">
              <li v-for="(l, i) in lines" :key="i">
                <label :class="{ done: checks.lines.includes(i) }">
                  <input
                    type="checkbox"
                    :checked="checks.lines.includes(i)"
                    data-testid="recipe-line-check"
                    @change="toggle('lines', i)"
                  />
                  <span>{{ l }}</span>
                </label>
              </li>
            </ul>
          </section>
          <section class="card steps">
            <h3>{{ T.steps }}</h3>
            <ol data-testid="recipe-steps">
              <li v-for="(s, i) in recipe.steps" :key="i">
                <label :class="{ done: checks.steps.includes(i) }">
                  <input
                    type="checkbox"
                    :checked="checks.steps.includes(i)"
                    data-testid="recipe-step-check"
                    @change="toggle('steps', i)"
                  />
                  <span>{{ s }}</span>
                </label>
              </li>
            </ol>
          </section>
        </template>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.full {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  flex-direction: column;
  background: var(--color-bg);
  color: var(--color-text);
}
header {
  flex: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-5);
  background: var(--color-layer-1, var(--color-surface));
}
h2 {
  margin: 0;
  font-size: var(--font-size-xl, var(--font-size-lg));
}
h3 {
  margin: 0 0 var(--space-2);
  font-size: var(--font-size-lg);
}
.close {
  min-width: var(--min-touch);
  min-height: var(--min-touch);
  border: 0;
  border-radius: var(--ui-radius-lg, var(--ui-radius-sm));
  background: var(--color-layer-2, var(--color-surface));
  color: inherit;
  font: inherit;
  cursor: pointer;
}
.body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: grid;
  gap: var(--space-4);
  align-content: start;
  padding: var(--space-4) var(--space-5);
  font-size: var(--font-size-lg);
}
.top {
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: var(--space-4);
}
.top.solo {
  grid-template-columns: 1fr;
}
/* Side by side on any wall screen; only a phone-narrow window stacks. */
@media (max-width: 600px) {
  .top {
    grid-template-columns: 1fr;
  }
}
.imgcard {
  padding: 0;
  overflow: hidden;
}
.photo {
  display: block;
  width: 100%;
  height: 22vh;
  object-fit: cover;
}
.card {
  padding: var(--space-3) var(--space-4);
  border-radius: var(--ui-radius-lg, var(--ui-radius-sm));
  background: var(--color-layer-2, var(--color-surface));
}
.steps {
  margin-top: var(--space-5);
}
dl {
  margin: 0;
  display: grid;
  grid-template-columns: auto 1fr;
  gap: var(--space-1, 4px) var(--space-3);
}
dt {
  color: var(--color-muted);
}
dd {
  margin: 0;
  overflow-wrap: anywhere;
}
ul,
ol {
  margin: 0;
  padding: 0;
  list-style: none;
  display: grid;
  gap: var(--space-2);
}
ol {
  counter-reset: step;
}
ol li {
  counter-increment: step;
}
ol li span::before {
  content: counter(step) '. ';
  font-weight: 600;
}
label {
  display: flex;
  align-items: flex-start;
  gap: var(--space-3);
  min-height: var(--min-touch);
  padding: var(--space-1, 4px) 0;
  cursor: pointer;
}
label input {
  flex: none;
  width: var(--min-touch);
  height: var(--min-touch);
  margin: 0;
  accent-color: var(--color-accent, currentColor);
}
label span {
  align-self: center;
}
label.done span {
  text-decoration: line-through;
  opacity: 0.5;
}
.reset {
  margin-left: auto;
  min-height: var(--min-touch);
  padding: 0 var(--space-4);
  border: 0;
  border-radius: var(--ui-radius-lg, var(--ui-radius-sm));
  background: var(--color-layer-2, var(--color-surface));
  color: inherit;
  font: inherit;
  cursor: pointer;
}
.none {
  margin: 0;
  color: var(--color-muted);
}
</style>
