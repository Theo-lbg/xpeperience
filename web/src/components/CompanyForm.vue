<script setup lang="ts">
import { reactive } from 'vue'
import { api } from '../lib/api'
import { orNull } from '../lib/format'
import { useForm } from '../lib/forms'
import { isRequired, notify, refsOf, users } from '../lib/store'
import type { Company } from '../lib/types'
import Field from './Field.vue'
import Modal from './Modal.vue'

const props = defineProps<{ company?: Company; name?: string }>()
const emit = defineEmits<{ close: []; saved: [Company] }>()
const { errors, message, busy, conflict, submit } = useForm()

const c = props.company
const form = reactive({
  name: c?.name ?? props.name ?? '', type_id: c?.type_id ?? null, sector_id: c?.sector_id ?? null,
  website: c?.website ?? '', phone: c?.phone ?? '', email: c?.email ?? '', address: c?.address ?? '',
  postal_code: c?.postal_code ?? '', city: c?.city ?? '', country: c?.country ?? '', notes: c?.notes ?? '',
  owner_id: c?.owner?.id ?? null as number | null,
})
const req = (f: string) => isRequired('company', f)

async function save() {
  const body = { ...form, website: orNull(form.website), phone: orNull(form.phone), email: orNull(form.email), address: orNull(form.address), postal_code: orNull(form.postal_code), city: orNull(form.city), country: orNull(form.country), notes: orNull(form.notes) }
  const saved = await submit(() => (c ? api.put<Company>(`/api/companies/${c.id}`, body) : api.post<Company>('/api/companies', body)), Object.keys(form))
  if (saved) {
    notify(c ? 'Société modifiée.' : 'Société créée.')
    emit('saved', saved)
  }
}
</script>

<template>
  <Modal :title="company ? 'Modifier la société' : 'Nouvelle société'" :busy="busy" @close="emit('close')">
    <form id="company-form" class="form-grid" @submit.prevent="save" novalidate>
      <Field class="full" label="Nom de la société" required :error="errors.name" for="cn"><input id="cn" v-model="form.name" class="input" :class="{ invalid: errors.name }" autofocus /></Field>
      <Field label="Type" :required="req('type_id')" :error="errors.type_id" for="ct">
        <select id="ct" v-model="form.type_id" class="select"><option :value="null">—</option><option v-for="r in refsOf('company_type')" :key="r.id" :value="r.id">{{ r.label }}</option></select>
      </Field>
      <Field label="Secteur" :required="req('sector_id')" :error="errors.sector_id" for="cs">
        <select id="cs" v-model="form.sector_id" class="select"><option :value="null">—</option><option v-for="r in refsOf('sector')" :key="r.id" :value="r.id">{{ r.label }}</option></select>
      </Field>
      <Field label="Responsable commercial" :required="req('owner_id')" :error="errors.owner_id" for="co" hint="Vous, par défaut.">
        <select id="co" v-model="form.owner_id" class="select"><option :value="null">Moi</option><option v-for="u in users" :key="u.id" :value="u.id">{{ u.name }}</option></select>
      </Field>
      <Field label="Site web" :error="errors.website" for="cw"><input id="cw" v-model="form.website" class="input" placeholder="https://" /></Field>
      <Field label="Téléphone" :required="req('phone')" :error="errors.phone" for="cp"><input id="cp" v-model="form.phone" class="input" type="tel" /></Field>
      <Field label="E-mail" :required="req('email')" :error="errors.email" for="ce"><input id="ce" v-model="form.email" class="input" :class="{ invalid: errors.email }" type="email" /></Field>
      <Field class="full" label="Adresse" for="ca"><input id="ca" v-model="form.address" class="input" /></Field>
      <Field label="Code postal" for="cz"><input id="cz" v-model="form.postal_code" class="input" /></Field>
      <Field label="Ville" :required="req('city')" :error="errors.city" for="cv"><input id="cv" v-model="form.city" class="input" /></Field>
      <Field label="Pays" for="cy"><input id="cy" v-model="form.country" class="input" /></Field>
      <Field class="full" label="Notes" for="cno"><textarea id="cno" v-model="form.notes" class="textarea" /></Field>
      <div v-if="conflict" class="full alert warn" role="alert">
        <div>
          {{ conflict.message }}
          <RouterLink v-if="conflict.details?.id" :to="`/crm/companies/${conflict.details.id}`" class="link" @click="emit('close')">Ouvrir la fiche existante</RouterLink>
        </div>
      </div>
      <p v-if="message" class="full alert error" role="alert">{{ message }}</p>
    </form>
    <template #footer>
      <button class="btn" type="button" :disabled="busy" @click="emit('close')">Annuler</button>
      <button class="btn primary" form="company-form" :disabled="busy">{{ busy ? 'Enregistrement…' : 'Enregistrer' }}</button>
    </template>
  </Modal>
</template>
