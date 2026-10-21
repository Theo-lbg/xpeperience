<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { api } from '../lib/api'
import { RESOURCE_KIND_LABEL, fmtDate, fmtMoney, initials, num, orNull } from '../lib/format'
import { useForm } from '../lib/forms'
import { can, firstRef, notify, refsOf } from '../lib/store'
import type { Need, Positioning, PositioningSaved, Resource, Suggestion } from '../lib/types'
import EmptyState from './EmptyState.vue'
import Field from './Field.vue'
import Icon from './Icon.vue'
import Modal from './Modal.vue'

const props = defineProps<{ need: Need; positioning?: Positioning; resource?: Resource }>()
const emit = defineEmits<{ close: []; saved: [PositioningSaved] }>()
const { errors, message, busy, conflict, submit } = useForm()

const editing = !!props.positioning
const picked = ref<Resource | null>(props.resource ?? null)
const mode = ref<'direct' | 'search' | 'suggest'>('suggest')

// ── Choix de la ressource ──
const all = ref<Resource[]>([])
const suggestions = ref<Suggestion[]>([])
const loading = ref(true)
const q = ref('')
const skill = ref('')
const availability = ref('')
const showCost = can('costs', 'read')

onMounted(async () => {
  if (editing || picked.value) {
    loading.value = false
    return
  }
  try {
    const [r, s] = await Promise.all([api.get<Resource[]>('/api/resources'), api.get<Suggestion[]>(`/api/needs/${props.need.id}/suggestions`)])
    all.value = r
    suggestions.value = s
    if (!s.length) mode.value = 'direct'
  } finally {
    loading.value = false
  }
})

const filtered = computed(() => {
  const needle = (s: string) => s.toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, '')
  return all.value.filter((r) => {
    if (mode.value === 'direct' && q.value && !needle(`${r.first_name} ${r.last_name}`).includes(needle(q.value))) return false
    if (mode.value === 'search') {
      if (skill.value && !r.skills.some((s) => needle(s).includes(needle(skill.value)))) return false
      if (q.value && !needle(`${r.first_name} ${r.last_name} ${r.title ?? ''}`).includes(needle(q.value))) return false
      if (availability.value === 'now' && (r.on_mission || (r.availability_date && r.availability_date > new Date().toISOString().slice(0, 10)))) return false
    }
    return true
  })
})

// ── Qualification ──
const p = props.positioning
const form = reactive({
  state_id: p?.state_id ?? firstRef('positioning_state', 'open'),
  start_date: p?.start_date ?? props.need.start_date ?? '',
  end_date: p?.end_date ?? '',
  daily_rate: p?.daily_rate?.toString() ?? '',
  days: p?.days?.toString() ?? props.need.planned_days?.toString() ?? '',
  daily_cost: p?.daily_cost?.toString() ?? '',
  notes: p?.notes ?? '',
})
watch(picked, (r) => {
  if (r && !editing && !form.daily_cost) form.daily_cost = r.effective_daily_cost?.toString() ?? ''
}, { immediate: true })

async function save() {
  const body = {
    need_id: props.need.id, resource_id: picked.value?.id, state_id: form.state_id, start_date: orNull(form.start_date), end_date: orNull(form.end_date),
    daily_rate: num(form.daily_rate), days: num(form.days), daily_cost: num(form.daily_cost), notes: orNull(form.notes),
  }
  const saved = await submit(
    () => (p ? api.put<PositioningSaved>(`/api/positionings/${p.id}`, body) : api.post<PositioningSaved>('/api/positionings', body)),
    Object.keys(body),
  )
  if (saved) {
    notify(p ? 'Positionnement modifié.' : 'Ressource positionnée sur le besoin.')
    emit('saved', saved)
  }
}
const dot = (r: Suggestion['reasons'][number]) => (r.positive ? 'green' : 'red')
</script>

<template>
  <Modal :title="editing ? 'Modifier le positionnement' : picked ? 'Qualifier le positionnement' : 'Positionner une ressource'" size="wide" :busy="busy" @close="emit('close')">
    <!-- Étape 1 : choisir la ressource -->
    <div v-if="!picked" class="stack">
      <div class="tabs" role="tablist" style="align-self: flex-start">
        <button type="button" role="tab" :class="{ on: mode === 'suggest' }" @click="mode = 'suggest'"><Icon name="sparkle" style="width: 15px; vertical-align: -3px" /> Suggestions</button>
        <button type="button" role="tab" :class="{ on: mode === 'search' }" @click="mode = 'search'">Recherche filtrée</button>
        <button type="button" role="tab" :class="{ on: mode === 'direct' }" @click="mode = 'direct'">Sélection directe</button>
      </div>

      <div v-if="loading" class="stack tight"><div class="skeleton" style="height: 54px" /><div class="skeleton" style="height: 54px" /></div>

      <template v-else-if="mode === 'suggest'">
        <p class="muted small">Profils proposés d'après les compétences ({{ need.skills.join(', ') || 'titre du besoin' }}), la disponibilité au {{ fmtDate(need.start_date) || 'démarrage' }} et la localisation.</p>
        <EmptyState v-if="!suggestions.length" compact title="Aucun profil ne correspond">Ajoutez des compétences au besoin ou utilisez la recherche filtrée.</EmptyState>
        <button v-for="s in suggestions" :key="s.resource.id" type="button" class="pick" @click="picked = s.resource">
          <span class="avatar">{{ initials(`${s.resource.first_name} ${s.resource.last_name}`) }}</span>
          <span class="grow">
            <strong>{{ s.resource.first_name }} {{ s.resource.last_name }}</strong>
            <span class="muted"> · {{ s.resource.title ?? RESOURCE_KIND_LABEL[s.resource.kind] }}</span>
            <span class="why"><span v-for="(r, i) in s.reasons" :key="i" class="pill plain" :class="dot(r)">{{ r.text }}</span></span>
          </span>
          <span class="score" :title="'Correspondance : ' + s.score + ' %'"><b>{{ s.score }}</b><small>%</small></span>
        </button>
      </template>

      <template v-else>
        <div class="filters">
          <div class="search-box search"><Icon name="search" /><input v-model="q" class="input" :placeholder="mode === 'direct' ? 'Nom de la ressource' : 'Nom ou métier'" autofocus /></div>
          <template v-if="mode === 'search'">
            <input v-model="skill" class="input" style="width: 180px; border-radius: 999px" placeholder="Compétence" />
            <select v-model="availability" class="select"><option value="">Toute disponibilité</option><option value="now">Disponible maintenant</option></select>
          </template>
        </div>
        <EmptyState v-if="!filtered.length" compact title="Aucune ressource trouvée">Modifiez la recherche ou créez la fiche depuis la page Ressources.</EmptyState>
        <button v-for="r in filtered.slice(0, 40)" :key="r.id" type="button" class="pick" @click="picked = r">
          <span class="avatar">{{ initials(`${r.first_name} ${r.last_name}`) }}</span>
          <span class="grow">
            <strong>{{ r.first_name }} {{ r.last_name }}</strong><span class="muted"> · {{ r.title ?? RESOURCE_KIND_LABEL[r.kind] }}</span>
            <span class="tags" style="margin-top: 4px"><span v-for="s in r.skills.slice(0, 6)" :key="s" class="tag">{{ s }}</span></span>
          </span>
          <span class="small muted nowrap">{{ r.on_mission ? 'En mission' : r.availability_date ? 'Dispo ' + fmtDate(r.availability_date) : 'Disponible' }}</span>
        </button>
      </template>
    </div>

    <!-- Étape 2 : qualifier -->
    <form v-else id="pos-form" class="form-grid" @submit.prevent="save" novalidate>
      <div class="full row" style="gap: 12px">
        <span class="avatar">{{ initials(`${picked.first_name} ${picked.last_name}`) }}</span>
        <div class="grow"><strong>{{ picked.first_name }} {{ picked.last_name }}</strong><div class="muted small">{{ picked.title ?? RESOURCE_KIND_LABEL[picked.kind] }} · pour « {{ need.title }} »</div></div>
        <button v-if="!editing && !resource" class="btn small" type="button" @click="picked = null">Changer</button>
      </div>
      <Field label="État du positionnement" :error="errors.state_id" for="ps">
        <select id="ps" v-model="form.state_id" class="select"><option v-for="s in refsOf('positioning_state')" :key="s.id" :value="s.id">{{ s.label }}</option></select>
      </Field>
      <span />
      <Field label="Début prévu" :error="errors.start_date" for="pd"><input id="pd" v-model="form.start_date" class="input" type="date" /></Field>
      <Field label="Fin prévue" :error="errors.end_date" for="pe"><input id="pe" v-model="form.end_date" class="input" :class="{ invalid: errors.end_date }" type="date" /></Field>
      <Field label="Tarif journalier de vente (€)" :error="errors.daily_rate" hint="Facultatif." for="pr"><input id="pr" v-model="form.daily_rate" class="input" inputmode="decimal" /></Field>
      <Field label="Jours à vendre" :error="errors.days" hint="Facultatif." for="pj"><input id="pj" v-model="form.days" class="input" inputmode="decimal" /></Field>
      <Field v-if="showCost" label="CJM (€)" :error="errors.daily_cost" :hint="picked.effective_daily_cost != null ? `Proposé d’après ${picked.daily_cost_source === 'contract' ? 'le contrat RH' : 'la fiche'} : ${fmtMoney(picked.effective_daily_cost)}` : 'Facultatif : inconnu pour le moment, vous pourrez le compléter plus tard.'" for="pc">
        <input id="pc" v-model="form.daily_cost" class="input" inputmode="decimal" placeholder="Non renseigné" :disabled="!can('costs', 'update')" />
      </Field>
      <Field class="full" label="Notes" for="pn"><textarea id="pn" v-model="form.notes" class="textarea" /></Field>
      <div v-if="conflict" class="full alert warn" role="alert">{{ conflict.message }}</div>
      <p v-if="message" class="full alert error" role="alert">{{ message }}</p>
    </form>

    <template #footer>
      <button class="btn" type="button" :disabled="busy" @click="emit('close')">Annuler</button>
      <button v-if="picked" class="btn primary" form="pos-form" :disabled="busy">{{ busy ? 'Enregistrement…' : 'Enregistrer' }}</button>
    </template>
  </Modal>
</template>

<style scoped>
.pick { display: flex; align-items: center; gap: 12px; width: 100%; text-align: left; padding: 12px; background: #fff; border: 1px solid var(--line); border-radius: 14px; }
.pick:hover { border-color: var(--green-strong); background: #fbfdf6; }
.why { display: flex; gap: 6px; flex-wrap: wrap; margin-top: 6px; }
.why .pill { height: auto; padding: 3px 9px; white-space: normal; font-weight: 550; }
.score { display: flex; align-items: baseline; gap: 1px; padding: 6px 12px; border-radius: 999px; background: var(--green-soft); color: var(--green-ink); }
.score b { font-size: 18px; }
</style>
