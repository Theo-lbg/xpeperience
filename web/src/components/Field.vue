<script setup lang="ts">
defineProps<{ label: string; error?: string; hint?: string; required?: boolean; for?: string }>()
</script>

<template>
  <div class="field">
    <label :for="$props.for">{{ label }}<span v-if="required" class="req" aria-hidden="true">*</span></label>
    <slot />
    <span v-if="error" class="err" role="alert">{{ error }}</span>
    <span v-else-if="hint" class="hint">{{ hint }}</span>
  </div>
</template>
