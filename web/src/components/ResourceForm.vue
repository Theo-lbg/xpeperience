<script setup lang="ts">
import { reactive } from 'vue'
import { api } from '../lib/api'
import { fmtMoney, num, orNull } from '../lib/format'
import { useForm } from '../lib/forms'
import { can, isRequired, notify, refsOf } from '../lib/store'
import type { Resource, ResourceKind } from '../lib/types'
import Field from './Field.vue'
import Modal from './Modal.vue'
import TagInput from './TagInput.vue'

const props = defineProps<{ resource?: Resource; kind?: ResourceKind }>()
const emit = defineEmits<{ close: []; saved: [Resource] }>()
const { errors, message, busy, conflict, submit } = useForm()

const r = props.resource
const form = reactive({
  kind: (r?.kind ?? props.kind ?? 'candidate') as ResourceKind, first_name: r?.first_name ?? '', last_name: r?.last_name ?? '',
  title: r?.title ?? '', email: r?.email ?? '', phone: r?.phone ?? '', location: r?.location ?? '', linkedin_url: r?.linkedin_url ?? '',
  skills: [...(r?.skills ?? [])], experience_years: r?.experience_years?.toString() ?? '', experience: r?.experience ?? '',
  availability_date: r?.availability_date ?? '', availability_note: r?.availability_note ?? '', contract_type_id: r?.contract_type_id ?? (null as number | null),
  daily_cost: r?.daily_cost?.toString() ?? '', notes: r?.notes ?? '',
})
const req = (f: string) => isRequired('resource', f)
const showCost = can('costs', 'read')
const editCost = can('costs', 'update')

async function save() {
  const body = {
    ...form, title: orNull(form.title), email: orNull(form.email), phone: orNull(form.phone), location: orNull(form.location),
    linkedin_url: orNull(form.linkedin_url), experience_years: num(form.experience_years), experience: orNull(form.experience),
    availability_date: orNull(form.availability_date), availability_note: orNull(form.availability_note),
    daily_cost: num(form.daily_cost), notes: orNull(form.notes),
  }
  const saved = await submit(() => (r ? api.put<Resource>(`/api/resources/${r.id}`, body) : api.post<Resource>('/api/resources', body)), Object.keys(form))
  if (saved) {
    notify(r ? 'Fiche modifiée.' : 'Ressource créée.', 'success', r ? {} : { to: `/resources/${saved.id}`, action: 'Ouvrir' })
    emit('saved', saved)
  }
}
</script>

<template>
  <Modal :title="resource ? 'Modifier la fiche' : 'Nouveau profil'" size="wide" :busy="busy" @close="emit('close')">
    <form id="resource-form" class="form-grid" @submit.prevent="save" novalidate>
      <Field class="full" label="Statut" for="rk">
        <div class="tabs" role="radiogroup" style="align-self: flex-start">
          <button type="button" role="radio" :aria-checked="form.kind === 'candidate'" :class="{ on: form.kind === 'candidate' }" @click="form.kind = 'candidate'">Candidat</button>
          <button type="button" role="radio" :aria-checked="form.kind === 'consultant'" :class="{ on: form.kind === 'consultant' }" @click="form.kind = 'consultant'">Consultant</button>
        </div>
      </Field>
      <Field label="Prénom" required :error="errors.first_name" for="rf"><input id="rf" v-model="form.first_name" class="input" :class="{ invalid: errors.first_name }" autofocus /></Field>
      <Field label="Nom" required :error="errors.last_name" for="rl"><input id="rl" v-model="form.last_name" class="input" :class="{ invalid: errors.last_name }" /></Field>
      <Field label="Métier" :required="req('title')" :error="errors.title" for="rt"><input id="rt" v-model="form.title" class="input" placeholder="Développeur back-end…" /></Field>
      <Field label="Localisation" :required="req('location')" :error="errors.location" for="rloc"><input id="rloc" v-model="form.location" class="input" /></Field>
      <Field label="E-mail professionnel" :required="req('email')" :error="errors.email" for="re"><input id="re" v-model="form.email" class="input" :class="{ invalid: errors.email }" type="email" /></Field>
      <Field label="Téléphone" :required="req('phone')" :error="errors.phone" for="rp"><input id="rp" v-model="form.phone" class="input" type="tel" /></Field>
      <Field class="full" label="Compétences" :required="req('skills')" :error="errors.skills" hint="Entrée ou virgule pour valider chaque compétence.">
        <TagInput v-model="form.skills" placeholder="Rust, Vue.js, PostgreSQL…" />
      </Field>
      <Field label="Années d'expérience" :error="errors.experience_years" for="rx"><input id="rx" v-model="form.experience_years" class="input" inputmode="decimal" /></Field>
      <Field label="Profil LinkedIn" :error="errors.linkedin_url" for="rli"><input id="rli" v-model="form.linkedin_url" class="input" :class="{ invalid: errors.linkedin_url }" placeholder="https://www.linkedin.com/in/…" /></Field>
      <Field class="full" label="Expérience et parcours" for="rex"><textarea id="rex" v-model="form.experience" class="textarea" /></Field>
      <Field label="Disponible à partir du" :required="req('availability_date')" :error="errors.availability_date" hint="Vide = disponible." for="rd"><input id="rd" v-model="form.availability_date" class="input" type="date" /></Field>
      <Field label="Précision" for="rn"><input id="rn" v-model="form.availability_note" class="input" placeholder="Préavis, mi-temps…" /></Field>
      <Field label="Type de contrat" :required="req('contract_type_id')" :error="errors.contract_type_id" for="rc">
        <select id="rc" v-model="form.contract_type_id" class="select"><option :value="null">—</option><option v-for="c in refsOf('contract_type')" :key="c.id" :value="c.id">{{ c.label }}</option></select>
      </Field>
      <Field v-if="showCost" label="Coût journalier moyen (CJM)" :error="errors.daily_cost" :hint="editCost ? 'Facultatif : laissez vide s’il n’est pas connu. Un contrat RH en cours prime sur cette valeur.' : `CJM saisi : ${fmtMoney(r?.daily_cost)}`" for="rcj">
        <input id="rcj" v-model="form.daily_cost" class="input" inputmode="decimal" :disabled="!editCost" placeholder="Non renseigné" />
      </Field>
      <Field class="full" label="Notes internes" for="rno"><textarea id="rno" v-model="form.notes" class="textarea" /></Field>
      <div v-if="conflict" class="full alert warn" role="alert">
        <div>{{ conflict.message }} <RouterLink v-if="conflict.details?.id" :to="`/resources/${conflict.details.id}`" class="link" @click="emit('close')">Ouvrir la fiche</RouterLink></div>
      </div>
      <p v-if="message" class="full alert error" role="alert">{{ message }}</p>
    </form>
    <template #footer>
      <button class="btn" type="button" :disabled="busy" @click="emit('close')">Annuler</button>
      <button class="btn primary" form="resource-form" :disabled="busy">{{ busy ? 'Enregistrement…' : 'Enregistrer' }}</button>
    </template>
  </Modal>
</template>
