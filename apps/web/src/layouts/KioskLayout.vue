<template>
  <div class="kiosk-layout"><slot /></div>
</template>

<style scoped>
.kiosk-layout {
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  font-size: var(--kiosk-font-base);
  cursor: none;
}
</style>
