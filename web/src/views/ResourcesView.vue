<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import EmptyState from '../components/EmptyState.vue'
import Icon from '../components/Icon.vue'
import RefPill from '../components/RefPill.vue'
import ResourceForm from '../components/ResourceForm.vue'
import { api } from '../lib/api'
import { RESOURCE_KIND_LABEL, fmtDate, fmtMoney, initials } from '../lib/format'
import { can, refsOf } from '../lib/store'
import type { Resource, ResourceKind } from '../lib/types'

const route = useRoute()
const router = useRouter()
const kind = ref<'all' | ResourceKind>('all')
const filters = reactive({ q: '', skill: '', location: '', availability: '', contract_type_id: '', archived: false })
const resources = ref<Resource[]>([])
const loading = ref(true)
const creating = ref<ResourceKind | null>(null)
const showCost = can('costs', 'read')

async function load() {
  loading.value = true
  try {
    resources.value = await api.get<Resource[]>('/api/resources', { ...filters, kind: kind.value === 'all' ? undefined : kind.value, archived: filters.archived || undefined })
  } finally {
    loading.value = false
  }
}
let timer: number | undefined
watch(() => [{ ...filters }, kind.value], () => { clearTimeout(timer); timer = window.setTimeout(load, 200) }, { deep: true })
onMounted(() => {
  load()
  if (route.query.new) creating.value = 'candidate'
})

const hasFilters = computed(() => !!(filters.q || filters.skill || filters.location || filters.availability || filters.contract_type_id || filters.archived))
function reset() {
  Object.assign(filters, { q: '', skill: '', location: '', availability: '', contract_type_id: '', archived: false })
}
function availability(r: Resource): { text: string; cls: string } {
  const today = new Date().toISOString().slice(0, 10)
  if (r.on_mission) return { text: r.on_mission_until ? `En mission jusqu’au ${fmtDate(r.on_mission_until)}` : 'En mission', cls: 'blue' }
  if (r.availability_date && r.availability_date > today) return { text: `Dispo le ${fmtDate(r.availability_date)}`, cls: 'orange' }
  return { text: 'Disponible', cls: 'green' }
}
</script>

<template>
  <div class="page">
    <div class="page-head">
      <div><h1>Ressources</h1><p class="sub">Vos candidats et vos consultants, avec leurs compétences et leurs disponibilités.</p></div>
      <div v-if="can('resources', 'create')" class="row">
        <button class="btn" type="button" @click="creating = 'candidate'"><Icon name="plus" /> Candidat</button>
        <button class="btn primary" type="button" @click="creating = 'consultant'"><Icon name="plus" /> Consultant</button>
      </div>
    </div>

    <div class="tabs" role="tablist" style="align-self: flex-start">
      <button type="button" :class="{ on: kind === 'all' }" @click="kind = 'all'">Toutes</button>
      <button type="button" :class="{ on: kind === 'consultant' }" @click="kind = 'consultant'">Consultants</button>
      <button type="button" :class="{ on: kind === 'candidate' }" @click="kind = 'candidate'">Candidats</button>
    </div>

    <div class="filters">
      <div class="search-box search"><Icon name="search" /><input v-model="filters.q" class="input" placeholder="Rechercher par nom, métier, compétence…" aria-label="Recherche" /></div>
      <input v-model="filters.skill" class="input" style="width: 170px; border-radius: 999px" placeholder="Compétence" aria-label="Compétence" />
      <input v-model="filters.location" class="input" style="width: 170px; border-radius: 999px" placeholder="Localisation" aria-label="Localisation" />
      <select v-model="filters.availability" class="select" aria-label="Disponibilité">
        <option value="">Toute disponibilité</option><option value="now">Disponible maintenant</option><option value="soon">Disponible sous 30 jours</option><option value="mission">En mission</option>
      </select>
      <select v-model="filters.contract_type_id" class="select" aria-label="Type de contrat"><option value="">Tous les contrats</option><option v-for="c in refsOf('contract_type')" :key="c.id" :value="c.id">{{ c.label }}</option></select>
      <label class="check small"><input v-model="filters.archived" type="checkbox" /> Archivés</label>
      <button v-if="hasFilters" class="btn ghost small" type="button" @click="reset">Effacer les filtres</button>
    </div>

    <div v-if="loading && !resources.length" class="card"><div class="stack tight"><div v-for="i in 4" :key="i" class="skeleton" style="height: 30px" /></div></div>
    <EmptyState v-else-if="!resources.length" icon="briefcase" :title="hasFilters ? 'Aucune ressource ne correspond' : 'Aucune ressource pour le moment'">
      {{ hasFilters ? 'Modifiez ou effacez les filtres.' : 'Ajoutez vos consultants et vos candidats pour pouvoir les positionner sur des besoins.' }}
      <template v-if="!hasFilters && can('resources', 'create')" #actions><button class="btn primary" type="button" @click="creating = 'consultant'">Ajouter une ressource</button></template>
    </EmptyState>
    <div v-else class="card flush">
      <div class="table-wrap">
        <table class="table">
          <thead><tr><th>Ressource</th><th>Statut</th><th>Compétences</th><th>Localisation</th><th>Disponibilité</th><th>Contrat</th><th v-if="showCost" class="right">CJM</th></tr></thead>
          <tbody>
            <tr v-for="r in resources" :key="r.id" class="click" :class="{ 'archived-row': r.archived }" @click="router.push(`/resources/${r.id}`)">
              <td>
                <div class="row" style="gap: 12px">
                  <span class="avatar">{{ initials(`${r.first_name} ${r.last_name}`) }}</span>
                  <div><RouterLink :to="`/resources/${r.id}`" class="name" @click.stop>{{ r.first_name }} {{ r.last_name }}</RouterLink><div class="sub">{{ r.title ?? '' }}</div></div>
                </div>
              </td>
              <td><span class="pill" :class="r.kind === 'consultant' ? 'green' : 'purple'">{{ RESOURCE_KIND_LABEL[r.kind] }}</span></td>
              <td><div class="tags"><span v-for="s in r.skills.slice(0, 3)" :key="s" class="tag">{{ s }}</span><span v-if="r.skills.length > 3" class="faint small">+{{ r.skills.length - 3 }}</span></div></td>
              <td>{{ r.location ?? '—' }}</td>
              <td><span class="pill" :class="availability(r).cls">{{ availability(r).text }}</span></td>
              <td><RefPill :id="r.contract_type_id" empty="—" /></td>
              <td v-if="showCost" class="right">{{ fmtMoney(r.effective_daily_cost) }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <ResourceForm v-if="creating" :kind="creating" @close="creating = null" @saved="(r) => { creating = null; router.push(`/resources/${r.id}`) }" />
  </div>
</template>
