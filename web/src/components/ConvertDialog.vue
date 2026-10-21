<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api } from '../lib/api'
import { useForm } from '../lib/forms'
import { can, notify } from '../lib/store'
import type { Named, Positioning, Project } from '../lib/types'
import Field from './Field.vue'
import Modal from './Modal.vue'

const props = defineProps<{ positioning: Positioning }>()
const emit = defineEmits<{ close: []; done: [Project] }>()
const { message, busy, submit } = useForm()

const mode = ref<'new' | 'existing'>('new')
const projectId = ref<number | null>(null)
const open = ref<Named[]>([])

onMounted(async () => {
  if (props.positioning.company && can('projects', 'read')) {
    open.value = await api.get<Named[]>(`/api/companies/${props.positioning.company.id}/open-projects`)
  }
})

async function convert() {
  const project = await submit(() =>
    api.post<Project>(`/api/positionings/${props.positioning.id}/convert`, { mode: mode.value, project_id: mode.value === 'existing' ? projectId.value : null }),
  )
  if (project) {
    notify('Projet prêt : les informations du positionnement ont été reprises.', 'success', { to: `/projects/${project.id}`, action: 'Voir le projet' })
    emit('done', project)
  }
}
</script>

<template>
  <Modal title="Créer le projet" size="narrow" :busy="busy" @close="emit('close')">
    <div class="stack">
      <p class="muted">
        <strong>{{ positioning.resource.name }}</strong> est positionné(e) sur « {{ positioning.need.name }} ». Les dates, le tarif, les jours vendus et le CJM éventuel sont repris dans une prestation : rien à ressaisir.
      </p>
      <label class="check"><input v-model="mode" type="radio" value="new" /><span><strong>Nouveau projet</strong><br /><span class="muted small">Créé à partir du besoin (client, contact, dates).</span></span></label>
      <label class="check" :style="{ opacity: open.length ? 1 : 0.5 }">
        <input v-model="mode" type="radio" value="existing" :disabled="!open.length" />
        <span><strong>Ajouter à un projet existant</strong><br /><span class="muted small">{{ open.length ? 'Une nouvelle prestation sera ajoutée.' : 'Aucun projet en cours pour cette société.' }}</span></span>
      </label>
      <Field v-if="mode === 'existing'" label="Projet" for="cp">
        <select id="cp" v-model="projectId" class="select"><option :value="null">— Choisir —</option><option v-for="p in open" :key="p.id" :value="p.id">{{ p.name }}</option></select>
      </Field>
      <p v-if="message" class="alert error" role="alert">{{ message }}</p>
    </div>
    <template #footer>
      <button class="btn" type="button" :disabled="busy" @click="emit('close')">Plus tard</button>
      <button class="btn primary" type="button" :disabled="busy || (mode === 'existing' && !projectId)" @click="convert">{{ busy ? 'Création…' : 'Créer' }}</button>
    </template>
  </Modal>
</template>
