<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import ActivityPanel from '../components/ActivityPanel.vue'
import CompanyForm from '../components/CompanyForm.vue'
import ContactForm from '../components/ContactForm.vue'
import EmptyState from '../components/EmptyState.vue'
import HistoryPanel from '../components/HistoryPanel.vue'
import Icon from '../components/Icon.vue'
import NeedForm from '../components/NeedForm.vue'
import ProjectForm from '../components/ProjectForm.vue'
import RefPill from '../components/RefPill.vue'
import { ApiError, api } from '../lib/api'
import { fmtDate, initials, personName } from '../lib/format'
import { can, confirmAction, notify, refLabel } from '../lib/store'
import type { Company, Contact, Need, Project } from '../lib/types'

const route = useRoute()
const router = useRouter()
const id = Number(route.params.id)
const company = ref<Company | null>(null)
const contacts = ref<Contact[]>([])
const needs = ref<Need[]>([])
const projects = ref<Project[]>([])
const error = ref('')
const editing = ref(false)
const adding = ref<'contact' | 'need' | 'project' | null>(null)
const refresh = ref(0)

async function load() {
  try {
    company.value = await api.get<Company>(`/api/companies/${id}`)
    const [k, n, p] = await Promise.all([
      api.get<Contact[]>('/api/contacts', { company_id: id }),
      can('needs') ? api.get<Need[]>('/api/needs', { company_id: id }) : Promise.resolve([]),
      can('projects') ? api.get<Project[]>('/api/projects', { company_id: id }) : Promise.resolve([]),
    ])
    contacts.value = k
    needs.value = n
    projects.value = p
    refresh.value++
  } catch (e) {
    error.value = e instanceof ApiError ? e.message : 'Chargement impossible.'
  }
}
onMounted(load)

async function archive() {
  const c = company.value!
  if (!c.archived && !(await confirmAction('Archiver cette société ?', 'Elle n’apparaîtra plus dans les listes, mais son historique et ses liens sont conservés. Vous pourrez la restaurer.', 'Archiver'))) return
  company.value = await api.post<Company>(`/api/companies/${id}/archive`, { archived: !c.archived })
  notify(c.archived ? 'Société restaurée.' : 'Société archivée.')
  refresh.value++
}

async function remove() {
  if (!(await confirmAction('Supprimer définitivement cette société ?', 'Cette action est irréversible. Elle n’est possible que si aucun contact, besoin ou projet n’y est rattaché.', 'Supprimer', true))) return
  try {
    await api.del(`/api/companies/${id}`)
    notify('Société supprimée.')
    router.replace('/crm')
  } catch (e) {
    notify(e instanceof ApiError ? e.message : 'Suppression impossible.', 'error')
  }
}

const address = computed(() => [company.value?.address, [company.value?.postal_code, company.value?.city].filter(Boolean).join(' '), company.value?.country].filter(Boolean).join(', '))
const activeNeeds = computed(() => needs.value.filter((n) => !n.archived))
</script>

<template>
  <div class="page">
    <p v-if="error" class="alert error" role="alert">{{ error }} <RouterLink to="/crm" class="link">Retour au CRM</RouterLink></p>
    <template v-if="company">
      <nav class="crumbs"><RouterLink to="/crm">CRM</RouterLink><span>/</span><RouterLink to="/crm?tab=companies">Sociétés</RouterLink><span>/</span><span>{{ company.name }}</span></nav>
      <div class="page-head">
        <div class="row" style="gap: 16px">
          <span class="avatar lg"><Icon name="building" style="width: 26px" /></span>
          <div>
            <h1>{{ company.name }}</h1>
            <div class="row wrap" style="margin-top: 6px"><RefPill :id="company.type_id" /><span v-if="company.sector_id" class="tag">{{ refLabel(company.sector_id) }}</span><span v-if="company.archived" class="pill gray">Archivée</span></div>
          </div>
        </div>
        <div class="row wrap">
          <template v-if="can('crm', 'update')">
            <button class="btn" type="button" @click="editing = true"><Icon name="edit" /> Modifier</button>
            <button class="btn" type="button" @click="archive"><Icon :name="company.archived ? 'restore' : 'archive'" /> {{ company.archived ? 'Restaurer' : 'Archiver' }}</button>
          </template>
          <button v-if="can('crm', 'delete')" class="btn danger" type="button" @click="remove"><Icon name="trash" /> Supprimer</button>
        </div>
      </div>

      <div class="split">
        <div class="stack">
          <section class="card">
            <div class="card-head"><h2>Coordonnées</h2></div>
            <dl class="kv">
              <dt>Responsable commercial</dt><dd>{{ company.owner?.name ?? '—' }}</dd>
              <dt>Site web</dt><dd><a v-if="company.website" :href="company.website.startsWith('http') ? company.website : 'https://' + company.website" target="_blank" rel="noopener noreferrer" class="link">{{ company.website }}</a><template v-else>—</template></dd>
              <dt>Téléphone</dt><dd>{{ company.phone ?? '—' }}</dd>
              <dt>E-mail</dt><dd><a v-if="company.email" :href="`mailto:${company.email}`" class="link">{{ company.email }}</a><template v-else>—</template></dd>
              <dt>Adresse</dt><dd>{{ address || '—' }}</dd>
              <template v-if="company.notes"><dt>Notes</dt><dd style="white-space: pre-wrap">{{ company.notes }}</dd></template>
            </dl>
          </section>

          <section class="card flush">
            <div class="card-head"><h2>Contacts ({{ contacts.length }})</h2><button v-if="can('crm', 'create')" class="btn small" type="button" @click="adding = 'contact'"><Icon name="plus" /> Contact</button></div>
            <EmptyState v-if="!contacts.length" compact title="Aucun contact">Ajoutez les interlocuteurs de cette société.</EmptyState>
            <div v-else class="table-wrap">
              <table class="table">
                <tbody>
                  <tr v-for="k in contacts" :key="k.id" class="click" @click="router.push(`/crm/contacts/${k.id}`)">
                    <td style="width: 52px"><span class="avatar">{{ initials(personName(k.first_name, k.last_name)) }}</span></td>
                    <td><RouterLink :to="`/crm/contacts/${k.id}`" class="name" @click.stop>{{ personName(k.first_name, k.last_name) }}</RouterLink><div class="sub">{{ k.job_title ?? '' }}</div></td>
                    <td class="muted">{{ k.email ?? k.phone ?? '' }}</td>
                    <td><span v-if="k.source === 'linkedin'" class="pill blue">LinkedIn</span></td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          <section v-if="can('needs')" class="card flush">
            <div class="card-head"><h2>Besoins ({{ activeNeeds.length }})</h2><button v-if="can('needs', 'create') && contacts.length" class="btn small" type="button" @click="adding = 'need'"><Icon name="plus" /> Besoin</button></div>
            <EmptyState v-if="!activeNeeds.length" compact title="Aucun besoin">{{ contacts.length ? 'Notez les demandes de ce client.' : 'Ajoutez d’abord un contact : un besoin est toujours rattaché à un contact.' }}</EmptyState>
            <div v-else class="table-wrap">
              <table class="table"><tbody>
                <tr v-for="n in activeNeeds" :key="n.id" class="click" @click="router.push(`/needs/${n.id}`)">
                  <td><RouterLink :to="`/needs/${n.id}`" class="name" @click.stop>{{ n.title }}</RouterLink><div class="sub">{{ n.contact?.name }}</div></td>
                  <td><RefPill :id="n.state_id" /></td>
                  <td class="muted">{{ n.start_date ? 'dès le ' + fmtDate(n.start_date) : '' }}</td>
                  <td class="right muted">{{ n.positionings_count }} positionnement(s)</td>
                </tr>
              </tbody></table>
            </div>
          </section>

          <section v-if="can('projects')" class="card flush">
            <div class="card-head"><h2>Projets ({{ projects.length }})</h2><button v-if="can('projects', 'create')" class="btn small" type="button" @click="adding = 'project'"><Icon name="plus" /> Projet</button></div>
            <EmptyState v-if="!projects.length" compact title="Aucun projet">Les projets apparaissent ici une fois un besoin gagné.</EmptyState>
            <div v-else class="table-wrap">
              <table class="table"><tbody>
                <tr v-for="p in projects" :key="p.id" class="click" @click="router.push(`/projects/${p.id}`)">
                  <td><RouterLink :to="`/projects/${p.id}`" class="name" @click.stop>{{ p.title }}</RouterLink><div class="sub">{{ p.resources.map((r) => r.name).join(', ') }}</div></td>
                  <td><RefPill :id="p.type_id" /></td>
                  <td><RefPill :id="p.state_id" /></td>
                  <td class="muted nowrap">{{ fmtDate(p.start_date) }} → {{ p.end_date ? fmtDate(p.end_date) : 'en cours' }}</td>
                </tr>
              </tbody></table>
            </div>
          </section>
        </div>

        <div class="stack">
          <ActivityPanel :filter="{ company_id: id }" :links="{ company_id: id }" area="crm" show-context :key="refresh" />
          <HistoryPanel entity="company" :id="id" :refresh-key="refresh" />
        </div>
      </div>

      <CompanyForm v-if="editing" :company="company" @close="editing = false" @saved="editing = false; load()" />
      <ContactForm v-if="adding === 'contact'" :company-id="id" @close="adding = null" @saved="(k) => { adding = null; router.push(`/crm/contacts/${k.id}`) }" />
      <NeedForm v-if="adding === 'need'" :contact-id="contacts[0]?.id" @close="adding = null" @saved="adding = null; load()" />
      <ProjectForm v-if="adding === 'project'" :company-id="id" @close="adding = null" @saved="(p) => { adding = null; router.push(`/projects/${p.id}`) }" />
    </template>
  </div>
</template>
