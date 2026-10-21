<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { api } from '../lib/api'
import { ACTIVITY_LABEL, fmtDate, fmtDateTime, todayIso } from '../lib/format'
import { can, confirmAction, me, notify } from '../lib/store'
import type { Activity, ActivityKind, Area } from '../lib/types'
import ActivityForm from './ActivityForm.vue'
import EmptyState from './EmptyState.vue'
import Icon from './Icon.vue'

type Links = Partial<Record<'company_id' | 'contact_id' | 'need_id' | 'resource_id' | 'positioning_id' | 'project_id', number>>
const props = defineProps<{ filter: Links; links?: Links; title?: string; area: Area; showContext?: boolean }>()

const items = ref<Activity[]>([])
const loading = ref(true)
const editing = ref<{ activity?: Activity; kind?: ActivityKind } | null>(null)

async function load() {
  loading.value = true
  try {
    items.value = await api.get<Activity[]>('/api/activities', props.filter)
  } catch {
    items.value = []
  } finally {
    loading.value = false
  }
}
onMounted(load)
watch(() => props.filter, load, { deep: true })

const canAdd = computed(() => can(props.area, 'create') || can(props.area, 'update'))
const icon: Record<ActivityKind, string> = { call: 'phone', meeting: 'calendar', email: 'mail', follow_up: 'bell', note: 'note' }
const overdue = (a: Activity) => !!a.due_date && !a.done_at && a.due_date < todayIso()
const mine = (a: Activity) => a.owner?.id === me.value?.id
const editable = (a: Activity) => mine(a) || can(props.area, 'update')

async function toggle(a: Activity) {
  const updated = await api.post<Activity>(`/api/activities/${a.id}/done`, { done: !a.done_at })
  Object.assign(a, updated)
}

async function remove(a: Activity) {
  if (!(await confirmAction('Supprimer cette activité ?', 'Elle disparaîtra définitivement de l’historique des échanges.', 'Supprimer', true))) return
  await api.del(`/api/activities/${a.id}`)
  notify('Activité supprimée.')
  await load()
}

function saved() {
  editing.value = null
  load()
}
</script>

<template>
  <div class="card">
    <div class="card-head">
      <h2>{{ title ?? 'Échanges et rappels' }}</h2>
      <div v-if="canAdd" class="row">
        <button v-for="k in (['call', 'meeting', 'email', 'follow_up', 'note'] as ActivityKind[])" :key="k" class="btn small ghost icon" type="button" :title="`Ajouter : ${ACTIVITY_LABEL[k]}`" :aria-label="`Ajouter : ${ACTIVITY_LABEL[k]}`" @click="editing = { kind: k }">
          <Icon :name="icon[k]" />
        </button>
      </div>
    </div>
    <div v-if="loading" class="stack tight"><div class="skeleton" style="height: 18px" /><div class="skeleton" style="height: 18px; width: 60%" /></div>
    <EmptyState v-else-if="!items.length" compact title="Aucun échange enregistré">
      Notez un appel, un rendez-vous, un e-mail ou programmez une relance.
    </EmptyState>
    <div v-else class="list">
      <div v-for="a in items" :key="a.id" class="act" :class="{ done: a.done_at }">
        <button v-if="a.due_date" class="chk" type="button" :aria-label="a.done_at ? 'Rouvrir le rappel' : 'Marquer comme fait'" :title="a.done_at ? 'Rouvrir' : 'Marquer comme fait'" :disabled="!editable(a)" @click="toggle(a)">
          <Icon v-if="a.done_at" name="check" />
        </button>
        <span v-else class="kind"><Icon :name="icon[a.kind]" /></span>
        <div class="grow">
          <div class="row wrap" style="gap: 8px">
            <strong>{{ ACTIVITY_LABEL[a.kind] }}</strong>
            <span v-if="a.due_date" class="pill plain" :class="overdue(a) ? 'red' : a.done_at ? 'green' : 'orange'">
              {{ a.done_at ? 'Fait' : overdue(a) ? 'En retard' : 'À faire' }} · {{ fmtDate(a.due_date) }}
            </span>
            <span class="faint small">{{ fmtDateTime(a.occurred_at) }}<template v-if="a.owner"> · {{ a.owner.name }}</template></span>
          </div>
          <p v-if="a.comment" class="comment">{{ a.comment }}</p>
          <div v-if="showContext" class="small faint row wrap" style="gap: 10px; margin-top: 4px">
            <RouterLink v-if="a.contact" :to="`/crm/contacts/${a.contact.id}`" class="link">{{ a.contact.name }}</RouterLink>
            <RouterLink v-if="a.company" :to="`/crm/companies/${a.company.id}`" class="link">{{ a.company.name }}</RouterLink>
            <RouterLink v-if="a.need" :to="`/needs/${a.need.id}`" class="link">{{ a.need.name }}</RouterLink>
          </div>
        </div>
        <div v-if="editable(a)" class="row" style="gap: 2px">
          <button class="btn ghost small icon" type="button" aria-label="Modifier" @click="editing = { activity: a }"><Icon name="edit" /></button>
          <button class="btn ghost small icon" type="button" aria-label="Supprimer" @click="remove(a)"><Icon name="trash" /></button>
        </div>
      </div>
    </div>
    <ActivityForm v-if="editing" :activity="editing.activity" :kind="editing.kind" :links="links ?? filter" @close="editing = null" @saved="saved" />
  </div>
</template>

<style scoped>
.act { display: flex; gap: 12px; align-items: flex-start; }
.kind, .chk { width: 28px; height: 28px; border-radius: 50%; display: grid; place-items: center; flex: none; background: var(--green-soft); color: var(--green-ink); }
.kind svg { width: 15px; height: 15px; }
.chk { border: 2px solid var(--line-strong); background: #fff; padding: 0; }
.chk:hover:not(:disabled) { border-color: var(--green-strong); }
.chk svg { width: 14px; height: 14px; }
.done .chk { background: var(--green); border-color: var(--green); color: var(--black); }
.done .comment { text-decoration: line-through; color: var(--ink-3); }
.comment { margin-top: 4px; white-space: pre-wrap; overflow-wrap: anywhere; }
</style>
