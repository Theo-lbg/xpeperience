<script setup lang="ts">
import EmptyState from '../components/EmptyState.vue'
</script>

<template>
  <div class="page">
    <EmptyState icon="search" title="Page introuvable">
      Cette page n'existe pas ou n'est plus accessible.
      <template #actions><RouterLink class="btn primary" to="/">Retour au tableau de bord</RouterLink></template>
    </EmptyState>
  </div>
</template>
