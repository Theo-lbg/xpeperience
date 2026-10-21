<script setup lang="ts">
import { computed } from 'vue'
import { refItem } from '../lib/store'

const props = defineProps<{ id: number | null | undefined; empty?: string }>()
const item = computed(() => refItem(props.id))
</script>

<template>
  <span v-if="item" class="pill" :class="item.color ?? 'gray'">{{ item.label }}</span>
  <span v-else-if="empty" class="faint">{{ empty }}</span>
</template>
