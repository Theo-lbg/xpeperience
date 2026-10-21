<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import EmptyState from '../components/EmptyState.vue'
import Icon from '../components/Icon.vue'
import ProjectForm from '../components/ProjectForm.vue'
import RefPill from '../components/RefPill.vue'
import { api } from '../lib/api'
import { fmtDate } from '../lib/format'
import { can, refsOf, users } from '../lib/store'
import type { Company, Project, Resource } from '../lib/types'

const route = useRoute()
const router = useRouter()
const filters = reactive({ q: '', company_id: '', resource_id: '', manager_id: '', state_id: '', type_id: '', period_from: '', period_to: '', archived: false })
const projects = ref<Project[]>([])
const companies = ref<Company[]>([])
const resources = ref<Resource[]>([])
const loading = ref(true)
const creating = ref(false)

async function load() {
  loading.value = true
  try {
    projects.value = await api.get<Project[]>('/api/projects', { ...filters, archived: filters.archived || undefined })
  } finally {
    loading.value = false
  }
}
let timer: number | undefined
watch(() => ({ ...filters }), () => { clearTimeout(timer); timer = window.setTimeout(load, 200) })
onMounted(async () => {
  load()
  if (can('crm')) companies.value = await api.get<Company[]>('/api/companies')
  if (can('resources')) resources.value = await api.get<Resource[]>('/api/resources')
  if (route.query.new) creating.value = true
})
const hasFilters = computed(() => Object.entries(filters).some(([k, v]) => k !== 'archived' ? !!v : v))
function reset() {
  Object.assign(filters, { q: '', company_id: '', resource_id: '', manager_id: '', state_id: '', type_id: '', period_from: '', period_to: '', archived: false })
}
</script>

<template>
  <div class="page">
    <div class="page-head">
      <div><h1>Projets et prestations</h1><p class="sub">Les projets signés, leurs prestations et les ressources affectées.</p></div>
      <button v-if="can('projects', 'create')" class="btn primary" type="button" @click="creating = true"><Icon name="plus" /> Nouveau projet</button>
    </div>

    <div class="filters">
      <div class="search-box search"><Icon name="search" /><input v-model="filters.q" class="input" placeholder="Rechercher un projet, un client, une référence…" aria-label="Recherche" /></div>
      <select v-model="filters.company_id" class="select" aria-label="Client"><option value="">Tous les clients</option><option v-for="c in companies" :key="c.id" :value="c.id">{{ c.name }}</option></select>
      <select v-model="filters.resource_id" class="select" aria-label="Ressource"><option value="">Toutes les ressources</option><option v-for="r in resources" :key="r.id" :value="r.id">{{ r.first_name }} {{ r.last_name }}</option></select>
      <select v-model="filters.manager_id" class="select" aria-label="Responsable"><option value="">Tous les responsables</option><option v-for="u in users" :key="u.id" :value="u.id">{{ u.name }}</option></select>
      <select v-model="filters.state_id" class="select" aria-label="État"><option value="">Tous les états</option><option v-for="s in refsOf('project_state')" :key="s.id" :value="s.id">{{ s.label }}</option></select>
      <select v-model="filters.type_id" class="select" aria-label="Type"><option value="">Tous les types</option><option v-for="s in refsOf('project_type')" :key="s.id" :value="s.id">{{ s.label }}</option></select>
      <label class="row small muted" style="gap: 6px">Période <input v-model="filters.period_from" class="input" type="date" style="width: 150px" aria-label="Période : du" /> → <input v-model="filters.period_to" class="input" type="date" style="width: 150px" aria-label="Période : au" /></label>
      <label class="check small"><input v-model="filters.archived" type="checkbox" /> Archivés</label>
      <button v-if="hasFilters" class="btn ghost small" type="button" @click="reset">Effacer les filtres</button>
    </div>

    <div v-if="loading && !projects.length" class="card"><div class="stack tight"><div v-for="i in 4" :key="i" class="skeleton" style="height: 30px" /></div></div>
    <EmptyState v-else-if="!projects.length" icon="folder" :title="hasFilters ? 'Aucun projet ne correspond' : 'Aucun projet pour le moment'">
      {{ hasFilters ? 'Modifiez ou effacez les filtres.' : 'Un projet naît d’un besoin gagné : positionnez une ressource, passez le positionnement à « gagné », et le projet se crée sans ressaisie.' }}
      <template v-if="!hasFilters" #actions>
        <RouterLink v-if="can('needs')" class="btn" to="/needs">Voir les besoins</RouterLink>
        <button v-if="can('projects', 'create')" class="btn primary" type="button" @click="creating = true">Créer un projet</button>
      </template>
    </EmptyState>
    <div v-else class="card flush">
      <div class="table-wrap">
        <table class="table">
          <thead><tr><th>Projet</th><th>Client</th><th>Type</th><th>État</th><th>Période</th><th>Ressources</th><th>Responsable</th></tr></thead>
          <tbody>
            <tr v-for="p in projects" :key="p.id" class="click" :class="{ 'archived-row': p.archived }" @click="router.push(`/projects/${p.id}`)">
              <td class="cell-link"><RouterLink :to="`/projects/${p.id}`" class="name" @click.stop>{{ p.title }}</RouterLink><div v-if="p.reference" class="sub">Réf. {{ p.reference }}</div></td>
              <td><RouterLink v-if="p.company" :to="`/crm/companies/${p.company.id}`" class="link" @click.stop>{{ p.company.name }}</RouterLink><span v-else class="faint">Interne</span></td>
              <td><RefPill :id="p.type_id" empty="—" /></td>
              <td><RefPill :id="p.state_id" empty="—" /></td>
              <td class="nowrap">{{ fmtDate(p.start_date) }} → {{ p.end_date ? fmtDate(p.end_date) : 'en cours' }}<div v-if="p.forecast_end_date && p.forecast_end_date !== p.end_date" class="sub">prévu jusqu'au {{ fmtDate(p.forecast_end_date) }}</div></td>
              <td>{{ p.resources.map((r) => r.name).join(', ') || '—' }}<div v-if="p.deliveries_count" class="sub">{{ p.deliveries_count }} prestation(s)</div></td>
              <td>{{ p.manager?.name ?? '—' }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <ProjectForm v-if="creating" @close="creating = false" @saved="(p) => { creating = false; router.push(`/projects/${p.id}`) }" />
  </div>
</template>
