<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { api } from '../lib/api'
import { fmtMoney, num, orNull } from '../lib/format'
import { useForm } from '../lib/forms'
import { can, isRequired, notify, refsOf } from '../lib/store'
import type { Delivery, Resource } from '../lib/types'
import Field from './Field.vue'
import Modal from './Modal.vue'

const props = defineProps<{ projectId: number; delivery?: Delivery }>()
const emit = defineEmits<{ close: []; saved: [Delivery] }>()
const { errors, message, busy, submit } = useForm()

const d = props.delivery
const form = reactive({
  resource_id: d?.resource?.id ?? (null as number | null), type_id: d?.type_id ?? (null as number | null), title: d?.title ?? '',
  start_date: d?.start_date ?? '', end_date: d?.end_date ?? '', daily_rate: d?.daily_rate?.toString() ?? '',
  sold_days: d?.sold_days?.toString() ?? '', daily_cost: d?.daily_cost?.toString() ?? '', notes: d?.notes ?? '',
})
const resources = ref<Resource[]>([])
onMounted(async () => {
  if (can('resources')) resources.value = await api.get<Resource[]>('/api/resources')
})
const req = (f: string) => isRequired('delivery', f)
const showCost = can('costs', 'read')
const known = () => resources.value.find((r) => r.id === form.resource_id)?.effective_daily_cost

async function save() {
  const body = {
    ...form, title: orNull(form.title), start_date: orNull(form.start_date), end_date: orNull(form.end_date), daily_rate: num(form.daily_rate),
    sold_days: num(form.sold_days), daily_cost: num(form.daily_cost), notes: orNull(form.notes),
  }
  const saved = await submit(
    () => (d ? api.put<Delivery>(`/api/deliveries/${d.id}`, body) : api.post<Delivery>(`/api/projects/${props.projectId}/deliveries`, body)),
    Object.keys(form),
  )
  if (saved) {
    notify(d ? 'Prestation modifiée.' : 'Prestation ajoutée.')
    emit('saved', saved)
  }
}
</script>

<template>
  <Modal :title="delivery ? 'Modifier la prestation' : 'Nouvelle prestation'" size="wide" :busy="busy" @close="emit('close')">
    <form id="delivery-form" class="form-grid" @submit.prevent="save" novalidate>
      <Field label="Ressource affectée" :required="req('resource_id')" :error="errors.resource_id" for="dr">
        <select id="dr" v-model="form.resource_id" class="select"><option :value="null">— À affecter —</option><option v-for="r in resources" :key="r.id" :value="r.id">{{ r.first_name }} {{ r.last_name }}</option></select>
      </Field>
      <Field label="Type de prestation" :error="errors.type_id" for="dt">
        <select id="dt" v-model="form.type_id" class="select"><option :value="null">—</option><option v-for="t in refsOf('delivery_type')" :key="t.id" :value="t.id">{{ t.label }}</option></select>
      </Field>
      <Field class="full" label="Intitulé" for="di"><input id="di" v-model="form.title" class="input" placeholder="Facultatif" /></Field>
      <Field label="Début" required :error="errors.start_date" for="ds"><input id="ds" v-model="form.start_date" class="input" :class="{ invalid: errors.start_date }" type="date" /></Field>
      <Field label="Fin" :required="req('end_date')" :error="errors.end_date" :hint="delivery?.extensions.length ? 'Fin initiale : les prolongations se gèrent à part.' : 'Vide = prestation sans fin définie.'" for="de">
        <input id="de" v-model="form.end_date" class="input" :class="{ invalid: errors.end_date }" type="date" />
      </Field>
      <Field label="Tarif journalier de vente (€)" :required="req('daily_rate')" :error="errors.daily_rate" for="dtj"><input id="dtj" v-model="form.daily_rate" class="input" inputmode="decimal" /></Field>
      <Field label="Jours vendus" :required="req('sold_days')" :error="errors.sold_days" for="dj"><input id="dj" v-model="form.sold_days" class="input" inputmode="decimal" /></Field>
      <Field v-if="showCost" label="CJM (€)" :error="errors.daily_cost" :hint="known() != null && !form.daily_cost ? `Sera repris de la ressource : ${fmtMoney(known())}` : 'Facultatif : peut rester vide et être complété plus tard.'" for="dc">
        <input id="dc" v-model="form.daily_cost" class="input" inputmode="decimal" placeholder="Non renseigné" :disabled="!can('costs', 'update')" />
      </Field>
      <Field class="full" label="Notes de suivi" for="dn"><textarea id="dn" v-model="form.notes" class="textarea" /></Field>
      <p v-if="message" class="full alert error" role="alert">{{ message }}</p>
    </form>
    <template #footer>
      <button class="btn" type="button" :disabled="busy" @click="emit('close')">Annuler</button>
      <button class="btn primary" form="delivery-form" :disabled="busy">{{ busy ? 'Enregistrement…' : 'Enregistrer' }}</button>
    </template>
  </Modal>
</template>
