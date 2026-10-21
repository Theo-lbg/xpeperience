<script setup lang="ts">
import { reactive } from 'vue'
import { api } from '../lib/api'
import { num, orNull, todayIso } from '../lib/format'
import { useForm } from '../lib/forms'
import { notify, refsOf } from '../lib/store'
import type { HrContract } from '../lib/types'
import Field from './Field.vue'
import Modal from './Modal.vue'

const props = defineProps<{ resourceId: number; contract?: HrContract }>()
const emit = defineEmits<{ close: []; saved: [] }>()
const { errors, message, busy, submit } = useForm()

const c = props.contract
const form = reactive({
  contract_type_id: c?.contract_type_id ?? (null as number | null), start_date: c?.start_date ?? todayIso(),
  end_date: c?.end_date ?? '', daily_cost: c?.daily_cost?.toString() ?? '', notes: c?.notes ?? '',
})

async function save() {
  const body = { ...form, end_date: orNull(form.end_date), daily_cost: num(form.daily_cost), notes: orNull(form.notes) }
  const ok = await submit(
    () => (c ? api.put(`/api/contracts/${c.id}`, body) : api.post(`/api/resources/${props.resourceId}/contracts`, body)),
    Object.keys(form),
  )
  if (ok) {
    notify(c ? 'Contrat RH modifié.' : 'Contrat RH ajouté.')
    emit('saved')
  }
}
</script>

<template>
  <Modal :title="contract ? 'Modifier le contrat RH' : 'Nouveau contrat RH'" size="narrow" :busy="busy" @close="emit('close')">
    <form id="contract-form" class="stack" @submit.prevent="save" novalidate>
      <Field label="Type de contrat" :error="errors.contract_type_id" for="kt">
        <select id="kt" v-model="form.contract_type_id" class="select"><option :value="null">—</option><option v-for="t in refsOf('contract_type')" :key="t.id" :value="t.id">{{ t.label }}</option></select>
      </Field>
      <div class="form-grid">
        <Field label="Début" required :error="errors.start_date" for="ks"><input id="ks" v-model="form.start_date" class="input" :class="{ invalid: errors.start_date }" type="date" /></Field>
        <Field label="Fin" :error="errors.end_date" hint="Vide = sans fin." for="ke"><input id="ke" v-model="form.end_date" class="input" :class="{ invalid: errors.end_date }" type="date" /></Field>
      </div>
      <Field label="Coût journalier moyen (€)" :error="errors.daily_cost" hint="Facultatif. Quand il est renseigné, il devient le CJM proposé pour les positionnements." for="kc">
        <input id="kc" v-model="form.daily_cost" class="input" inputmode="decimal" placeholder="Non renseigné" />
      </Field>
      <Field label="Notes" for="kn"><textarea id="kn" v-model="form.notes" class="textarea" style="min-height: 60px" /></Field>
      <p v-if="message" class="alert error" role="alert">{{ message }}</p>
    </form>
    <template #footer>
      <button class="btn" type="button" :disabled="busy" @click="emit('close')">Annuler</button>
      <button class="btn primary" form="contract-form" :disabled="busy">{{ busy ? 'Enregistrement…' : 'Enregistrer' }}</button>
    </template>
  </Modal>
</template>
