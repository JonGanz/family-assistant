<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { foodKiosk, type RecipeView } from '../../api/food'
import { getDeviceToken } from '../../kiosk/deviceToken'
import { S } from '../strings'

// Local strings: strings.ts belongs to another task.
const T = { ingredients: 'Ingredients', steps: 'Steps', failed: 'The recipe could not be loaded' }

const props = defineProps<{ id: string; title: string }>()
const emit = defineEmits<{ close: [] }>()

const recipe = ref<RecipeView | null>(null)
const failed = ref(false)
const imageSrc = ref<string | null>(null)
const lines = computed(() => (recipe.value?.lines ?? []).map((l) => l.raw_text))

/** Same idle close as Modal.vue, so a forgotten recipe does not stay on the wall. */
const IDLE_CLOSE_MS = 5 * 60_000
let timer: ReturnType<typeof setTimeout> | undefined
function arm() {
  clearTimeout(timer)
  timer = setTimeout(() => emit('close'), IDLE_CLOSE_MS)
}

onMounted(async () => {
  arm()
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
  clearTimeout(timer)
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
      @pointerdown.passive="arm"
      @scroll.capture.passive="arm"
    >
      <header>
        <h2>{{ recipe?.title ?? title }}</h2>
        <button type="button" class="close" :aria-label="S.close" @click="emit('close')">✕</button>
      </header>
      <div class="body">
        <p v-if="failed" class="none" data-testid="recipe-error">{{ T.failed }}</p>
        <p v-else-if="!recipe" class="none">{{ S.loading }}</p>
        <template v-else>
          <img v-if="imageSrc" :src="imageSrc" alt="" class="photo" data-testid="recipe-image" />
          <section class="card">
            <h3>{{ T.ingredients }}</h3>
            <ul data-testid="recipe-lines">
              <li v-for="(l, i) in lines" :key="i">{{ l }}</li>
            </ul>
          </section>
          <section class="card">
            <h3>{{ T.steps }}</h3>
            <ol data-testid="recipe-steps">
              <li v-for="(s, i) in recipe.steps" :key="i">{{ s }}</li>
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
.photo {
  width: 100%;
  max-height: 40vh;
  object-fit: cover;
  border-radius: var(--ui-radius-lg, var(--ui-radius-sm));
}
.card {
  padding: var(--space-3) var(--space-4);
  border-radius: var(--ui-radius-lg, var(--ui-radius-sm));
  background: var(--color-layer-2, var(--color-surface));
}
ul,
ol {
  margin: 0;
  padding-left: var(--space-5);
  display: grid;
  gap: var(--space-2);
}
.none {
  margin: 0;
  color: var(--color-muted);
}
</style>
