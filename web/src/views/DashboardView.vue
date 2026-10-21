<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import EmptyState from '../components/EmptyState.vue'
import Icon from '../components/Icon.vue'
import { ApiError, api } from '../lib/api'
import { ACTIVITY_LABEL, actionLabel, entityLabel, fmtAgo, fmtDate, fmtDateTime, historyRoute, todayIso } from '../lib/format'
import { can, me } from '../lib/store'
import type { Dashboard, EntityKind } from '../lib/types'

const data = ref<Dashboard | null>(null)
const error = ref('')

onMounted(async () => {
  try {
    data.value = await api.get<Dashboard>('/api/dashboard')
  } catch (e) {
    error.value = e instanceof ApiError ? e.message : 'Chargement impossible.'
  }
})

const cards = computed(() => {
  const c = data.value?.counts
  return [
    { label: 'Sociétés', value: c?.companies, to: '/crm?tab=companies', icon: 'building', area: 'crm' as const },
    { label: 'Contacts', value: c?.contacts, to: '/crm?tab=contacts', icon: 'users', area: 'crm' as const },
    { label: 'Besoins ouverts', value: c?.open_needs, to: '/needs', icon: 'target', area: 'needs' as const },
    { label: 'Ressources', value: c?.resources, to: '/resources', icon: 'briefcase', area: 'resources' as const, sub: c ? `${c.candidates} candidat(s) · ${c.consultants} consultant(s)` : '' },
  ].filter((k) => can(k.area))
})

const icon: Record<EntityKind, string> = { company: 'building', contact: 'users', need: 'target', positioning: 'link', resource: 'briefcase', project: 'folder', delivery: 'folder' }
const overdue = (d: string | null) => !!d && d < todayIso()
</script>

<template>
  <div class="page">
    <section class="hero">
      <div class="hero-text">
        <h1>Salut {{ me?.first_name }}</h1>
        <p>Content de vous revoir. Retrouvez vos clients, vos besoins et vos équipes en un clin d'œil.</p>
      </div>
      <svg class="art" viewBox="0 0 260 150" fill="none" aria-hidden="true">
        <circle cx="62" cy="76" r="22" fill="#fff" fill-opacity=".85" />
        <circle cx="198" cy="52" r="18" fill="#fff" fill-opacity=".85" />
        <circle cx="168" cy="116" r="14" fill="#111312" fill-opacity=".9" />
        <path d="M82 70 180 56M72 96 156 112M192 70 174 104" stroke="#111312" stroke-opacity=".55" stroke-width="2.5" stroke-linecap="round" stroke-dasharray="1 7" />
        <rect x="104" y="38" width="64" height="44" rx="12" fill="#fff" />
        <path d="M116 54h40M116 64h26" stroke="#B3D26B" stroke-width="5" stroke-linecap="round" />
        <path d="M46 70a10 10 0 0 1 16-4M62 90c-10 0-16 4-16 10" stroke="#111312" stroke-width="2.2" stroke-linecap="round" />
      </svg>
    </section>

    <p v-if="error" class="alert error" role="alert">{{ error }}</p>

    <section class="kpis" aria-label="Chiffres clés">
      <RouterLink v-for="k in cards" :key="k.label" :to="k.to" class="kpi card">
        <span class="kpi-icon"><Icon :name="k.icon" /></span>
        <span>
          <span class="kpi-label">{{ k.label }}</span>
          <strong v-if="data" class="kpi-value">{{ k.value ?? 0 }}</strong>
          <span v-else class="skeleton" style="display: block; width: 50px; height: 30px" />
          <small v-if="k.sub" class="faint">{{ k.sub }}</small>
        </span>
      </RouterLink>
    </section>

    <div class="split">
      <section class="card">
        <div class="card-head"><h2>Activité récente</h2></div>
        <div v-if="!data" class="stack tight"><div class="skeleton" style="height: 20px" /><div class="skeleton" style="height: 20px; width: 80%" /><div class="skeleton" style="height: 20px; width: 60%" /></div>
        <EmptyState v-else-if="!data.recent.length" icon="sparkle" title="Rien pour le moment">
          Dès que vous créerez une société, un contact, un besoin ou un projet, les dernières actions apparaîtront ici.
          <template #actions>
            <RouterLink v-if="can('crm', 'create')" class="btn primary" to="/crm?new=company">Créer une première société</RouterLink>
          </template>
        </EmptyState>
        <div v-else class="list">
          <component :is="historyRoute(h) ? 'RouterLink' : 'div'" v-for="h in data.recent" :key="h.id" :to="historyRoute(h) ?? undefined" class="recent">
            <span class="r-icon"><Icon :name="icon[h.entity]" /></span>
            <span class="grow">
              <strong>{{ h.entity_label }}</strong>
              <span class="muted"> · {{ entityLabel(h.entity) }} {{ actionLabel(h.action) }}</span>
              <span v-if="h.changes.length && h.action !== 'created'" class="faint small block">{{ h.changes.slice(0, 2).map((c) => c.field).join(', ') }}</span>
            </span>
            <span class="faint small nowrap" :title="fmtDateTime(h.at)">{{ fmtAgo(h.at) }}<template v-if="h.user"> · {{ h.user.name.split(' ')[0] }}</template></span>
          </component>
        </div>
      </section>

      <section v-if="can('crm')" class="card">
        <div class="card-head"><h2>Mes rappels</h2><RouterLink to="/crm?tab=reminders" class="link small">Tout voir</RouterLink></div>
        <div v-if="!data" class="skeleton" style="height: 44px" />
        <EmptyState v-else-if="!data.reminders.length" compact title="Aucun rappel à venir">Les relances programmées sur vos fiches s'afficheront ici.</EmptyState>
        <div v-else class="list">
          <RouterLink v-for="a in data.reminders" :key="a.id" :to="a.contact ? `/crm/contacts/${a.contact.id}` : a.company ? `/crm/companies/${a.company.id}` : '/crm'" class="rem">
            <div class="row" style="gap: 8px">
              <strong>{{ ACTIVITY_LABEL[a.kind] }}</strong>
              <span class="pill plain" :class="overdue(a.due_date) ? 'red' : 'orange'">{{ overdue(a.due_date) ? 'En retard' : fmtDate(a.due_date) }}</span>
            </div>
            <div class="muted small">{{ a.contact?.name ?? a.company?.name }}<template v-if="a.comment"> — {{ a.comment }}</template></div>
          </RouterLink>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.hero { background: var(--green); border-radius: 24px; padding: 34px 40px; display: flex; align-items: center; justify-content: space-between; gap: 24px; overflow: hidden; color: var(--black); }
.hero h1 { font-size: 34px; letter-spacing: -0.03em; }
.hero p { margin-top: 8px; font-size: 16px; max-width: 460px; opacity: 0.8; }
.art { width: 260px; flex: none; }
@media (max-width: 760px) { .art { display: none; } .hero { padding: 24px; } }
.kpis { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 16px; }
.kpi { display: flex; align-items: center; gap: 16px; transition: transform 0.12s, border-color 0.12s; }
.kpi:hover { transform: translateY(-2px); border-color: var(--green-strong); }
.kpi-icon { width: 46px; height: 46px; border-radius: 14px; background: var(--green-soft); color: var(--green-ink); display: grid; place-items: center; }
.kpi-icon svg { width: 24px; height: 24px; }
.kpi-label { display: block; color: var(--ink-2); font-weight: 600; }
.kpi-value { display: block; font-size: 32px; letter-spacing: -0.03em; line-height: 1.15; }
.recent, .rem { display: flex; align-items: center; gap: 12px; border-radius: 10px; }
.rem { flex-direction: column; align-items: flex-start; gap: 2px; }
a.recent:hover strong, a.rem:hover strong { text-decoration: underline; text-decoration-color: var(--green); text-underline-offset: 3px; }
.r-icon { width: 34px; height: 34px; border-radius: 10px; background: #f0f2ec; display: grid; place-items: center; flex: none; color: var(--ink-2); }
.r-icon svg { width: 17px; height: 17px; }
.block { display: block; }
</style>
