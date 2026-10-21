<script setup lang="ts">
import { reactive, ref } from 'vue'
import { ApiError, api } from '../lib/api'
import { ACTIVITY_LABEL, addDays, orNull } from '../lib/format'
import { notify, settings, users } from '../lib/store'
import type { Activity, ActivityKind } from '../lib/types'
import Field from './Field.vue'
import Modal from './Modal.vue'

const props = defineProps<{
  activity?: Activity
  kind?: ActivityKind
  links: Partial<Record<'company_id' | 'contact_id' | 'need_id' | 'resource_id' | 'positioning_id' | 'project_id', number>>
}>()
const emit = defineEmits<{ close: []; saved: [Activity] }>()

const kind0 = props.activity?.kind ?? props.kind ?? 'note'
const form = reactive({
  kind: kind0 as ActivityKind,
  comment: props.activity?.comment ?? '',
  due_date: props.activity?.due_date ?? (kind0 === 'follow_up' ? addDays(settings.value.reminders.default_follow_up_days) : ''),
  owner_id: props.activity?.owner?.id ?? (null as number | null),
})
const errors = ref<Record<string, string>>({})
const message = ref('')
const busy = ref(false)

async function save() {
  busy.value = true
  errors.value = {}
  message.value = ''
  const body = { kind: form.kind, comment: form.comment, due_date: orNull(form.due_date), owner_id: form.owner_id, ...props.links }
  try {
    const saved = props.activity
      ? await api.put<Activity>(`/api/activities/${props.activity.id}`, body)
      : await api.post<Activity>('/api/activities', body)
    notify(props.activity ? 'Activité modifiée.' : 'Activité enregistrée.')
    emit('saved', saved)
  } catch (e) {
    if (e instanceof ApiError) {
      errors.value = e.fieldMap
      if (!e.fields.length || e.fields.some((f) => !(f.field in form))) message.value = e.message
    } else message.value = 'Enregistrement impossible.'
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <Modal :title="activity ? 'Modifier l’activité' : 'Nouvelle activité'" size="narrow" :busy="busy" @close="emit('close')">
    <form id="activity-form" class="stack" @submit.prevent="save" novalidate>
      <Field label="Type" for="ak">
        <select id="ak" v-model="form.kind" class="select">
          <option v-for="(label, k) in ACTIVITY_LABEL" :key="k" :value="k">{{ label }}</option>
        </select>
      </Field>
      <Field :label="form.kind === 'note' ? 'Note' : 'Commentaire'" :error="errors.comment" for="ac">
        <textarea id="ac" v-model="form.comment" class="textarea" :class="{ invalid: errors.comment }" autofocus />
      </Field>
      <div class="form-grid">
        <Field label="Échéance (rappel)" :error="errors.due_date" hint="Laissez vide s'il n'y a rien à faire ensuite." for="ad">
          <input id="ad" v-model="form.due_date" class="input" type="date" />
        </Field>
        <Field label="Responsable" :error="errors.owner_id" for="ao">
          <select id="ao" v-model="form.owner_id" class="select">
            <option :value="null">Moi</option>
            <option v-for="u in users" :key="u.id" :value="u.id">{{ u.name }}</option>
          </select>
        </Field>
      </div>
      <p v-if="message" class="alert error" role="alert">{{ message }}</p>
    </form>
    <template #footer>
      <button class="btn" type="button" :disabled="busy" @click="emit('close')">Annuler</button>
      <button class="btn primary" form="activity-form" :disabled="busy">{{ busy ? 'Enregistrement…' : 'Enregistrer' }}</button>
    </template>
  </Modal>
</template>
