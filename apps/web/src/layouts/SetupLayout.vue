<template>
  <div class="app-layout">
    <a class="skip-link" href="#main">Skip to content</a>
    <header class="app-header"><strong>Family Assistant</strong></header>
    <main id="main" class="setup-main"><slot /></main>
  </div>
</template>

<style scoped>
.app-header {
  padding: 0.5rem 1rem;
  border-bottom: 1px solid var(--color-border);
}
.setup-main {
  padding: 1rem;
  max-width: 44rem;
  margin: 0 auto;
}
</style>
