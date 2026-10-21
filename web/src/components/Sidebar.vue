<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { api } from '../lib/api'
import { boot, can, loadSession, me, session } from '../lib/store'
import { initials } from '../lib/format'
import type { Area } from '../lib/types'
import Icon from './Icon.vue'

const route = useRoute()
const router = useRouter()

// Emplacement du logo : image personnalisée (Réglages) ou logo fourni dans web/public/logo.png.
const customLogo = ref(true)
const logoSrc = computed(() => (customLogo.value ? `/api/branding/logo?v=${logoVersion.value}` : '/logo.png'))
const logoVersion = ref(Date.now())
const logoMissing = ref(false)

const items: { to: string; label: string; icon: string; area?: Area; match: string }[] = [
  { to: '/', label: 'Tableau de bord', icon: 'home', match: '/' },
  { to: '/crm', label: 'CRM', icon: 'users', area: 'crm', match: '/crm' },
  { to: '/needs', label: 'Besoins', icon: 'target', area: 'needs', match: '/needs' },
  { to: '/resources', label: 'Ressources', icon: 'briefcase', area: 'resources', match: '/resources' },
  { to: '/projects', label: 'Projets et prestations', icon: 'folder', area: 'projects', match: '/projects' },
  { to: '/settings', label: 'Réglages', icon: 'cog', match: '/settings' },
]

const visible = computed(() => items.filter((i) => !i.area || can(i.area)))
const isActive = (match: string) => (match === '/' ? route.path === '/' : route.path === match || route.path.startsWith(match + '/'))

const menu = ref(false)

async function logout() {
  await api.post('/api/auth/logout')
  boot.value = null
  session.authenticated = false
  await loadSession()
  router.push('/login')
}
</script>

<template>
  <aside class="sidebar">
    <RouterLink to="/" class="brand" aria-label="Boond by XPEHO — accueil">
      <img v-if="!logoMissing" :src="logoSrc" alt="XPEHO" class="logo" @error="customLogo ? (customLogo = false) : (logoMissing = true)" />
      <span v-else class="logo-slot">XPEHO</span>
      <span class="product">Boond <i>by XPEHO</i></span>
    </RouterLink>

    <nav class="nav" aria-label="Navigation principale">
      <RouterLink v-for="i in visible" :key="i.to" :to="i.to" class="nav-item" :class="{ active: isActive(i.match) }" :aria-current="isActive(i.match) ? 'page' : undefined">
        <Icon :name="i.icon" />
        <span>{{ i.label }}</span>
      </RouterLink>
    </nav>

    <div class="who" v-if="me">
      <button type="button" class="who-btn" @click="menu = !menu" :aria-expanded="menu">
        <span class="avatar">{{ initials(`${me.first_name} ${me.last_name}`) }}</span>
        <span class="grow">
          <strong>{{ me.first_name }} {{ me.last_name }}</strong>
          <small>{{ me.role_name }}</small>
        </span>
        <Icon name="down" />
      </button>
      <div v-if="menu" class="menu who-menu" @mouseleave="menu = false">
        <RouterLink to="/settings?tab=account" @click="menu = false"><Icon name="user" /> Mon compte</RouterLink>
        <div class="sep" />
        <button type="button" @click="logout"><Icon name="logout" /> Se déconnecter</button>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.sidebar { position: fixed; inset: 0 auto 0 0; width: var(--sidebar); background: #fff; border-right: 1px solid var(--line); display: flex; flex-direction: column; padding: 20px 14px 14px; gap: 22px; z-index: 20; }
.brand { display: flex; flex-direction: column; gap: 6px; padding: 0 8px; }
.logo { height: 34px; width: auto; max-width: 160px; object-fit: contain; align-self: flex-start; }
.logo-slot { height: 34px; display: inline-grid; place-items: center; align-self: flex-start; padding: 0 12px; border: 1.5px dashed var(--line-strong); border-radius: 8px; font-weight: 800; letter-spacing: 0.1em; color: var(--green-strong); }
.product { font-weight: 700; font-size: 15px; letter-spacing: -0.01em; }
.product i { font-style: normal; font-weight: 500; color: var(--ink-3); }
.nav { display: flex; flex-direction: column; gap: 4px; flex: 1; }
.nav-item { position: relative; display: flex; align-items: center; gap: 12px; height: 42px; padding: 0 14px; border-radius: 12px; font-weight: 600; color: var(--ink-2); transition: background 0.12s, color 0.12s; }
.nav-item svg { width: 20px; height: 20px; flex: none; }
.nav-item:hover { background: #f4f5f1; color: var(--ink); }
.nav-item.active { background: var(--green-soft); color: var(--black); }
.nav-item.active::before { content: ''; position: absolute; left: -14px; top: 9px; bottom: 9px; width: 4px; border-radius: 0 4px 4px 0; background: var(--green); }
.nav-item.active svg { color: var(--green-ink); }
.who { position: relative; border-top: 1px solid var(--line); padding-top: 12px; }
.who-btn { display: flex; align-items: center; gap: 10px; width: 100%; border: 0; background: none; padding: 6px 8px; border-radius: 12px; text-align: left; }
.who-btn:hover { background: #f4f5f1; }
.who-btn strong, .who-btn small { display: block; }
.who-btn small { color: var(--ink-3); }
.who-btn > svg { width: 16px; color: var(--ink-3); }
.who-menu { bottom: 62px; left: 8px; right: 8px; }
@media (max-width: 860px) {
  .sidebar { position: static; width: auto; flex-direction: row; flex-wrap: wrap; padding: 10px 14px; gap: 10px; border-right: 0; border-bottom: 1px solid var(--line); }
  .nav { flex-direction: row; flex-wrap: wrap; }
  .nav-item.active::before { display: none; }
  .who { border: 0; padding: 0; }
}
</style>
