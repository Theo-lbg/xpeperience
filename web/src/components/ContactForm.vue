<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { api } from '../lib/api'
import { orNull } from '../lib/format'
import { useForm } from '../lib/forms'
import { isRequired, notify, users } from '../lib/store'
import type { Company, Contact, DuplicateContact } from '../lib/types'
import Field from './Field.vue'
import Modal from './Modal.vue'

const props = defineProps<{ contact?: Contact; companyId?: number }>()
const emit = defineEmits<{ close: []; saved: [Contact] }>()
const { errors, message, busy, conflict, submit } = useForm()

const k = props.contact
const form = reactive({
  first_name: k?.first_name ?? '', last_name: k?.last_name ?? '', company_id: k?.company?.id ?? props.companyId ?? (null as number | null),
  job_title: k?.job_title ?? '', email: k?.email ?? '', phone: k?.phone ?? '', mobile: k?.mobile ?? '',
  linkedin_url: k?.linkedin_url ?? '', location: k?.location ?? '', notes: k?.notes ?? '', owner_id: k?.owner?.id ?? (null as number | null),
})
const companies = ref<Company[]>([])
onMounted(async () => {
  companies.value = await api.get<Company[]>('/api/companies')
})
const req = (f: string) => isRequired('contact', f)

async function save(force = false) {
  const body = {
    ...form, force, first_name: orNull(form.first_name), job_title: orNull(form.job_title), email: orNull(form.email),
    phone: orNull(form.phone), mobile: orNull(form.mobile), linkedin_url: orNull(form.linkedin_url),
    location: orNull(form.location), notes: orNull(form.notes),
  }
  const saved = await submit(() => (k ? api.put<Contact>(`/api/contacts/${k.id}`, body) : api.post<Contact>('/api/contacts', body)), Object.keys(form))
  if (saved) {
    notify(k ? 'Contact modifié.' : 'Contact créé.')
    emit('saved', saved)
  }
}
const duplicates = (): DuplicateContact[] => conflict.value?.details?.duplicates ?? []
</script>

<template>
  <Modal :title="contact ? 'Modifier le contact' : 'Nouveau contact'" :busy="busy" @close="emit('close')">
    <form id="contact-form" class="form-grid" @submit.prevent="save(false)" novalidate>
      <Field label="Prénom" :required="req('first_name')" :error="errors.first_name" for="kf"><input id="kf" v-model="form.first_name" class="input" autofocus /></Field>
      <Field label="Nom" required :error="errors.last_name" for="kl"><input id="kl" v-model="form.last_name" class="input" :class="{ invalid: errors.last_name }" /></Field>
      <Field label="Société" :required="req('company_id')" :error="errors.company_id" for="kc">
        <select id="kc" v-model="form.company_id" class="select"><option :value="null">— Aucune —</option><option v-for="c in companies" :key="c.id" :value="c.id">{{ c.name }}</option></select>
      </Field>
      <Field label="Poste" :required="req('job_title')" :error="errors.job_title" for="kj"><input id="kj" v-model="form.job_title" class="input" /></Field>
      <Field label="E-mail professionnel" :required="req('email')" :error="errors.email" for="ke"><input id="ke" v-model="form.email" class="input" :class="{ invalid: errors.email }" type="email" /></Field>
      <Field label="Téléphone" :required="req('phone')" :error="errors.phone" for="kp"><input id="kp" v-model="form.phone" class="input" type="tel" /></Field>
      <Field label="Mobile" for="km"><input id="km" v-model="form.mobile" class="input" type="tel" /></Field>
      <Field label="Localisation" for="kv"><input id="kv" v-model="form.location" class="input" placeholder="Ville, région…" /></Field>
      <Field label="Responsable commercial" :required="req('owner_id')" :error="errors.owner_id" for="ko">
        <select id="ko" v-model="form.owner_id" class="select"><option :value="null">Moi</option><option v-for="u in users" :key="u.id" :value="u.id">{{ u.name }}</option></select>
      </Field>
      <Field label="Profil LinkedIn" :error="errors.linkedin_url" for="kli"><input id="kli" v-model="form.linkedin_url" class="input" :class="{ invalid: errors.linkedin_url }" placeholder="https://www.linkedin.com/in/…" /></Field>
      <Field class="full" label="Notes" for="kn"><textarea id="kn" v-model="form.notes" class="textarea" /></Field>

      <div v-if="duplicates().length" class="full alert warn" role="alert">
        <div>
          <strong>Ce contact existe peut-être déjà.</strong>
          <ul>
            <li v-for="d in duplicates()" :key="d.id">
              <RouterLink :to="`/crm/contacts/${d.id}`" class="link" @click="emit('close')">{{ d.name }}</RouterLink>
              <span v-if="d.job_title || d.company"> — {{ [d.job_title, d.company].filter(Boolean).join(' · ') }}</span>
              <span class="faint"> ({{ d.reason }})</span>
            </li>
          </ul>
        </div>
      </div>
      <p v-if="message" class="full alert error" role="alert">{{ message }}</p>
    </form>
    <template #footer>
      <button class="btn" type="button" :disabled="busy" @click="emit('close')">Annuler</button>
      <button v-if="duplicates().length" class="btn dark" type="button" :disabled="busy" @click="save(true)">Créer quand même</button>
      <button v-else class="btn primary" form="contact-form" :disabled="busy">{{ busy ? 'Enregistrement…' : 'Enregistrer' }}</button>
    </template>
  </Modal>
</template>
