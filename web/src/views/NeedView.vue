<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import ActivityPanel from '../components/ActivityPanel.vue'
import ConvertDialog from '../components/ConvertDialog.vue'
import EmptyState from '../components/EmptyState.vue'
import HistoryPanel from '../components/HistoryPanel.vue'
import Icon from '../components/Icon.vue'
import NeedForm from '../components/NeedForm.vue'
import PositioningDialog from '../components/PositioningDialog.vue'
import RefPill from '../components/RefPill.vue'
import { ApiError, api } from '../lib/api'
import { RESOURCE_KIND_LABEL, fmtDate, fmtMoney, fmtNumber } from '../lib/format'
import { can, confirmAction, notify, refCategory, refLabel, refsOf } from '../lib/store'
import type { Need, Positioning, PositioningSaved } from '../lib/types'

const route = useRoute()
const router = useRouter()
const id = Number(route.params.id)
const need = ref<Need | null>(null)
const positionings = ref<Positioning[]>([])
const error = ref('')
const editing = ref(false)
const positioning = ref<{ p?: Positioning } | null>(null)
const converting = ref<Positioning | null>(null)
const refresh = ref(0)
const showCost = can('costs', 'read')

async function load() {
  try {
    need.value = await api.get<Need>(`/api/needs/${id}`)
    positionings.value = can('positionings') ? await api.get<Positioning[]>('/api/positionings', { need_id: id }) : []
    refresh.value++
  } catch (e) {
    error.value = e instanceof ApiError ? e.message : 'Chargement impossible.'
  }
}
onMounted(load)

async function changeState(e: Event) {
  const stateId = Number((e.target as HTMLSelectElement).value)
  try {
    need.value = await api.put<Need>(`/api/needs/${id}/state`, { state_id: stateId })
    notify('État mis à jour.')
    refresh.value++
  } catch (err) {
    notify(err instanceof ApiError ? err.message : 'Changement impossible.', 'error')
    ;(e.target as HTMLSelectElement).value = String(need.value!.state_id)
  }
}

async function archive() {
  const n = need.value!
  if (!n.archived && !(await confirmAction('Archiver ce besoin ?', 'Il sortira des listes actives. Son historique est conservé et vous pourrez le restaurer.', 'Archiver'))) return
  need.value = await api.post<Need>(`/api/needs/${id}/archive`, { archived: !n.archived })
  notify(n.archived ? 'Besoin restauré.' : 'Besoin archivé.')
  refresh.value++
}

async function remove() {
  if (!(await confirmAction('Supprimer ce besoin ?', 'Ses positionnements seront supprimés avec lui. Cette action est irréversible.', 'Supprimer', true))) return
  try {
    await api.del(`/api/needs/${id}`)
    notify('Besoin supprimé.')
    router.replace('/needs')
  } catch (e) {
    notify(e instanceof ApiError ? e.message : 'Suppression impossible.', 'error')
  }
}

async function removePositioning(p: Positioning) {
  if (!(await confirmAction('Retirer ce positionnement ?', `${p.resource.name} ne sera plus associé(e) à ce besoin.`, 'Retirer', true))) return
  await api.del(`/api/positionings/${p.id}`)
  notify('Positionnement retiré.')
  await load()
}

function onSaved(s: PositioningSaved) {
  positioning.value = null
  if (s.suggest_project) converting.value = s.positioning
  load()
}
async function convertForfait() {
  const project = await api.post<{ id: number }>(`/api/needs/${id}/convert`)
  notify('Projet créé à partir du besoin.', 'success', { to: `/projects/${project.id}`, action: 'Ouvrir' })
  await load()
}

const excluded = computed(() => !!need.value && refCategory(need.value.state_id) === 'lost')
const kind = computed(() => refCategory(need.value?.type_id))
const canConvert = computed(() => !!need.value && !need.value.project && can('projects', 'create') && can('needs', 'update') && (kind.value === 'forfait' || kind.value === 'produit'))
const money = (p: Positioning) => (p.daily_rate != null && p.days != null ? fmtMoney(p.daily_rate * p.days) : '—')
</script>

<template>
  <div class="page">
    <p v-if="error" class="alert error" role="alert">{{ error }} <RouterLink to="/needs" class="link">Retour aux besoins</RouterLink></p>
    <template v-if="need">
      <nav class="crumbs">
        <RouterLink to="/needs">Besoins</RouterLink><span>/</span>
        <template v-if="need.company"><RouterLink :to="`/crm/companies/${need.company.id}`">{{ need.company.name }}</RouterLink><span>/</span></template>
        <span>{{ need.title }}</span>
      </nav>

      <div class="page-head">
        <div>
          <h1>{{ need.title }}</h1>
          <div class="row wrap" style="margin-top: 8px">
            <RefPill :id="need.type_id" />
            <span v-if="need.archived" class="pill gray">Archivé</span>
            <RouterLink v-if="need.project" :to="`/projects/${need.project.id}`" class="pill green"><Icon name="folder" style="width: 13px" /> Projet : {{ need.project.name }}</RouterLink>
          </div>
        </div>
        <div class="row wrap">
          <div v-if="can('needs', 'update')" class="field" style="min-width: 190px">
            <select class="select" aria-label="État du besoin" :value="need.state_id" @change="changeState"><option v-for="s in refsOf('need_state')" :key="s.id" :value="s.id">{{ s.label }}</option></select>
          </div>
          <RefPill v-else :id="need.state_id" />
          <button v-if="canConvert" class="btn primary" type="button" @click="convertForfait"><Icon name="folder" /> Créer le projet</button>
          <template v-if="can('needs', 'update')">
            <button class="btn" type="button" @click="editing = true"><Icon name="edit" /> Modifier</button>
            <button class="btn" type="button" @click="archive"><Icon :name="need.archived ? 'restore' : 'archive'" /> {{ need.archived ? 'Restaurer' : 'Archiver' }}</button>
          </template>
          <button v-if="can('needs', 'delete')" class="btn danger icon" type="button" aria-label="Supprimer" @click="remove"><Icon name="trash" /></button>
        </div>
      </div>

      <div class="split">
        <div class="stack">
          <section class="card">
            <div class="card-head"><h2>Le besoin</h2></div>
            <dl class="kv">
              <dt>Société</dt><dd><RouterLink v-if="need.company" :to="`/crm/companies/${need.company.id}`" class="link">{{ need.company.name }}</RouterLink><template v-else>—</template></dd>
              <dt>Contact</dt><dd><RouterLink v-if="need.contact" :to="`/crm/contacts/${need.contact.id}`" class="link">{{ need.contact.name }}</RouterLink></dd>
              <dt>Provenance</dt><dd>{{ refLabel(need.origin_id) || '—' }}</dd>
              <dt>Démarrage souhaité</dt><dd>{{ fmtDate(need.start_date) }}</dd>
              <dt>Durée</dt><dd>{{ fmtNumber(need.duration_months, 'mois') }}</dd>
              <dt>Jours prévus</dt><dd>{{ fmtNumber(need.planned_days, 'jours') }}</dd>
              <template v-if="need.budget != null"><dt>Budget éventuel</dt><dd>{{ fmtMoney(need.budget) }}</dd></template>
              <template v-if="need.tender_reference"><dt>Réf. appel d’offres</dt><dd>{{ need.tender_reference }}</dd></template>
              <dt>Lieu</dt><dd>{{ need.location ?? '—' }}</dd>
              <dt>Responsable commercial</dt><dd>{{ need.owner?.name ?? '—' }}</dd>
              <dt>Compétences</dt><dd><div v-if="need.skills.length" class="tags"><span v-for="s in need.skills" :key="s" class="tag">{{ s }}</span></div><template v-else>—</template></dd>
              <template v-if="need.description"><dt>Description</dt><dd style="white-space: pre-wrap">{{ need.description }}</dd></template>
            </dl>
          </section>

          <section v-if="can('positionings')" class="card flush">
            <div class="card-head">
              <h2>Ressources positionnées ({{ positionings.length }})</h2>
              <button v-if="can('positionings', 'create') && !need.archived" class="btn primary small" type="button" @click="positioning = {}"><Icon name="plus" /> Positionner</button>
            </div>
            <EmptyState v-if="!positionings.length" icon="briefcase" title="Aucune ressource positionnée">
              Proposez des profils pour ce besoin : par sélection directe, recherche filtrée ou suggestions.
              <template v-if="can('positionings', 'create') && !need.archived" #actions><button class="btn primary" type="button" @click="positioning = {}">Positionner une ressource</button></template>
            </EmptyState>
            <div v-else class="table-wrap">
              <table class="table">
                <thead><tr><th>Ressource</th><th>État</th><th>Période prévue</th><th class="right">Tarif / jour</th><th class="right">Jours</th><th v-if="showCost" class="right">CJM</th><th /></tr></thead>
                <tbody>
                  <tr v-for="p in positionings" :key="p.id">
                    <td><RouterLink :to="`/resources/${p.resource.id}`" class="name">{{ p.resource.name }}</RouterLink><div class="sub">{{ RESOURCE_KIND_LABEL[p.resource_kind] }}<template v-if="p.resource_title"> · {{ p.resource_title }}</template></div></td>
                    <td>
                      <RefPill :id="p.state_id" />
                      <RouterLink v-if="p.project" :to="`/projects/${p.project.id}`" class="small link" style="margin-left: 6px">Projet</RouterLink>
                    </td>
                    <td class="nowrap">{{ p.start_date ? fmtDate(p.start_date) : '—' }} → {{ p.end_date ? fmtDate(p.end_date) : '—' }}</td>
                    <td class="right">{{ fmtMoney(p.daily_rate) }}</td>
                    <td class="right">{{ fmtNumber(p.days) }}</td>
                    <td v-if="showCost" class="right">{{ fmtMoney(p.daily_cost) }}</td>
                    <td class="right nowrap">
                      <button v-if="refCategory(p.state_id) === 'won' && !p.project && can('projects', 'create')" class="btn small primary" type="button" @click="converting = p">Créer le projet</button>
                      <template v-if="can('positionings', 'update')">
                        <button class="btn ghost small icon" type="button" aria-label="Modifier" @click="positioning = { p }"><Icon name="edit" /></button>
                      </template>
                      <button v-if="can('positionings', 'delete')" class="btn ghost small icon" type="button" aria-label="Retirer" @click="removePositioning(p)"><Icon name="trash" /></button>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>
          <p v-if="excluded" class="alert">Ce besoin est perdu : vous pouvez le rouvrir en changeant son état.</p>
        </div>

        <div class="stack">
          <ActivityPanel :filter="{ need_id: id }" :links="{ need_id: id }" area="needs" title="Échanges sur ce besoin" :key="refresh" />
          <HistoryPanel entity="need" :id="id" :refresh-key="refresh" />
        </div>
      </div>

      <NeedForm v-if="editing" :need="need" @close="editing = false" @saved="editing = false; load()" />
      <PositioningDialog v-if="positioning" :need="need" :positioning="positioning.p" @close="positioning = null" @saved="onSaved" />
      <ConvertDialog v-if="converting" :positioning="converting" @close="converting = null" @done="converting = null; load()" />
    </template>
  </div>
</template>
