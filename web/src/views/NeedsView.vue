<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import EmptyState from '../components/EmptyState.vue'
import Icon from '../components/Icon.vue'
import NeedForm from '../components/NeedForm.vue'
import RefPill from '../components/RefPill.vue'
import { ApiError, api } from '../lib/api'
import { fmtDate } from '../lib/format'
import { boot, can, notify, refCategory, refsOf, users } from '../lib/store'
import type { Company, Need, Preferences } from '../lib/types'

const route = useRoute()
const router = useRouter()
const view = ref<'list' | 'kanban'>(boot.value?.me.preferences.needs_view ?? 'list')
const filters = reactive({ q: '', company_id: '', owner_id: '', state_id: '', status: 'open', type_id: '', start_from: '', start_to: '' })
const needs = ref<Need[]>([])
const companies = ref<Company[]>([])
const loading = ref(true)
const creating = ref(false)
const dragging = ref<number | null>(null)
const over = ref<number | null>(null)

async function load() {
  loading.value = true
  try {
    needs.value = await api.get<Need[]>('/api/needs', {
      q: filters.q, company_id: filters.company_id, owner_id: filters.owner_id, state_id: filters.state_id, type_id: filters.type_id,
      start_from: filters.start_from, start_to: filters.start_to, archived: filters.status === 'archived' || undefined,
    })
  } finally {
    loading.value = false
  }
}
let timer: number | undefined
watch(() => ({ ...filters }), () => { clearTimeout(timer); timer = window.setTimeout(load, 200) })
onMounted(async () => {
  load()
  if (can('crm')) companies.value = await api.get<Company[]>('/api/companies')
  if (route.query.new) creating.value = true
})

async function setView(v: 'list' | 'kanban') {
  view.value = v
  const prefs = { ...boot.value!.me.preferences, needs_view: v } as Preferences
  boot.value!.me.preferences = await api.put<Preferences>('/api/me/preferences', prefs)
}

// « Ouverts / Gagnés / Perdus » par catégorie d'état.
const byStatus = computed(() => needs.value.filter((n) => {
  if (filters.status === 'archived' || filters.status === 'all') return true
  const cat = refCategory(n.state_id)
  return filters.status === 'open' ? cat === 'open' || cat === 'standby' : cat === filters.status
}))
const counts = computed(() => {
  const c = { open: 0, won: 0, lost: 0 }
  for (const n of needs.value) {
    const k = refCategory(n.state_id)
    if (k === 'won') c.won++
    else if (k === 'lost') c.lost++
    else c.open++
  }
  return c
})
const columns = computed(() => refsOf('need_state').map((s) => ({ state: s, items: needs.value.filter((n) => n.state_id === s.id) })))
const hasFilters = computed(() => !!(filters.q || filters.company_id || filters.owner_id || filters.state_id || filters.type_id || filters.start_from || filters.start_to))
function reset() {
  Object.assign(filters, { q: '', company_id: '', owner_id: '', state_id: '', type_id: '', start_from: '', start_to: '' })
}

async function drop(stateId: number) {
  const id = dragging.value
  dragging.value = over.value = null
  const n = needs.value.find((x) => x.id === id)
  if (!n || n.state_id === stateId || !can('needs', 'update')) return
  const previous = n.state_id
  n.state_id = stateId
  try {
    Object.assign(n, await api.put<Need>(`/api/needs/${n.id}/state`, { state_id: stateId }))
  } catch (e) {
    n.state_id = previous
    notify(e instanceof ApiError ? e.message : 'Changement d’état impossible.', 'error')
  }
}
</script>

<template>
  <div class="page">
    <div class="page-head">
      <div><h1>Besoins</h1><p class="sub">Les demandes de vos clients, de la première discussion à la signature.</p></div>
      <button v-if="can('needs', 'create')" class="btn primary" type="button" @click="creating = true"><Icon name="plus" /> Nouveau besoin</button>
    </div>

    <div class="row between wrap">
      <div class="tabs" role="tablist" aria-label="Statut commercial">
        <button type="button" :class="{ on: filters.status === 'open' }" @click="filters.status = 'open'">Ouverts<span class="count">{{ counts.open }}</span></button>
        <button type="button" :class="{ on: filters.status === 'won' }" @click="filters.status = 'won'">Gagnés<span class="count">{{ counts.won }}</span></button>
        <button type="button" :class="{ on: filters.status === 'lost' }" @click="filters.status = 'lost'">Perdus<span class="count">{{ counts.lost }}</span></button>
        <button type="button" :class="{ on: filters.status === 'all' }" @click="filters.status = 'all'">Tous</button>
        <button type="button" :class="{ on: filters.status === 'archived' }" @click="filters.status = 'archived'">Archivés</button>
      </div>
      <div class="tabs" role="group" aria-label="Affichage">
        <button type="button" :class="{ on: view === 'list' }" aria-label="Vue liste" @click="setView('list')"><Icon name="list" style="width: 16px; vertical-align: -3px" /> Liste</button>
        <button type="button" :class="{ on: view === 'kanban' }" aria-label="Vue Kanban" @click="setView('kanban')"><Icon name="columns" style="width: 16px; vertical-align: -3px" /> Kanban</button>
      </div>
    </div>

    <div class="filters">
      <div class="search-box search"><Icon name="search" /><input v-model="filters.q" class="input" placeholder="Rechercher un besoin, une compétence, un client…" aria-label="Recherche" /></div>
      <select v-model="filters.company_id" class="select" aria-label="Société"><option value="">Toutes les sociétés</option><option v-for="c in companies" :key="c.id" :value="c.id">{{ c.name }}</option></select>
      <select v-model="filters.owner_id" class="select" aria-label="Responsable"><option value="">Tous les responsables</option><option v-for="u in users" :key="u.id" :value="u.id">{{ u.name }}</option></select>
      <select v-model="filters.state_id" class="select" aria-label="État"><option value="">Tous les états</option><option v-for="s in refsOf('need_state')" :key="s.id" :value="s.id">{{ s.label }}</option></select>
      <select v-model="filters.type_id" class="select" aria-label="Type"><option value="">Tous les types</option><option v-for="s in refsOf('need_type')" :key="s.id" :value="s.id">{{ s.label }}</option></select>
      <label class="row small muted" style="gap: 6px">Démarrage <input v-model="filters.start_from" class="input" type="date" style="width: 150px" aria-label="Démarrage à partir du" /> → <input v-model="filters.start_to" class="input" type="date" style="width: 150px" aria-label="Démarrage jusqu’au" /></label>
      <button v-if="hasFilters" class="btn ghost small" type="button" @click="reset">Effacer les filtres</button>
    </div>

    <div v-if="loading && !needs.length" class="card"><div class="stack tight"><div v-for="i in 4" :key="i" class="skeleton" style="height: 26px" /></div></div>

    <EmptyState v-else-if="view === 'list' && !byStatus.length" icon="target" :title="hasFilters ? 'Aucun besoin ne correspond' : 'Aucun besoin dans cette vue'">
      {{ hasFilters ? 'Modifiez ou effacez les filtres.' : 'Un besoin décrit la demande d’un client. Il est toujours rattaché à un contact du CRM.' }}
      <template v-if="can('needs', 'create') && !hasFilters" #actions><button class="btn primary" type="button" @click="creating = true">Créer un besoin</button></template>
    </EmptyState>

    <div v-else-if="view === 'list'" class="card flush">
      <div class="table-wrap">
        <table class="table">
          <thead><tr><th>Besoin</th><th>Société</th><th>Contact</th><th>Type</th><th>État</th><th>Démarrage</th><th>Responsable</th><th class="right">Positionnements</th></tr></thead>
          <tbody>
            <tr v-for="n in byStatus" :key="n.id" class="click" :class="{ 'archived-row': n.archived }" @click="router.push(`/needs/${n.id}`)">
              <td class="cell-link"><RouterLink :to="`/needs/${n.id}`" class="name" @click.stop>{{ n.title }}</RouterLink><div v-if="n.skills.length" class="sub">{{ n.skills.slice(0, 4).join(' · ') }}</div></td>
              <td><RouterLink v-if="n.company" :to="`/crm/companies/${n.company.id}`" class="link" @click.stop>{{ n.company.name }}</RouterLink><span v-else class="faint">—</span></td>
              <td>{{ n.contact?.name ?? '—' }}</td>
              <td><RefPill :id="n.type_id" empty="—" /></td>
              <td><RefPill :id="n.state_id" /></td>
              <td>{{ fmtDate(n.start_date) }}</td>
              <td>{{ n.owner?.name ?? '—' }}</td>
              <td class="right">{{ n.positionings_count }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div v-else class="kanban" aria-label="Besoins par état">
      <div v-for="col in columns" :key="col.state.id" class="kcol" :class="{ over: over === col.state.id }" @dragover.prevent="over = col.state.id" @dragleave="over = null" @drop.prevent="drop(col.state.id)">
        <div class="kcol-head"><RefPill :id="col.state.id" /><span class="faint small">{{ col.items.length }}</span></div>
        <article v-for="n in col.items" :key="n.id" class="kcard" :draggable="can('needs', 'update')" @dragstart="dragging = n.id" @dragend="dragging = over = null" @click="router.push(`/needs/${n.id}`)">
          <div class="title">{{ n.title }}</div>
          <div class="muted small">{{ n.company?.name ?? '—' }}<template v-if="n.contact"> · {{ n.contact.name }}</template></div>
          <div class="row between small">
            <RefPill :id="n.type_id" />
            <span class="faint">{{ n.start_date ? fmtDate(n.start_date) : '' }}</span>
          </div>
          <div v-if="n.positionings_count" class="small faint"><Icon name="link" style="width: 13px; vertical-align: -2px" /> {{ n.positionings_count }} positionnement(s)</div>
        </article>
        <p v-if="!col.items.length" class="faint small" style="padding: 8px 6px">Aucun besoin</p>
      </div>
    </div>

    <NeedForm v-if="creating" @close="creating = false" @saved="(n) => { creating = false; router.push(`/needs/${n.id}`) }" />
  </div>
</template>
