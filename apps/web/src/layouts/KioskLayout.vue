<script setup lang="ts">
// The kiosk shell: exactly the visible viewport (`innerWidth` x `innerHeight`, which leaves out a
// browser bar where `100vh` does not), with the root font scaled so the dashboard fits it.
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { kioskRootPx } from '../kiosk/fit'

const size = ref<{ w: number; h: number } | null>(null)
function fit() {
  const w = window.innerWidth
  const h = window.innerHeight
  size.value = { w, h }
  document.documentElement.style.fontSize = `${kioskRootPx(w, h)}px`
}
onMounted(() => {
  fit()
  window.addEventListener('resize', fit)
  window.addEventListener('orientationchange', fit)
  window.visualViewport?.addEventListener('resize', fit)
})
onBeforeUnmount(() => {
  window.removeEventListener('resize', fit)
  window.removeEventListener('orientationchange', fit)
  window.visualViewport?.removeEventListener('resize', fit)
  document.documentElement.style.fontSize = ''
})
</script>

<template>
  <div
    class="kiosk-layout"
    data-testid="kiosk-layout"
    :style="size ? { width: `${size.w}px`, height: `${size.h}px` } : undefined"
  >
    <slot />
  </div>
</template>

<style scoped>
/* Fixed to the visible viewport; the inline size from `fit()` wins once mounted. */
.kiosk-layout {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  overflow: hidden;
  font-size: var(--kiosk-font-base);
}
</style>
