<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { api } from '../lib/api'
import { actionLabel, entityLabel, fmtAgo, fmtDateTime, historyRoute } from '../lib/format'
import type { EntityKind, HistoryEntry } from '../lib/types'
import EmptyState from './EmptyState.vue'
import Icon from './Icon.vue'

const props = defineProps<{ entity: EntityKind; id: number; refreshKey?: unknown }>()
const entries = ref<HistoryEntry[]>([])
const loading = ref(true)

async function load() {
  loading.value = true
  try {
    entries.value = await api.get<HistoryEntry[]>(`/api/history/${props.entity}/${props.id}`)
  } catch {
    entries.value = []
  } finally {
    loading.value = false
  }
}
onMounted(load)
watch(() => [props.id, props.refreshKey], load)

const icon = (a: string) => ({ created: 'plus', imported: 'linkedin', state_changed: 'flag', archived: 'archive', restored: 'restore', converted: 'folder' })[a] ?? 'edit'
const own = (h: HistoryEntry) => h.entity === props.entity && h.entity_id === props.id
</script>

<template>
  <div class="card">
    <div class="card-head"><h2>Historique</h2></div>
    <div v-if="loading" class="stack tight"><div class="skeleton" style="height: 18px" /><div class="skeleton" style="height: 18px; width: 70%" /></div>
    <EmptyState v-else-if="!entries.length" compact title="Aucune modification enregistrée">Les créations, modifications et changements d'état apparaîtront ici.</EmptyState>
    <div v-else class="timeline">
      <div v-for="h in entries" :key="h.id" class="tl-item">
        <span class="tl-dot"><Icon :name="icon(h.action)" /></span>
        <div>
          <div>
            <RouterLink v-if="!own(h) && historyRoute(h)" :to="historyRoute(h)!" class="link">{{ entityLabel(h.entity) }} · {{ h.entity_label }}</RouterLink>
            <strong v-else-if="!own(h)">{{ entityLabel(h.entity) }} · {{ h.entity_label }}</strong>
            <span v-if="!own(h)"> — </span>{{ actionLabel(h.action) }}
          </div>
          <div class="faint small" :title="fmtDateTime(h.at)">{{ fmtAgo(h.at) }}<template v-if="h.user"> · {{ h.user.name }}</template></div>
          <div v-for="c in h.changes" :key="c.field" class="change">
            <strong>{{ c.field }}</strong> :
            <s v-if="c.from">{{ c.from }}</s><template v-else><em>vide</em></template>
            → {{ c.to ?? 'vide' }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
