<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useClock } from '../composables/useClock'
import DashboardCanvas from './DashboardCanvas.vue'
import type { PreviewData } from './preview'
import { S } from './strings'
import type { DashboardSettings } from './types'

const props = defineProps<{ data: PreviewData; settings: DashboardSettings }>()
const now = useClock(30_000)

/** Only the preview's own screen shape; a real screen's is detected. */
const portrait = ref(false)
const W = computed(() => (portrait.value ? 1080 : 1920))
const H = computed(() => (portrait.value ? 1920 : 1080))

const box = ref<HTMLElement | null>(null)
const width = ref(640)
let ro: ResizeObserver | undefined
onMounted(() => {
  if (box.value) {
    width.value = box.value.clientWidth || width.value
    if (typeof ResizeObserver !== 'undefined') {
      ro = new ResizeObserver(() => (width.value = box.value?.clientWidth || width.value))
      ro.observe(box.value)
    }
  }
})
onBeforeUnmount(() => ro?.disconnect())

const scale = computed(() => width.value / W.value)
const full = computed(() => ({ ...props.data, settings: props.settings }))
</script>

<template>
  <label class="shape"
    >{{ S.previewAs }}
    <select
      data-testid="preview-shape"
      :value="portrait ? 'portrait' : 'landscape'"
      @change="portrait = ($event.target as HTMLSelectElement).value === 'portrait'"
    >
      <option value="landscape">{{ S.landscape }}</option>
      <option value="portrait">{{ S.portrait }}</option>
    </select>
  </label>
  <div
    ref="box"
    class="preview"
    :style="{ height: `${H * scale}px` }"
    role="img"
    :aria-label="S.preview"
    data-testid="layout-preview"
  >
    <div class="stage" :style="{ width: `${W}px`, height: `${H}px`, transform: `scale(${scale})` }">
      <DashboardCanvas :data="full" :now="now" />
    </div>
  </div>
</template>

<style scoped>
.preview {
  position: relative;
  width: 100%;
  max-width: 960px;
  overflow: hidden;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  pointer-events: none;
}
.stage {
  position: absolute;
  top: 0;
  left: 0;
  transform-origin: top left;
}
</style>
