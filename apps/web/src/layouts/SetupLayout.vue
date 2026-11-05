<template>
  <div class="setup-layout">
    <a class="skip-link" href="#main">Skip to content</a>
    <header class="setup-header"><strong>Family Assistant</strong></header>
    <main id="main" class="setup-main"><slot /></main>
  </div>
</template>

<style scoped>
.setup-layout {
  min-height: 100dvh;
  background: var(--ui-surface);
  color: var(--ui-on-surface);
}
.setup-header {
  padding: var(--ui-space-3) var(--ui-space-4);
  font: var(--ui-type-title-md);
  border-bottom: 1px solid var(--ui-outline-variant);
}
.setup-main {
  padding: var(--ui-space-4);
  max-width: 44rem;
  margin: 0 auto;
}
</style>
