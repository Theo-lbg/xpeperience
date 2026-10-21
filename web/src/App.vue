<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import GlobalSearch from './components/GlobalSearch.vue'
import Host from './components/Host.vue'
import Sidebar from './components/Sidebar.vue'
import { session } from './lib/store'

const route = useRoute()
const shell = computed(() => session.authenticated && !route.meta.public)
</script>

<template>
  <template v-if="session.loaded">
    <template v-if="shell">
      <Sidebar />
      <div class="main">
        <header class="topbar"><GlobalSearch /></header>
        <RouterView v-slot="{ Component }">
          <component :is="Component" :key="route.path" />
        </RouterView>
      </div>
    </template>
    <RouterView v-else />
  </template>
  <div v-else class="boot"><div class="skeleton" style="width: 160px; height: 14px" /></div>
  <Host />
</template>

<style>
.main { margin-left: var(--sidebar); min-height: 100%; }
.topbar { position: sticky; top: 0; z-index: 10; display: flex; align-items: center; padding: 14px 36px; background: rgba(244, 245, 242, 0.85); backdrop-filter: blur(14px); border-bottom: 1px solid transparent; }
.boot { height: 100%; display: grid; place-items: center; }
@media (max-width: 860px) { .main { margin-left: 0; } .topbar { padding: 12px 20px; } }
</style>
