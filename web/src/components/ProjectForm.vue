<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { api } from '../lib/api'
import { orNull } from '../lib/format'
import { useForm } from '../lib/forms'
import { firstRef, isRequired, notify, refCategory, refsOf, users } from '../lib/store'
import type { Company, Contact, Project } from '../lib/types'
import Field from './Field.vue'
import Modal from './Modal.vue'

const props = defineProps<{ project?: Project; companyId?: number }>()
const emit = defineEmits<{ close: []; saved: [Project] }>()
const { errors, message, busy, submit } = useForm()

const p = props.project
const form = reactive({
  title: p?.title ?? '', reference: p?.reference ?? '', company_id: p?.company?.id ?? props.companyId ?? (null as number | null),
  contact_id: p?.contact?.id ?? (null as number | null), type_id: p?.type_id ?? firstRef('project_type', 'regie'),
  state_id: p?.state_id ?? firstRef('project_state', 'planned'), manager_id: p?.manager?.id ?? (null as number | null),
  start_date: p?.start_date ?? '', end_date: p?.end_date ?? '', notes: p?.notes ?? '',
})
const companies = ref<Company[]>([])
const contacts = ref<Contact[]>([])
onMounted(async () => {
  ;[companies.value, contacts.value] = await Promise.all([api.get<Company[]>('/api/companies'), api.get<Contact[]>('/api/contacts')])
})
const companyContacts = computed(() => contacts.value.filter((c) => !form.company_id || c.company?.id === form.company_id))
const internal = computed(() => refCategory(form.type_id) === 'interne')
const datesLocked = !!p?.dates_from_deliveries
const req = (f: string) => isRequired('project', f)

async function save() {
  const body = { ...form, reference: orNull(form.reference), start_date: orNull(form.start_date), end_date: orNull(form.end_date), notes: orNull(form.notes) }
  const saved = await submit(() => (p ? api.put<Project>(`/api/projects/${p.id}`, body) : api.post<Project>('/api/projects', body)), Object.keys(form))
  if (saved) {
    notify(p ? 'Projet modifié.' : 'Projet créé.', 'success', p ? {} : { to: `/projects/${saved.id}`, action: 'Ouvrir' })
    emit('saved', saved)
  }
}
</script>

<template>
  <Modal :title="project ? 'Modifier le projet' : 'Nouveau projet'" size="wide" :busy="busy" @close="emit('close')">
    <form id="project-form" class="form-grid" @submit.prevent="save" novalidate>
      <Field class="full" label="Titre du projet" required :error="errors.title" for="pt"><input id="pt" v-model="form.title" class="input" :class="{ invalid: errors.title }" autofocus /></Field>
      <Field label="Type de projet" :required="req('type_id')" :error="errors.type_id" for="py">
        <select id="py" v-model="form.type_id" class="select"><option :value="null">—</option><option v-for="r in refsOf('project_type')" :key="r.id" :value="r.id">{{ r.label }}</option></select>
      </Field>
      <Field label="État" :error="errors.state_id" for="ps">
        <select id="ps" v-model="form.state_id" class="select"><option v-for="r in refsOf('project_state')" :key="r.id" :value="r.id">{{ r.label }}</option></select>
      </Field>
      <Field label="Société cliente" :required="!internal" :error="errors.company_id" :hint="internal ? 'Facultative pour un projet interne.' : undefined" for="pc">
        <select id="pc" v-model="form.company_id" class="select" :class="{ invalid: errors.company_id }" @change="form.contact_id = null"><option :value="null">— Choisir —</option><option v-for="c in companies" :key="c.id" :value="c.id">{{ c.name }}</option></select>
      </Field>
      <Field label="Contact" :required="req('contact_id')" :error="errors.contact_id" for="pk">
        <select id="pk" v-model="form.contact_id" class="select"><option :value="null">—</option><option v-for="c in companyContacts" :key="c.id" :value="c.id">{{ c.first_name ? `${c.first_name} ${c.last_name}` : c.last_name }}</option></select>
      </Field>
      <Field label="Responsable / chef de projet" :required="req('manager_id')" :error="errors.manager_id" for="pm">
        <select id="pm" v-model="form.manager_id" class="select"><option :value="null">Moi</option><option v-for="u in users" :key="u.id" :value="u.id">{{ u.name }}</option></select>
      </Field>
      <Field label="Référence" :required="req('reference')" :error="errors.reference" for="pr"><input id="pr" v-model="form.reference" class="input" /></Field>
      <template v-if="!datesLocked">
        <Field label="Date de début" :error="errors.start_date" for="pd"><input id="pd" v-model="form.start_date" class="input" type="date" /></Field>
        <Field label="Date de fin" :error="errors.end_date" for="pe"><input id="pe" v-model="form.end_date" class="input" :class="{ invalid: errors.end_date }" type="date" /></Field>
      </template>
      <p v-else class="full alert">Les dates du projet suivent ses prestations : modifiez ou prolongez les prestations pour les faire évoluer.</p>
      <Field class="full" label="Notes" for="pn"><textarea id="pn" v-model="form.notes" class="textarea" /></Field>
      <p v-if="message" class="full alert error" role="alert">{{ message }}</p>
    </form>
    <template #footer>
      <button class="btn" type="button" :disabled="busy" @click="emit('close')">Annuler</button>
      <button class="btn primary" form="project-form" :disabled="busy">{{ busy ? 'Enregistrement…' : 'Enregistrer' }}</button>
    </template>
  </Modal>
</template>
