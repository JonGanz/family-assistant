<script setup lang="ts">
import PageHeader from '../components/PageHeader.vue'
import MembersManager from './parts/MembersManager.vue'
</script>

<template>
  <section aria-labelledby="page-title">
    <PageHeader eyebrow="Household" title="Members" />
    <MembersManager />
  </section>
</template>
