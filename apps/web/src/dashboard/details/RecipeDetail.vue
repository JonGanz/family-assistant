<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { foodKiosk, type RecipeLineView, type RecipeView } from '../../api/food'
import { groupRecipe } from '../../lib/recipeSections'
import { formatQty, scaleLine } from '../../lib/recipeScale'
import { getDeviceToken } from '../../kiosk/deviceToken'
import Modal from '../Modal.vue'
import { S } from '../strings'
import {
  emptyChecks,
  loadChecks,
  pruneChecks,
  saveChecks,
  toggleCheck,
  type Checks,
} from './recipeChecks'

const T = S.recipe

const props = defineProps<{ id: string; title: string }>()
const emit = defineEmits<{ close: [] }>()

const recipe = ref<RecipeView | null>(null)
const failed = ref(false)
const imageSrc = ref<string | null>(null)
const checks = ref<Checks>(emptyChecks())
const scale = ref(1)
const factors = computed(() => {
  const f = new Set([...(recipe.value?.scale_factors ?? []), 1])
  return [...f].sort((a, b) => a - b)
})
const grouped = computed(() => (recipe.value ? groupRecipe(recipe.value) : null))
const hasSections = computed(() => (grouped.value?.sections.length ?? 0) > 0)
const lineText = (l: RecipeLineView) => scaleLine(l, scale.value)

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
  if (r.servings) out.push({ k: T.servings, v: formatQty(r.servings * scale.value) })
  if (r.yield_text) out.push({ k: T.yield, v: r.yield_text })
  if (r.source_url) out.push({ k: T.source, v: host(r.source_url) })
  if (r.note) out.push({ k: T.note, v: r.note })
  return out
})
const anyChecked = computed(() => checks.value.lines.length + checks.value.steps.length > 0)

function toggle(kind: 'lines' | 'steps', id: string) {
  checks.value = toggleCheck(checks.value, kind, id)
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
    <div class="cover" data-testid="recipe-detail">
      <Modal :title="recipe?.title ?? title" fullscreen :idle-close="false" @close="emit('close')">
        <template #actions>
          <div
            v-if="recipe"
            class="scale"
            role="group"
            :aria-label="T.scale"
            data-testid="recipe-scale"
          >
            <button
              v-for="f in factors"
              :key="f"
              type="button"
              :class="{ on: f === scale }"
              :aria-pressed="f === scale"
              data-testid="recipe-scale-btn"
              @click="scale = f"
            >
              {{ formatQty(f) }}×
            </button>
          </div>
          <button
            v-if="anyChecked"
            type="button"
            class="reset"
            data-testid="recipe-reset"
            @click="reset"
          >
            {{ T.reset }}
          </button>
        </template>
        <div class="content">
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
            <section v-if="grouped!.shared.length" class="card" data-testid="recipe-shared">
              <h3>{{ T.ingredients }}</h3>
              <ul data-testid="recipe-lines" class="cols">
                <li v-for="l in grouped!.shared" :key="l.id">
                  <label :class="{ done: checks.lines.includes(l.id) }">
                    <input
                      type="checkbox"
                      :checked="checks.lines.includes(l.id)"
                      data-testid="recipe-line-check"
                      @change="toggle('lines', l.id)"
                    />
                    <span>{{ lineText(l) }}</span>
                  </label>
                </li>
              </ul>
            </section>
            <div
              v-if="hasSections"
              class="sections"
              data-testid="recipe-sections"
              :style="{ '--cols': recipe.section_columns }"
            >
              <div v-for="g in grouped!.sections" :key="g.section.id" class="col">
                <section class="card" data-testid="recipe-section">
                  <h3>{{ g.section.name }}</h3>
                  <ul data-testid="recipe-lines">
                    <li v-for="l in g.lines" :key="l.id">
                      <label :class="{ done: checks.lines.includes(l.id) }">
                        <input
                          type="checkbox"
                          :checked="checks.lines.includes(l.id)"
                          data-testid="recipe-line-check"
                          @change="toggle('lines', l.id)"
                        />
                        <span>{{ lineText(l) }}</span>
                      </label>
                    </li>
                  </ul>
                </section>
                <section v-if="g.steps.length" class="card steps">
                  <ol data-testid="recipe-steps">
                    <li v-for="s in g.steps" :key="s.id">
                      <label :class="{ done: checks.steps.includes(s.id) }">
                        <input
                          type="checkbox"
                          :checked="checks.steps.includes(s.id)"
                          data-testid="recipe-step-check"
                          @change="toggle('steps', s.id)"
                        />
                        <span>{{ s.text }}</span>
                      </label>
                    </li>
                  </ol>
                </section>
              </div>
            </div>
            <section v-if="grouped!.finish.length" class="card steps">
              <h3>{{ hasSections ? T.finish : T.steps }}</h3>
              <ol data-testid="recipe-steps">
                <li v-for="s in grouped!.finish" :key="s.id">
                  <label :class="{ done: checks.steps.includes(s.id) }">
                    <input
                      type="checkbox"
                      :checked="checks.steps.includes(s.id)"
                      data-testid="recipe-step-check"
                      @change="toggle('steps', s.id)"
                    />
                    <span>{{ s.text }}</span>
                  </label>
                </li>
              </ol>
            </section>
          </template>
        </div>
      </Modal>
    </div>
  </Teleport>
</template>

<style scoped>
.cover {
  position: fixed;
  inset: 0;
  z-index: 50;
}
h3 {
  margin: 0 0 var(--space-2);
  font-size: var(--font-size-lg);
}
.content {
  display: grid;
  gap: var(--space-4);
  align-content: start;
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
  position: relative;
  padding: 0;
  overflow: hidden;
  min-height: 22vh;
}
.top.solo .imgcard {
  min-height: 30vh;
}
@media (max-width: 600px) {
  .imgcard {
    min-height: 30vh;
  }
}
.photo {
  position: absolute;
  inset: 0;
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.card {
  padding: var(--space-3) var(--space-4);
  border-radius: var(--ui-radius-lg, var(--ui-radius-sm));
  background: var(--color-layer-2, var(--color-surface));
}
.steps {
  margin-top: var(--space-5);
  font-size: calc(var(--font-size-lg) * 1.25);
}
ul.cols {
  display: block;
  columns: 2;
  column-gap: var(--space-5, 24px);
}
ul.cols li {
  break-inside: avoid;
  margin-bottom: var(--space-2);
}
@media (max-width: 600px) {
  ul.cols {
    columns: 1;
  }
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
.scale {
  display: flex;
  gap: var(--space-1, 4px);
  margin-left: auto;
}
.scale button {
  min-width: var(--min-touch);
  min-height: var(--min-touch);
  padding: 0 var(--space-3);
  border: 0;
  border-radius: var(--ui-radius-lg, var(--ui-radius-sm));
  background: var(--color-layer-2, var(--color-surface));
  color: inherit;
  font: inherit;
  cursor: pointer;
}
.scale button.on {
  background: var(--color-accent, currentColor);
  color: var(--color-on-accent, var(--color-bg));
}
.sections {
  display: grid;
  grid-template-columns: repeat(var(--cols, 2), minmax(0, 1fr));
  gap: var(--space-4);
  align-items: start;
}
.col {
  display: grid;
  gap: var(--space-3);
  align-content: start;
}
.col .steps {
  margin-top: 0;
}
@media (max-width: 600px) {
  .sections {
    grid-template-columns: 1fr;
  }
}
.reset {
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
