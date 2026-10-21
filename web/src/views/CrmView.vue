<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import CompanyForm from '../components/CompanyForm.vue'
import ContactForm from '../components/ContactForm.vue'
import EmptyState from '../components/EmptyState.vue'
import Icon from '../components/Icon.vue'
import RefPill from '../components/RefPill.vue'
import { api } from '../lib/api'
import { ACTIVITY_LABEL, fmtDate, personName, todayIso } from '../lib/format'
import { can, refsOf, users } from '../lib/store'
import type { Activity, Company, Contact } from '../lib/types'

const route = useRoute()
const router = useRouter()
type Tab = 'companies' | 'contacts' | 'reminders'
const tab = computed<Tab>(() => (['companies', 'contacts', 'reminders'].includes(route.query.tab as string) ? (route.query.tab as Tab) : 'companies'))
const setTab = (t: Tab) => router.replace({ query: { ...route.query, tab: t } })

const filters = reactive({ q: '', type_id: '', sector_id: '', owner_id: '', archived: false, source: '' })
const companies = ref<Company[]>([])
const contacts = ref<Contact[]>([])
const reminders = ref<Activity[]>([])
const loading = ref(true)
const form = ref<'company' | 'contact' | null>(null)

async function load() {
  loading.value = true
  const base = { q: filters.q, type_id: filters.type_id, sector_id: filters.sector_id, owner_id: filters.owner_id, archived: filters.archived || undefined }
  try {
    if (tab.value === 'companies') companies.value = await api.get<Company[]>('/api/companies', base)
    else if (tab.value === 'contacts') contacts.value = await api.get<Contact[]>('/api/contacts', { ...base, source: filters.source })
    else reminders.value = await api.get<Activity[]>('/api/activities', { reminders: true, limit: 200 })
  } finally {
    loading.value = false
  }
}

let timer: number | undefined
watch(() => ({ ...filters }), () => { clearTimeout(timer); timer = window.setTimeout(load, 200) })
watch(tab, load)
onMounted(() => {
  load()
  if (route.query.new === 'company') form.value = 'company'
  if (route.query.new === 'contact') form.value = 'contact'
})

const overdue = (d: string | null) => !!d && d < todayIso()
const hasFilters = computed(() => !!(filters.q || filters.type_id || filters.sector_id || filters.owner_id || filters.source || filters.archived))
function reset() {
  Object.assign(filters, { q: '', type_id: '', sector_id: '', owner_id: '', archived: false, source: '' })
}
const saved = (kind: 'company' | 'contact', id: number) => {
  form.value = null
  router.push(kind === 'company' ? `/crm/companies/${id}` : `/crm/contacts/${id}`)
}
const sectorName = (id: number | null) => refsOf('sector', true).find((s) => s.id === id)?.label
</script>

<template>
  <div class="page">
    <div class="page-head">
      <div><h1>CRM</h1><p class="sub">Vos sociétés, vos contacts et le suivi de la relation commerciale.</p></div>
      <div v-if="can('crm', 'create')" class="row">
        <button class="btn" type="button" @click="form = 'company'"><Icon name="plus" /> Société</button>
        <button class="btn primary" type="button" @click="form = 'contact'"><Icon name="plus" /> Contact</button>
      </div>
    </div>

    <div class="row between wrap">
      <div class="tabs" role="tablist">
        <button type="button" role="tab" :class="{ on: tab === 'companies' }" @click="setTab('companies')">Sociétés</button>
        <button type="button" role="tab" :class="{ on: tab === 'contacts' }" @click="setTab('contacts')">Contacts</button>
        <button type="button" role="tab" :class="{ on: tab === 'reminders' }" @click="setTab('reminders')">Rappels</button>
      </div>
    </div>

    <div v-if="tab !== 'reminders'" class="filters">
      <div class="search-box search"><Icon name="search" /><input v-model="filters.q" class="input" :placeholder="tab === 'companies' ? 'Rechercher une société, une ville…' : 'Rechercher un nom, un poste, une société…'" aria-label="Recherche" /></div>
      <select v-model="filters.type_id" class="select" aria-label="Type"><option value="">Tous les types</option><option v-for="r in refsOf('company_type')" :key="r.id" :value="r.id">{{ r.label }}</option></select>
      <select v-model="filters.sector_id" class="select" aria-label="Secteur"><option value="">Tous les secteurs</option><option v-for="r in refsOf('sector')" :key="r.id" :value="r.id">{{ r.label }}</option></select>
      <select v-model="filters.owner_id" class="select" aria-label="Responsable commercial"><option value="">Tous les responsables</option><option v-for="u in users" :key="u.id" :value="u.id">{{ u.name }}</option></select>
      <select v-if="tab === 'contacts'" v-model="filters.source" class="select" aria-label="Provenance"><option value="">Toutes provenances</option><option value="manual">Saisie manuelle</option><option value="linkedin">Import LinkedIn</option></select>
      <label class="check small"><input v-model="filters.archived" type="checkbox" /> Archivés</label>
      <button v-if="hasFilters" class="btn ghost small" type="button" @click="reset">Effacer les filtres</button>
    </div>

    <!-- Sociétés -->
    <div v-if="tab === 'companies'" class="card flush">
      <div v-if="loading && !companies.length" class="skel"><div v-for="i in 5" :key="i" class="skeleton" style="height: 24px" /></div>
      <EmptyState v-else-if="!companies.length" icon="building" :title="hasFilters ? 'Aucune société ne correspond' : 'Aucune société pour le moment'">
        {{ hasFilters ? 'Modifiez ou effacez les filtres.' : 'Ajoutez vos clients et prospects pour y rattacher des contacts, des besoins et des projets.' }}
        <template v-if="!hasFilters && can('crm', 'create')" #actions><button class="btn primary" type="button" @click="form = 'company'">Créer une société</button></template>
      </EmptyState>
      <div v-else class="table-wrap">
        <table class="table">
          <thead><tr><th>Société</th><th>Type</th><th>Secteur</th><th>Ville</th><th>Responsable</th><th class="right">Contacts</th><th class="right">Besoins ouverts</th></tr></thead>
          <tbody>
            <tr v-for="c in companies" :key="c.id" class="click" :class="{ 'archived-row': c.archived }" @click="router.push(`/crm/companies/${c.id}`)">
              <td class="cell-link"><RouterLink :to="`/crm/companies/${c.id}`" class="name" @click.stop>{{ c.name }}</RouterLink></td>
              <td><RefPill :id="c.type_id" empty="—" /></td>
              <td>{{ sectorName(c.sector_id) ?? '—' }}</td>
              <td>{{ c.city ?? '—' }}</td>
              <td>{{ c.owner?.name ?? '—' }}</td>
              <td class="right">{{ c.contacts_count }}</td>
              <td class="right">{{ c.open_needs_count }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Contacts -->
    <div v-else-if="tab === 'contacts'" class="card flush">
      <div v-if="loading && !contacts.length" class="skel"><div v-for="i in 5" :key="i" class="skeleton" style="height: 24px" /></div>
      <EmptyState v-else-if="!contacts.length" icon="users" :title="hasFilters ? 'Aucun contact ne correspond' : 'Aucun contact pour le moment'">
        {{ hasFilters ? 'Modifiez ou effacez les filtres.' : 'Ajoutez un contact à la main, ou importez-le depuis un profil LinkedIn avec l’extension Boond by XPEHO.' }}
        <template v-if="!hasFilters && can('crm', 'create')" #actions><button class="btn primary" type="button" @click="form = 'contact'">Créer un contact</button></template>
      </EmptyState>
      <div v-else class="table-wrap">
        <table class="table">
          <thead><tr><th>Contact</th><th>Poste</th><th>Société</th><th>Localisation</th><th>Responsable</th><th>Provenance</th></tr></thead>
          <tbody>
            <tr v-for="k in contacts" :key="k.id" class="click" :class="{ 'archived-row': k.archived }" @click="router.push(`/crm/contacts/${k.id}`)">
              <td class="cell-link"><RouterLink :to="`/crm/contacts/${k.id}`" class="name" @click.stop>{{ personName(k.first_name, k.last_name) }}</RouterLink><div v-if="k.email" class="sub">{{ k.email }}</div></td>
              <td>{{ k.job_title ?? '—' }}</td>
              <td><RouterLink v-if="k.company" :to="`/crm/companies/${k.company.id}`" class="link" @click.stop>{{ k.company.name }}</RouterLink><span v-else class="faint">—</span></td>
              <td>{{ k.location ?? '—' }}</td>
              <td>{{ k.owner?.name ?? '—' }}</td>
              <td><span v-if="k.source === 'linkedin'" class="pill blue" :title="'Importé le ' + fmtDate(k.imported_at)">LinkedIn</span><span v-else class="faint">Saisie</span></td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Rappels -->
    <div v-else class="card">
      <div v-if="loading && !reminders.length" class="skel"><div v-for="i in 3" :key="i" class="skeleton" style="height: 24px" /></div>
      <EmptyState v-else-if="!reminders.length" icon="bell" title="Aucun rappel en attente">Programmez une relance ou une échéance depuis une fiche société ou contact.</EmptyState>
      <div v-else class="list">
        <RouterLink v-for="a in reminders" :key="a.id" :to="a.contact ? `/crm/contacts/${a.contact.id}` : a.company ? `/crm/companies/${a.company.id}` : '/crm'" class="row" style="gap: 14px">
          <span class="pill plain" :class="overdue(a.due_date) ? 'red' : 'orange'" style="min-width: 110px; justify-content: center">{{ fmtDate(a.due_date) }}</span>
          <span class="grow">
            <strong>{{ ACTIVITY_LABEL[a.kind] }}</strong> <span class="muted">· {{ a.contact?.name ?? a.company?.name }}</span>
            <span v-if="a.comment" class="muted small" style="display: block">{{ a.comment }}</span>
          </span>
          <span class="faint small">{{ a.owner?.name }}</span>
        </RouterLink>
      </div>
    </div>

    <CompanyForm v-if="form === 'company'" @close="form = null" @saved="(c) => saved('company', c.id)" />
    <ContactForm v-if="form === 'contact'" @close="form = null" @saved="(c) => saved('contact', c.id)" />
  </div>
</template>

<style scoped>
.skel { padding: 20px; display: flex; flex-direction: column; gap: 14px; }
</style>
