<script setup lang="ts">
import Icon from './Icon.vue'

defineProps<{ icon?: string; title: string; compact?: boolean }>()
</script>

<template>
  <div class="empty" :class="{ compact }">
    <div v-if="!compact" class="art"><Icon :name="icon ?? 'sparkle'" /></div>
    <h3>{{ title }}</h3>
    <p v-if="$slots.default"><slot /></p>
    <div v-if="$slots.actions" class="row wrap" style="justify-content: center"><slot name="actions" /></div>
  </div>
</template>
