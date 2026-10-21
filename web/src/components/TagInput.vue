<script setup lang="ts">
import { ref } from 'vue'

const model = defineModel<string[]>({ required: true })
defineProps<{ placeholder?: string }>()
const draft = ref('')

function commit() {
  const value = draft.value.trim().replace(/,$/, '').trim()
  if (value && !model.value.some((t) => t.toLowerCase() === value.toLowerCase())) model.value = [...model.value, value]
  draft.value = ''
}

function onKey(e: KeyboardEvent) {
  if (e.key === 'Enter' || e.key === ',') {
    e.preventDefault()
    commit()
  } else if (e.key === 'Backspace' && !draft.value && model.value.length) model.value = model.value.slice(0, -1)
}
</script>

<template>
  <div class="tagbox input" @click="($refs.inp as HTMLInputElement).focus()">
    <span v-for="t in model" :key="t" class="tag">
      {{ t }}
      <button type="button" :aria-label="`Retirer ${t}`" @click.stop="model = model.filter((x) => x !== t)">×</button>
    </span>
    <input ref="inp" v-model="draft" :placeholder="model.length ? '' : placeholder" @keydown="onKey" @blur="commit" />
  </div>
</template>

<style scoped>
.tagbox { height: auto; min-height: 38px; display: flex; flex-wrap: wrap; gap: 6px; align-items: center; padding: 5px 8px; cursor: text; }
.tagbox:focus-within { border-color: var(--green-strong); box-shadow: 0 0 0 3px rgba(179, 210, 107, 0.35); }
.tagbox input { border: 0; outline: 0; flex: 1; min-width: 100px; height: 26px; font: inherit; background: none; }
.tag { background: var(--green-soft); color: var(--green-ink); gap: 4px; }
.tag button { border: 0; background: none; padding: 0 0 0 2px; font-size: 15px; line-height: 1; color: inherit; opacity: 0.7; }
.tag button:hover { opacity: 1; }
</style>
