<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import ActivityPanel from '../components/ActivityPanel.vue'
import EmptyState from '../components/EmptyState.vue'
import HistoryPanel from '../components/HistoryPanel.vue'
import Icon from '../components/Icon.vue'
import RefPill from '../components/RefPill.vue'
import ResourceForm from '../components/ResourceForm.vue'
import { ApiError, api } from '../lib/api'
import { FILE_KIND_LABEL, RESOURCE_KIND_LABEL, fmtDate, fmtMoney, fmtNumber, fmtSize, initials } from '../lib/format'
import { can, confirmAction, notify, refLabel } from '../lib/store'
import type { Delivery, Positioning, Resource, ResourceFile } from '../lib/types'

const route = useRoute()
const router = useRouter()
const id = Number(route.params.id)
const resource = ref<Resource | null>(null)
const positionings = ref<Positioning[]>([])
const deliveries = ref<Delivery[]>([])
const files = ref<ResourceFile[]>([])
const error = ref('')
const editing = ref(false)
const refresh = ref(0)
const uploadKind = ref<'cv' | 'skills_file' | 'other'>('cv')
const uploading = ref(false)
const over = ref(false)
const showCost = can('costs', 'read')

async function load() {
  try {
    resource.value = await api.get<Resource>(`/api/resources/${id}`)
    const [p, d, f] = await Promise.all([
      can('positionings') ? api.get<Positioning[]>('/api/positionings', { resource_id: id }) : Promise.resolve([]),
      can('projects') ? api.get<Delivery[]>('/api/deliveries', { resource_id: id }) : Promise.resolve([]),
      api.get<ResourceFile[]>(`/api/resources/${id}/files`),
    ])
    positionings.value = p
    deliveries.value = d
    files.value = f
    refresh.value++
  } catch (e) {
    error.value = e instanceof ApiError ? e.message : 'Chargement impossible.'
  }
}
onMounted(load)

async function upload(list: FileList | null) {
  const file = list?.[0]
  if (!file) return
  uploading.value = true
  try {
    await api.upload(`/api/resources/${id}/files?kind=${uploadKind.value}`, file)
    notify('Document ajouté.')
    await load()
  } catch (e) {
    notify(e instanceof ApiError ? e.message : 'Envoi impossible.', 'error')
  } finally {
    uploading.value = false
    over.value = false
  }
}

async function removeFile(f: ResourceFile) {
  if (!(await confirmAction('Supprimer ce document ?', `« ${f.filename} » sera supprimé définitivement.`, 'Supprimer', true))) return
  await api.del(`/api/files/${f.id}`)
  notify('Document supprimé.')
  await load()
}

async function archive() {
  const r = resource.value!
  if (!r.archived && !(await confirmAction('Archiver cette fiche ?', 'Elle n’apparaîtra plus dans les listes ni dans les suggestions. Son historique est conservé.', 'Archiver'))) return
  resource.value = await api.post<Resource>(`/api/resources/${id}/archive`, { archived: !r.archived })
  notify(r.archived ? 'Fiche restaurée.' : 'Fiche archivée.')
  refresh.value++
}
async function remove() {
  if (!(await confirmAction('Supprimer définitivement cette fiche ?', 'Irréversible. Impossible si la ressource a des positionnements ou des prestations : archivez-la alors.', 'Supprimer', true))) return
  try {
    await api.del(`/api/resources/${id}`)
    notify('Fiche supprimée.')
    router.replace('/resources')
  } catch (e) {
    notify(e instanceof ApiError ? e.message : 'Suppression impossible.', 'error')
  }
}

const today = new Date().toISOString().slice(0, 10)
const availability = computed(() => {
  const r = resource.value!
  if (r.on_mission) return { text: r.on_mission_until ? `En mission jusqu’au ${fmtDate(r.on_mission_until)}` : 'En mission (sans fin définie)', cls: 'blue' }
  if (r.availability_date && r.availability_date > today) return { text: `Disponible le ${fmtDate(r.availability_date)}`, cls: 'orange' }
  return { text: 'Disponible', cls: 'green' }
})
</script>

<template>
  <div class="page">
    <p v-if="error" class="alert error" role="alert">{{ error }} <RouterLink to="/resources" class="link">Retour aux ressources</RouterLink></p>
    <template v-if="resource">
      <nav class="crumbs"><RouterLink to="/resources">Profils</RouterLink><span>/</span><span>{{ resource.first_name }} {{ resource.last_name }}</span></nav>
      <div class="page-head">
        <div class="row" style="gap: 16px">
          <span class="avatar lg">{{ initials(`${resource.first_name} ${resource.last_name}`) }}</span>
          <div>
            <h1>{{ resource.first_name }} {{ resource.last_name }}</h1>
            <div class="muted" style="margin-top: 4px">{{ resource.title ?? '' }}</div>
            <div class="row wrap" style="margin-top: 8px">
              <span class="pill" :class="resource.kind === 'consultant' ? 'green' : 'purple'">{{ RESOURCE_KIND_LABEL[resource.kind] }}</span>
              <span class="pill" :class="availability.cls">{{ availability.text }}</span>
              <span v-if="resource.archived" class="pill gray">Archivé</span>
            </div>
          </div>
        </div>
        <div class="row wrap">
          <template v-if="can('resources', 'update')">
            <button class="btn" type="button" @click="editing = true"><Icon name="edit" /> Modifier</button>
            <button class="btn" type="button" @click="archive"><Icon :name="resource.archived ? 'restore' : 'archive'" /> {{ resource.archived ? 'Restaurer' : 'Archiver' }}</button>
          </template>
          <button v-if="can('resources', 'delete')" class="btn danger icon" type="button" aria-label="Supprimer" @click="remove"><Icon name="trash" /></button>
        </div>
      </div>

      <div class="split">
        <div class="stack">
          <section class="card">
            <div class="card-head"><h2>Profil</h2></div>
            <dl class="kv">
              <dt>E-mail</dt><dd><a v-if="resource.email" :href="`mailto:${resource.email}`" class="link">{{ resource.email }}</a><template v-else>—</template></dd>
              <dt>Téléphone</dt><dd>{{ resource.phone ?? '—' }}</dd>
              <dt>Localisation</dt><dd>{{ resource.location ?? '—' }}</dd>
              <dt>LinkedIn</dt><dd><a v-if="resource.linkedin_url" :href="resource.linkedin_url" target="_blank" rel="noopener noreferrer" class="link">Voir le profil</a><template v-else>—</template></dd>
              <dt>Expérience</dt><dd>{{ resource.experience_years != null ? fmtNumber(resource.experience_years, 'an(s)') : '—' }}</dd>
              <dt>Disponibilité</dt><dd>{{ resource.availability_date ? fmtDate(resource.availability_date) : 'Immédiate' }}<template v-if="resource.availability_note"> — {{ resource.availability_note }}</template></dd>
              <dt>Type de contrat</dt><dd><RefPill :id="resource.contract_type_id" empty="—" /></dd>
              <dt v-if="showCost">CJM</dt>
              <dd v-if="showCost">
                <template v-if="resource.daily_cost != null">{{ fmtMoney(resource.daily_cost) }}</template>
                <span v-else class="faint">Non renseigné — il pourra être complété plus tard, sans bloquer les positionnements.</span>
              </dd>
              <dt>Compétences</dt><dd><div v-if="resource.skills.length" class="tags"><span v-for="s in resource.skills" :key="s" class="tag">{{ s }}</span></div><template v-else>—</template></dd>
              <template v-if="resource.experience"><dt>Parcours</dt><dd style="white-space: pre-wrap">{{ resource.experience }}</dd></template>
              <template v-if="resource.notes"><dt>Notes</dt><dd style="white-space: pre-wrap">{{ resource.notes }}</dd></template>
            </dl>
          </section>

          <section class="card">
            <div class="card-head">
              <h2>CV et dossier de compétences</h2>
              <div v-if="can('resources', 'update')" class="row">
                <select v-model="uploadKind" class="select" style="width: auto; border-radius: 999px; height: 32px" aria-label="Type de document"><option value="cv">CV</option><option value="skills_file">Dossier de compétences</option><option value="other">Autre</option></select>
                <label class="btn small primary" :class="{ disabled: uploading }"><Icon name="upload" /> {{ uploading ? 'Envoi…' : 'Ajouter' }}<input type="file" hidden accept=".pdf,.doc,.docx,.odt,.txt,.png,.jpg,.jpeg" :disabled="uploading" @change="upload(($event.target as HTMLInputElement).files)" /></label>
              </div>
            </div>
            <div v-if="!files.length" class="dropzone" :class="{ over }" @dragover.prevent="over = true" @dragleave="over = false" @drop.prevent="can('resources', 'update') && upload($event.dataTransfer?.files ?? null)">
              Aucun document. {{ can('resources', 'update') ? 'Déposez un fichier ici (PDF, Word, image — 15 Mo max).' : '' }}
            </div>
            <div v-else class="list">
              <div v-for="f in files" :key="f.id" class="row">
                <Icon name="file" style="width: 20px; color: var(--ink-3)" />
                <div class="grow"><a :href="`/api/files/${f.id}`" class="link" download>{{ f.filename }}</a><div class="small faint">{{ FILE_KIND_LABEL[f.kind] }} · {{ fmtSize(f.size) }} · {{ fmtDate(f.uploaded_at) }}</div></div>
                <a v-if="f.mime === 'application/pdf'" class="btn small ghost" :href="`/api/files/${f.id}?inline=true`" target="_blank" rel="noopener">Aperçu</a>
                <button v-if="can('resources', 'update')" class="btn ghost small icon" type="button" aria-label="Supprimer" @click="removeFile(f)"><Icon name="trash" /></button>
              </div>
            </div>
          </section>

          <section v-if="can('positionings')" class="card flush">
            <div class="card-head"><h2>Positionnements ({{ positionings.length }})</h2></div>
            <EmptyState v-if="!positionings.length" compact title="Aucun positionnement">Depuis la fiche d'un besoin, positionnez cette ressource.</EmptyState>
            <div v-else class="table-wrap"><table class="table"><tbody>
              <tr v-for="p in positionings" :key="p.id" class="click" @click="router.push(`/needs/${p.need.id}`)">
                <td><RouterLink :to="`/needs/${p.need.id}`" class="name" @click.stop>{{ p.need.name }}</RouterLink><div class="sub">{{ p.company?.name }}</div></td>
                <td><RefPill :id="p.state_id" /></td>
                <td class="muted nowrap">{{ p.start_date ? fmtDate(p.start_date) : '—' }}</td>
              </tr>
            </tbody></table></div>
          </section>

          <section v-if="can('projects')" class="card flush">
            <div class="card-head"><h2>Projets et prestations ({{ deliveries.length }})</h2></div>
            <EmptyState v-if="!deliveries.length" compact title="Aucune prestation">Les prestations apparaissent ici quand la ressource est affectée à un projet.</EmptyState>
            <div v-else class="table-wrap"><table class="table"><tbody>
              <tr v-for="d in deliveries" :key="d.id" class="click" @click="router.push(`/projects/${d.project.id}`)">
                <td><RouterLink :to="`/projects/${d.project.id}`" class="name" @click.stop>{{ d.project.name }}</RouterLink><div class="sub">{{ d.company?.name }}</div></td>
                <td class="muted nowrap">{{ fmtDate(d.start_date) }} → {{ d.forecast_end_date ? fmtDate(d.forecast_end_date) : 'sans fin' }}</td>
                <td class="right">{{ fmtMoney(d.daily_rate) }}</td>
              </tr>
            </tbody></table></div>
          </section>
        </div>

        <div class="stack">
          <ActivityPanel :filter="{ resource_id: id }" :links="{ resource_id: id }" area="resources" title="Échanges et notes" :key="refresh" />
          <HistoryPanel entity="resource" :id="id" :refresh-key="refresh" />
        </div>
      </div>

      <ResourceForm v-if="editing" :resource="resource" @close="editing = false" @saved="editing = false; load()" />
    </template>
  </div>
</template>

<style scoped>
label.btn.disabled { opacity: 0.6; pointer-events: none; }
</style>
