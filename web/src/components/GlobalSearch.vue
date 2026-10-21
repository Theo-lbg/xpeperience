<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { api } from '../lib/api'
import { entityLabel, entityRoute } from '../lib/format'
import type { SearchHit } from '../lib/types'
import Icon from './Icon.vue'

const router = useRouter()
const q = ref('')
const hits = ref<SearchHit[]>([])
const open = ref(false)
const active = ref(0)
const input = ref<HTMLInputElement>()
const root = ref<HTMLElement>()
let timer: number | undefined
let seq = 0

watch(q, (value) => {
  clearTimeout(timer)
  if (value.trim().length < 2) {
    hits.value = []
    return
  }
  timer = window.setTimeout(async () => {
    const mine = ++seq
    try {
      const res = await api.get<SearchHit[]>('/api/search', { q: value })
      if (mine === seq) {
        hits.value = res
        active.value = 0
      }
    } catch {
      hits.value = []
    }
  }, 180)
})

function go(hit: SearchHit) {
  open.value = false
  q.value = ''
  router.push(entityRoute(hit.kind, hit.id))
}

function onKey(e: KeyboardEvent) {
  if (e.key === 'ArrowDown') { e.preventDefault(); active.value = Math.min(active.value + 1, hits.value.length - 1) }
  else if (e.key === 'ArrowUp') { e.preventDefault(); active.value = Math.max(active.value - 1, 0) }
  else if (e.key === 'Enter' && hits.value[active.value]) go(hits.value[active.value]!)
  else if (e.key === 'Escape') { open.value = false; input.value?.blur() }
}

function shortcut(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
    e.preventDefault()
    input.value?.focus()
  }
}
function outside(e: MouseEvent) {
  if (root.value && !root.value.contains(e.target as Node)) open.value = false
}
onMounted(() => { document.addEventListener('keydown', shortcut); document.addEventListener('mousedown', outside) })
onBeforeUnmount(() => { document.removeEventListener('keydown', shortcut); document.removeEventListener('mousedown', outside) })
</script>

<template>
  <div ref="root" class="gs">
    <div class="search-box">
      <Icon name="search" />
      <input ref="input" v-model="q" class="input" type="search" placeholder="Rechercher une société, un contact, un besoin…  (⌘K)" aria-label="Recherche globale" @focus="open = true" @keydown="onKey" />
    </div>
    <div v-if="open && q.trim().length >= 2" class="menu panel">
      <div v-if="!hits.length" class="faint" style="padding: 14px">Aucun résultat pour « {{ q }} ».</div>
      <button v-for="(h, i) in hits" :key="h.kind + h.id" type="button" :class="{ active: i === active }" @mouseenter="active = i" @click="go(h)">
        <span class="pill plain gray kind">{{ entityLabel(h.kind) }}</span>
        <span class="grow"><strong>{{ h.title }}</strong><small v-if="h.subtitle" class="faint"> · {{ h.subtitle }}</small></span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.gs { position: relative; width: min(560px, 100%); }
.panel { top: calc(100% + 8px); left: 0; right: 0; max-height: 420px; overflow-y: auto; }
.kind { flex: none; }
.menu button strong { font-weight: 650; }
</style>
