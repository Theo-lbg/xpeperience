<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { api } from '../lib/api'
import { num, orNull } from '../lib/format'
import { useForm } from '../lib/forms'
import { firstRef, isRequired, notify, refsOf, users } from '../lib/store'
import type { Contact, Need, NeedDraft } from '../lib/types'
import Field from './Field.vue'
import Modal from './Modal.vue'
import TagInput from './TagInput.vue'

const props = defineProps<{ need?: Need; contactId?: number }>()
const emit = defineEmits<{ close: []; saved: [Need] }>()
const { errors, message, busy, submit } = useForm()

const n = props.need
const form = reactive({
  title: n?.title ?? '', contact_id: n?.contact?.id ?? props.contactId ?? (null as number | null), description: n?.description ?? '',
  skills: [...(n?.skills ?? [])], type_id: n?.type_id ?? (refsOf('need_type').length === 1 ? refsOf('need_type')[0].id : null as number | null), state_id: n?.state_id ?? firstRef('need_state', 'open'),
  origin_id: n?.origin_id ?? null as number | null, budget: n?.budget?.toString() ?? '', duration_months: n?.duration_months?.toString() ?? '',
  planned_days: n?.planned_days?.toString() ?? '', start_date: n?.start_date ?? '', location: n?.location ?? '', owner_id: n?.owner?.id ?? (null as number | null),
})
const contacts = ref<Contact[]>([])
onMounted(async () => {
  contacts.value = await api.get<Contact[]>('/api/contacts')
})
const company = computed(() => contacts.value.find((c) => c.id === form.contact_id)?.company?.name)
const req = (f: string) => isRequired('need', f)

const pasted = ref('')
const analysing = ref(false)
async function analyse() {
  if (!pasted.value.trim()) return
  analysing.value = true
  try {
    const d = await api.post<NeedDraft>('/api/needs/draft', { text: pasted.value })
    // On complète sans écraser ce qui est déjà saisi.
    if (!form.title) form.title = d.title
    if (!form.description) form.description = d.description
    form.skills = [...new Set([...form.skills, ...d.skills])]
    if (!form.location && d.location) form.location = d.location
    if (!form.start_date && d.start_date) form.start_date = d.start_date
    if (!form.duration_months && d.duration_months != null) form.duration_months = d.duration_months.toString()
    if (!form.planned_days && d.planned_days != null) form.planned_days = d.planned_days.toString()
    if (!form.contact_id && d.contact) form.contact_id = d.contact.id
    notify(d.contact ? 'Champs préremplis. Vérifiez-les avant d’enregistrer.' : 'Champs préremplis. Choisissez le contact client et vérifiez le reste.', 'info')
    pasted.value = ''
  } catch (e) {
    notify(e instanceof Error ? e.message : 'Analyse impossible.', 'error')
  } finally {
    analysing.value = false
  }
}
async function loadFile(ev: Event) {
  const input = ev.target as HTMLInputElement
  const file = input.files?.[0]
  if (file) pasted.value = await file.text()
  input.value = ''
}

async function save() {
  const body = {
    ...form, description: orNull(form.description), budget: num(form.budget), duration_months: num(form.duration_months),
    planned_days: num(form.planned_days), start_date: orNull(form.start_date), location: orNull(form.location),
  }
  const saved = await submit(() => (n ? api.put<Need>(`/api/needs/${n.id}`, body) : api.post<Need>('/api/needs', body)), Object.keys(form))
  if (saved) {
    notify(n ? 'Besoin modifié.' : 'Besoin créé.', 'success', { to: `/needs/${saved.id}`, action: 'Ouvrir' })
    emit('saved', saved)
  }
}
</script>

<template>
  <Modal :title="need ? 'Modifier le besoin' : 'Nouveau besoin'" size="wide" :busy="busy" @close="emit('close')">
    <form id="need-form" class="form-grid" @submit.prevent="save" novalidate>
      <details v-if="!need" class="full" open>
        <summary>Remplir depuis une annonce ou un e-mail</summary>
        <div class="stack" style="margin-top: 8px">
          <textarea v-model="pasted" class="textarea" rows="5" aria-label="Texte de l’annonce" placeholder="Collez ici la description du client : le titre, les compétences, le lieu, la durée et la date de démarrage sont repérés pour vous." />
          <div class="row">
            <button class="btn" type="button" :disabled="analysing || !pasted.trim()" @click="analyse">{{ analysing ? 'Analyse…' : 'Remplir le formulaire' }}</button>
            <label class="btn ghost" style="cursor: pointer">Charger un fichier texte (.txt, .eml, .md)<input type="file" accept=".txt,.eml,.md,text/plain,message/rfc822" style="display: none" @change="loadFile" /></label>
          </div>
          <p class="faint small">L’analyse se fait sur votre machine ou votre serveur, sans service externe. Les PDF et Word ne sont pas lus : copiez-en le texte.</p>
        </div>
      </details>
      <Field class="full" label="Titre du besoin" required :error="errors.title" for="nt"><input id="nt" v-model="form.title" class="input" :class="{ invalid: errors.title }" placeholder="Ex. Développeur Rust confirmé" autofocus /></Field>
      <Field label="Contact client" required :error="errors.contact_id" :hint="company ? `Société : ${company}` : 'La société est déduite du contact.'" for="nc">
        <select id="nc" v-model="form.contact_id" class="select" :class="{ invalid: errors.contact_id }">
          <option :value="null">— Choisir un contact —</option>
          <option v-for="c in contacts" :key="c.id" :value="c.id">{{ c.first_name ? `${c.first_name} ${c.last_name}` : c.last_name }}{{ c.company ? ` — ${c.company.name}` : '' }}</option>
        </select>
      </Field>
      <Field label="État" :error="errors.state_id" for="ns">
        <select id="ns" v-model="form.state_id" class="select"><option v-for="r in refsOf('need_state')" :key="r.id" :value="r.id">{{ r.label }}</option></select>
      </Field>
      <Field v-if="refsOf('need_type').length > 1" label="Type" :required="req('type_id')" :error="errors.type_id" for="ny">
        <select id="ny" v-model="form.type_id" class="select"><option :value="null">—</option><option v-for="r in refsOf('need_type')" :key="r.id" :value="r.id">{{ r.label }}</option></select>
      </Field>
      <Field label="Provenance" :required="req('origin_id')" :error="errors.origin_id" for="no">
        <select id="no" v-model="form.origin_id" class="select"><option :value="null">—</option><option v-for="r in refsOf('need_origin')" :key="r.id" :value="r.id">{{ r.label }}</option></select>
      </Field>
      <Field class="full" label="Description" :required="req('description')" :error="errors.description" for="nd"><textarea id="nd" v-model="form.description" class="textarea" /></Field>
      <Field class="full" label="Compétences recherchées" :required="req('skills')" :error="errors.skills" hint="Elles servent à suggérer des profils. Entrée ou virgule pour valider.">
        <TagInput v-model="form.skills" placeholder="Rust, Vue.js, SQL…" />
      </Field>
      <Field label="Date de démarrage souhaitée" :required="req('start_date')" :error="errors.start_date" for="nsd"><input id="nsd" v-model="form.start_date" class="input" type="date" /></Field>
      <Field label="Durée (mois)" :required="req('duration_months')" :error="errors.duration_months" for="nm"><input id="nm" v-model="form.duration_months" class="input" inputmode="decimal" /></Field>
      <Field label="Nombre de jours prévus" :required="req('planned_days')" :error="errors.planned_days" for="nj"><input id="nj" v-model="form.planned_days" class="input" inputmode="decimal" /></Field>
      <Field label="Lieu de la mission" for="nl"><input id="nl" v-model="form.location" class="input" placeholder="Ville, télétravail…" /></Field>
      <Field label="Responsable commercial" :required="req('owner_id')" :error="errors.owner_id" for="nr">
        <select id="nr" v-model="form.owner_id" class="select"><option :value="null">Moi</option><option v-for="u in users" :key="u.id" :value="u.id">{{ u.name }}</option></select>
      </Field>
      <p v-if="message" class="full alert error" role="alert">{{ message }}</p>
    </form>
    <template #footer>
      <button class="btn" type="button" :disabled="busy" @click="emit('close')">Annuler</button>
      <button class="btn primary" form="need-form" :disabled="busy">{{ busy ? 'Enregistrement…' : 'Enregistrer' }}</button>
    </template>
  </Modal>
</template>
