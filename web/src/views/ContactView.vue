<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import ActivityPanel from '../components/ActivityPanel.vue'
import ContactForm from '../components/ContactForm.vue'
import EmptyState from '../components/EmptyState.vue'
import HistoryPanel from '../components/HistoryPanel.vue'
import Icon from '../components/Icon.vue'
import NeedForm from '../components/NeedForm.vue'
import RefPill from '../components/RefPill.vue'
import { ApiError, api } from '../lib/api'
import { fmtDate, initials, personName } from '../lib/format'
import { can, confirmAction, notify } from '../lib/store'
import type { Contact, Need, Project } from '../lib/types'

const route = useRoute()
const router = useRouter()
const id = Number(route.params.id)
const contact = ref<Contact | null>(null)
const needs = ref<Need[]>([])
const projects = ref<Project[]>([])
const error = ref('')
const editing = ref(false)
const addingNeed = ref(false)
const refresh = ref(0)

async function load() {
  try {
    contact.value = await api.get<Contact>(`/api/contacts/${id}`)
    needs.value = can('needs') ? await api.get<Need[]>('/api/needs', { contact_id: id }) : []
    projects.value = can('projects') && contact.value.company ? (await api.get<Project[]>('/api/projects', { company_id: contact.value.company.id })).filter((p) => p.contact?.id === id) : []
    refresh.value++
  } catch (e) {
    error.value = e instanceof ApiError ? e.message : 'Chargement impossible.'
  }
}
onMounted(load)

async function archive() {
  const k = contact.value!
  if (!k.archived && !(await confirmAction('Archiver ce contact ?', 'Il n’apparaîtra plus dans les listes, mais son historique et ses besoins sont conservés.', 'Archiver'))) return
  contact.value = await api.post<Contact>(`/api/contacts/${id}/archive`, { archived: !k.archived })
  notify(k.archived ? 'Contact restauré.' : 'Contact archivé.')
  refresh.value++
}

async function remove() {
  if (!(await confirmAction('Supprimer définitivement ce contact ?', 'Cette action est irréversible. Elle n’est possible que si aucun besoin ou projet n’y est rattaché.', 'Supprimer', true))) return
  try {
    await api.del(`/api/contacts/${id}`)
    notify('Contact supprimé.')
    router.replace('/crm?tab=contacts')
  } catch (e) {
    notify(e instanceof ApiError ? e.message : 'Suppression impossible.', 'error')
  }
}
</script>

<template>
  <div class="page">
    <p v-if="error" class="alert error" role="alert">{{ error }} <RouterLink to="/crm" class="link">Retour au CRM</RouterLink></p>
    <template v-if="contact">
      <nav class="crumbs">
        <RouterLink to="/crm">CRM</RouterLink><span>/</span>
        <template v-if="contact.company"><RouterLink :to="`/crm/companies/${contact.company.id}`">{{ contact.company.name }}</RouterLink><span>/</span></template>
        <span>{{ personName(contact.first_name, contact.last_name) }}</span>
      </nav>
      <div class="page-head">
        <div class="row" style="gap: 16px">
          <span class="avatar lg">{{ initials(personName(contact.first_name, contact.last_name)) }}</span>
          <div>
            <h1>{{ personName(contact.first_name, contact.last_name) }}</h1>
            <div class="muted" style="margin-top: 4px">{{ contact.job_title ?? '' }}<template v-if="contact.job_title && contact.company"> · </template><RouterLink v-if="contact.company" :to="`/crm/companies/${contact.company.id}`" class="link">{{ contact.company.name }}</RouterLink></div>
            <div class="row wrap" style="margin-top: 8px">
              <span v-if="contact.source === 'linkedin'" class="pill blue">Importé depuis LinkedIn le {{ fmtDate(contact.imported_at) }}</span>
              <span v-if="contact.archived" class="pill gray">Archivé</span>
            </div>
          </div>
        </div>
        <div class="row wrap">
          <template v-if="can('crm', 'update')">
            <button class="btn" type="button" @click="editing = true"><Icon name="edit" /> Modifier</button>
            <button class="btn" type="button" @click="archive"><Icon :name="contact.archived ? 'restore' : 'archive'" /> {{ contact.archived ? 'Restaurer' : 'Archiver' }}</button>
          </template>
          <button v-if="can('crm', 'delete')" class="btn danger" type="button" @click="remove"><Icon name="trash" /> Supprimer</button>
        </div>
      </div>

      <div class="split">
        <div class="stack">
          <section class="card">
            <div class="card-head"><h2>Coordonnées</h2></div>
            <dl class="kv">
              <dt>E-mail</dt><dd><a v-if="contact.email" :href="`mailto:${contact.email}`" class="link">{{ contact.email }}</a><template v-else>—</template></dd>
              <dt>Téléphone</dt><dd>{{ contact.phone ?? '—' }}</dd>
              <dt>Mobile</dt><dd>{{ contact.mobile ?? '—' }}</dd>
              <dt>Localisation</dt><dd>{{ contact.location ?? '—' }}</dd>
              <dt>LinkedIn</dt><dd><a v-if="contact.linkedin_url" :href="contact.linkedin_url" target="_blank" rel="noopener noreferrer" class="link">Voir le profil</a><template v-else>—</template></dd>
              <dt>Responsable commercial</dt><dd>{{ contact.owner?.name ?? '—' }}</dd>
              <template v-if="contact.notes"><dt>Notes</dt><dd style="white-space: pre-wrap">{{ contact.notes }}</dd></template>
            </dl>
          </section>

          <section v-if="can('needs')" class="card flush">
            <div class="card-head"><h2>Besoins ({{ needs.length }})</h2><button v-if="can('needs', 'create')" class="btn small" type="button" @click="addingNeed = true"><Icon name="plus" /> Besoin</button></div>
            <EmptyState v-if="!needs.length" compact title="Aucun besoin">Enregistrez les demandes exprimées par ce contact.</EmptyState>
            <div v-else class="table-wrap">
              <table class="table"><tbody>
                <tr v-for="n in needs" :key="n.id" class="click" @click="router.push(`/needs/${n.id}`)">
                  <td><RouterLink :to="`/needs/${n.id}`" class="name" @click.stop>{{ n.title }}</RouterLink></td>
                  <td><RefPill :id="n.state_id" /></td>
                  <td class="muted">{{ n.start_date ? 'dès le ' + fmtDate(n.start_date) : '' }}</td>
                  <td class="right muted">{{ n.positionings_count }} positionnement(s)</td>
                </tr>
              </tbody></table>
            </div>
          </section>

          <section v-if="can('projects') && projects.length" class="card flush">
            <div class="card-head"><h2>Projets ({{ projects.length }})</h2></div>
            <div class="table-wrap"><table class="table"><tbody>
              <tr v-for="p in projects" :key="p.id" class="click" @click="router.push(`/projects/${p.id}`)">
                <td><RouterLink :to="`/projects/${p.id}`" class="name" @click.stop>{{ p.title }}</RouterLink></td>
                <td><RefPill :id="p.state_id" /></td>
                <td class="muted nowrap">{{ fmtDate(p.start_date) }} → {{ p.end_date ? fmtDate(p.end_date) : 'en cours' }}</td>
              </tr>
            </tbody></table></div>
          </section>
        </div>

        <div class="stack">
          <ActivityPanel :filter="{ contact_id: id }" :links="{ contact_id: id }" area="crm" :key="refresh" />
          <HistoryPanel entity="contact" :id="id" :refresh-key="refresh" />
        </div>
      </div>

      <ContactForm v-if="editing" :contact="contact" @close="editing = false" @saved="editing = false; load()" />
      <NeedForm v-if="addingNeed" :contact-id="id" @close="addingNeed = false" @saved="(n) => { addingNeed = false; router.push(`/needs/${n.id}`) }" />
    </template>
  </div>
</template>
