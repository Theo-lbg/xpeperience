<script setup lang="ts">
import { reactive } from 'vue'
import { api } from '../lib/api'
import { addDays, fmtDate, num, orNull } from '../lib/format'
import { useForm } from '../lib/forms'
import { notify } from '../lib/store'
import type { Delivery, Extension } from '../lib/types'
import Field from './Field.vue'
import Modal from './Modal.vue'

const props = defineProps<{ delivery: Delivery; extension?: Extension }>()
const emit = defineEmits<{ close: []; saved: [Delivery] }>()
const { errors, message, busy, submit } = useForm()

// Une prolongation commence le lendemain de la dernière fin connue.
function nextDay(iso: string | null): string {
  if (!iso) return ''
  const d = new Date(iso + 'T00:00:00')
  d.setDate(d.getDate() + 1)
  return d.toISOString().slice(0, 10)
}
const e = props.extension
const form = reactive({
  status: e?.status ?? 'signed',
  start_date: e?.start_date ?? nextDay(props.delivery.forecast_end_date ?? props.delivery.end_date),
  end_date: e?.end_date ?? '',
  days: e?.days?.toString() ?? '',
  daily_rate: e?.daily_rate?.toString() ?? '',
  notes: e?.notes ?? '',
})

async function save() {
  const body = { ...form, days: num(form.days), daily_rate: num(form.daily_rate), notes: orNull(form.notes) }
  const saved = await submit(
    () => (e ? api.put<Delivery>(`/api/extensions/${e.id}`, body) : api.post<Delivery>(`/api/deliveries/${props.delivery.id}/extensions`, body)),
    Object.keys(form),
  )
  if (saved) {
    notify(e ? 'Prolongation modifiée.' : 'Prestation prolongée.')
    emit('saved', saved)
  }
}
void addDays
</script>

<template>
  <Modal :title="extension ? 'Modifier la prolongation' : 'Prolonger la prestation'" size="narrow" :busy="busy" @close="emit('close')">
    <form id="ext-form" class="stack" @submit.prevent="save" novalidate>
      <p v-if="!delivery.end_date" class="alert warn">Cette prestation n'a pas de date de fin : renseignez-la avant de la prolonger.</p>
      <p v-else class="muted small">Fin actuelle de la prestation : <strong>{{ fmtDate(delivery.forecast_end_date ?? delivery.end_date) }}</strong>.</p>
      <Field label="Nature" for="xs">
        <select id="xs" v-model="form.status" class="select">
          <option value="signed">Signée — la fin du projet est repoussée</option>
          <option value="forecast">Prévisionnelle — seule la fin prévisionnelle change</option>
        </select>
      </Field>
      <div class="form-grid">
        <Field label="Début" required :error="errors.start_date" for="xd"><input id="xd" v-model="form.start_date" class="input" :class="{ invalid: errors.start_date }" type="date" /></Field>
        <Field label="Fin" required :error="errors.end_date" for="xe"><input id="xe" v-model="form.end_date" class="input" :class="{ invalid: errors.end_date }" type="date" /></Field>
        <Field label="Jours" :error="errors.days" for="xj"><input id="xj" v-model="form.days" class="input" inputmode="decimal" /></Field>
        <Field label="Tarif journalier (€)" :error="errors.daily_rate" hint="Vide = tarif de la prestation." for="xr"><input id="xr" v-model="form.daily_rate" class="input" inputmode="decimal" /></Field>
      </div>
      <Field label="Notes" for="xn"><textarea id="xn" v-model="form.notes" class="textarea" style="min-height: 60px" /></Field>
      <p v-if="message" class="alert error" role="alert">{{ message }}</p>
    </form>
    <template #footer>
      <button class="btn" type="button" :disabled="busy" @click="emit('close')">Annuler</button>
      <button class="btn primary" form="ext-form" :disabled="busy || !delivery.end_date">{{ busy ? 'Enregistrement…' : 'Enregistrer' }}</button>
    </template>
  </Modal>
</template>
