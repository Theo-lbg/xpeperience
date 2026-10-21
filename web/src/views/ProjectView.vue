<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import ActivityPanel from '../components/ActivityPanel.vue'
import DeliveryForm from '../components/DeliveryForm.vue'
import EmptyState from '../components/EmptyState.vue'
import ExtensionForm from '../components/ExtensionForm.vue'
import HistoryPanel from '../components/HistoryPanel.vue'
import Icon from '../components/Icon.vue'
import ProjectForm from '../components/ProjectForm.vue'
import RefPill from '../components/RefPill.vue'
import { ApiError, api } from '../lib/api'
import { fmtDate, fmtMoney, fmtNumber } from '../lib/format'
import { can, confirmAction, notify, refLabel } from '../lib/store'
import type { Delivery, Extension, Project } from '../lib/types'

const route = useRoute()
const router = useRouter()
const id = Number(route.params.id)
const project = ref<Project | null>(null)
const deliveries = ref<Delivery[]>([])
const error = ref('')
const editing = ref(false)
const delivery = ref<{ d?: Delivery } | null>(null)
const extension = ref<{ delivery: Delivery; e?: Extension } | null>(null)
const refresh = ref(0)
const showCost = can('costs', 'read')

async function load() {
  try {
    project.value = await api.get<Project>(`/api/projects/${id}`)
    deliveries.value = await api.get<Delivery[]>('/api/deliveries', { project_id: id })
    refresh.value++
  } catch (e) {
    error.value = e instanceof ApiError ? e.message : 'Chargement impossible.'
  }
}
onMounted(load)

async function archive() {
  const p = project.value!
  if (!p.archived && !(await confirmAction('Archiver ce projet ?', 'Il sortira des listes actives. Prestations et historique sont conservés.', 'Archiver'))) return
  project.value = await api.post<Project>(`/api/projects/${id}/archive`, { archived: !p.archived })
  notify(p.archived ? 'Projet restauré.' : 'Projet archivé.')
  refresh.value++
}
async function remove() {
  if (!(await confirmAction('Supprimer ce projet ?', 'Ses prestations seront supprimées avec lui. Irréversible : préférez l’archivage pour garder l’historique.', 'Supprimer', true))) return
  await api.del(`/api/projects/${id}`)
  notify('Projet supprimé.')
  router.replace('/projects')
}
async function removeDelivery(d: Delivery) {
  if (!(await confirmAction('Supprimer cette prestation ?', 'Les dates du projet seront recalculées.', 'Supprimer', true))) return
  await api.del(`/api/deliveries/${d.id}`)
  notify('Prestation supprimée.')
  await load()
}
async function removeExtension(e: Extension) {
  if (!(await confirmAction('Supprimer cette prolongation ?', 'Les dates de la prestation et du projet seront recalculées.', 'Supprimer', true))) return
  await api.del(`/api/extensions/${e.id}`)
  await load()
}
</script>

<template>
  <div class="page">
    <p v-if="error" class="alert error" role="alert">{{ error }} <RouterLink to="/projects" class="link">Retour aux projets</RouterLink></p>
    <template v-if="project">
      <nav class="crumbs">
        <RouterLink to="/projects">Projets</RouterLink><span>/</span>
        <template v-if="project.company"><RouterLink :to="`/crm/companies/${project.company.id}`">{{ project.company.name }}</RouterLink><span>/</span></template>
        <span>{{ project.title }}</span>
      </nav>
      <div class="page-head">
        <div>
          <h1>{{ project.title }}</h1>
          <div class="row wrap" style="margin-top: 8px"><RefPill :id="project.type_id" /><RefPill :id="project.state_id" /><span v-if="project.archived" class="pill gray">Archivé</span></div>
        </div>
        <div class="row wrap">
          <template v-if="can('projects', 'update')">
            <button class="btn" type="button" @click="editing = true"><Icon name="edit" /> Modifier</button>
            <button class="btn" type="button" @click="archive"><Icon :name="project.archived ? 'restore' : 'archive'" /> {{ project.archived ? 'Restaurer' : 'Archiver' }}</button>
          </template>
          <button v-if="can('projects', 'delete')" class="btn danger icon" type="button" aria-label="Supprimer" @click="remove"><Icon name="trash" /></button>
        </div>
      </div>

      <div class="split">
        <div class="stack">
          <section class="card">
            <div class="card-head"><h2>Le projet</h2></div>
            <dl class="kv">
              <dt>Société cliente</dt><dd><RouterLink v-if="project.company" :to="`/crm/companies/${project.company.id}`" class="link">{{ project.company.name }}</RouterLink><template v-else>Projet interne</template></dd>
              <dt>Contact</dt><dd><RouterLink v-if="project.contact" :to="`/crm/contacts/${project.contact.id}`" class="link">{{ project.contact.name }}</RouterLink><template v-else>—</template></dd>
              <dt>Responsable</dt><dd>{{ project.manager?.name ?? '—' }}</dd>
              <dt>Référence</dt><dd>{{ project.reference ?? '—' }}</dd>
              <dt>Début</dt><dd>{{ fmtDate(project.start_date) }}</dd>
              <dt>Fin</dt><dd>{{ project.end_date ? fmtDate(project.end_date) : 'Non définie' }}</dd>
              <dt v-if="project.forecast_end_date && project.forecast_end_date !== project.end_date">Fin prévisionnelle</dt>
              <dd v-if="project.forecast_end_date && project.forecast_end_date !== project.end_date">{{ fmtDate(project.forecast_end_date) }} <span class="faint small">(avec prolongations prévisionnelles)</span></dd>
              <dt>Besoin d'origine</dt><dd><RouterLink v-if="project.need" :to="`/needs/${project.need.id}`" class="link">{{ project.need.name }}</RouterLink><template v-else>—</template></dd>
              <dt>Ressources</dt><dd><div v-if="project.resources.length" class="tags"><RouterLink v-for="r in project.resources" :key="r.id" :to="`/resources/${r.id}`" class="tag link" style="border: 0">{{ r.name }}</RouterLink></div><template v-else>—</template></dd>
              <template v-if="project.notes"><dt>Notes</dt><dd style="white-space: pre-wrap">{{ project.notes }}</dd></template>
            </dl>
            <p v-if="project.dates_from_deliveries" class="faint small" style="margin-top: 12px">Les dates suivent les prestations : elles évoluent quand vous modifiez ou prolongez une prestation.</p>
          </section>

          <section class="card">
            <div class="card-head">
              <h2>Prestations ({{ deliveries.length }})</h2>
              <button v-if="can('projects', 'update') && !project.archived" class="btn primary small" type="button" @click="delivery = {}"><Icon name="plus" /> Prestation</button>
            </div>
            <EmptyState v-if="!deliveries.length" icon="briefcase" title="Aucune prestation">
              Une prestation associe une ressource au projet, avec une période, un tarif et des jours vendus.
              <template v-if="can('projects', 'update') && !project.archived" #actions><button class="btn primary" type="button" @click="delivery = {}">Ajouter une prestation</button></template>
            </EmptyState>
            <div v-else class="stack">
              <article v-for="d in deliveries" :key="d.id" class="deliv">
                <div class="row between wrap">
                  <div>
                    <RouterLink v-if="d.resource" :to="`/resources/${d.resource.id}`" class="name link">{{ d.resource.name }}</RouterLink>
                    <strong v-else>Ressource à affecter</strong>
                    <span v-if="d.title || d.type_id" class="muted"> · {{ d.title ?? refLabel(d.type_id) }}</span>
                  </div>
                  <div v-if="can('projects', 'update')" class="row" style="gap: 2px">
                    <button class="btn small" type="button" @click="extension = { delivery: d }"><Icon name="repeat" /> Prolonger</button>
                    <button class="btn ghost small icon" type="button" aria-label="Modifier la prestation" @click="delivery = { d }"><Icon name="edit" /></button>
                    <button class="btn ghost small icon" type="button" aria-label="Supprimer la prestation" @click="removeDelivery(d)"><Icon name="trash" /></button>
                  </div>
                </div>
                <dl class="facts">
                  <div><dt>Période</dt><dd>{{ fmtDate(d.start_date) }} → {{ d.end_date ? fmtDate(d.end_date) : 'sans fin' }}</dd></div>
                  <div><dt>Tarif / jour</dt><dd>{{ fmtMoney(d.daily_rate) }}</dd></div>
                  <div><dt>Jours vendus</dt><dd>{{ fmtNumber(d.sold_days) }}</dd></div>
                  <div v-if="showCost"><dt>CJM</dt><dd>{{ d.daily_cost != null ? fmtMoney(d.daily_cost) : 'Non renseigné' }}</dd></div>
                </dl>
                <p v-if="d.notes" class="muted small" style="white-space: pre-wrap">{{ d.notes }}</p>
                <div v-if="d.extensions.length" class="exts">
                  <div v-for="e in d.extensions" :key="e.id" class="ext">
                    <span class="pill plain" :class="e.status === 'signed' ? 'green' : 'orange'">Prolongation {{ e.status === 'signed' ? 'signée' : 'prévisionnelle' }}</span>
                    <span>{{ fmtDate(e.start_date) }} → {{ fmtDate(e.end_date) }}</span>
                    <span v-if="e.days != null" class="muted">{{ fmtNumber(e.days, 'j') }}</span>
                    <span v-if="e.daily_rate != null" class="muted">{{ fmtMoney(e.daily_rate) }}/j</span>
                    <span class="grow" />
                    <template v-if="can('projects', 'update')">
                      <button class="btn ghost small icon" type="button" aria-label="Modifier la prolongation" @click="extension = { delivery: d, e }"><Icon name="edit" /></button>
                      <button class="btn ghost small icon" type="button" aria-label="Supprimer la prolongation" @click="removeExtension(e)"><Icon name="trash" /></button>
                    </template>
                  </div>
                </div>
              </article>
            </div>
          </section>
        </div>

        <div class="stack">
          <ActivityPanel :filter="{ project_id: id }" :links="{ project_id: id }" area="projects" title="Suivi du projet" :key="refresh" />
          <HistoryPanel entity="project" :id="id" :refresh-key="refresh" />
        </div>
      </div>

      <ProjectForm v-if="editing" :project="project" @close="editing = false" @saved="editing = false; load()" />
      <DeliveryForm v-if="delivery" :project-id="id" :delivery="delivery.d" @close="delivery = null" @saved="delivery = null; load()" />
      <ExtensionForm v-if="extension" :delivery="extension.delivery" :extension="extension.e" @close="extension = null" @saved="extension = null; load()" />
    </template>
  </div>
</template>

<style scoped>
.deliv { border: 1px solid var(--line); border-radius: 14px; padding: 14px 16px; display: flex; flex-direction: column; gap: 10px; }
.facts { display: grid; grid-template-columns: repeat(auto-fit, minmax(130px, 1fr)); gap: 10px; margin: 0; }
.facts dt { color: var(--ink-3); font-size: 12px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.03em; }
.facts dd { margin: 2px 0 0; font-weight: 600; }
.exts { display: flex; flex-direction: column; gap: 6px; border-top: 1px dashed var(--line-strong); padding-top: 10px; }
.ext { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; font-size: 13.5px; }
</style>
