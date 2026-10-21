<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import SettingsAccount from '../components/settings/SettingsAccount.vue'
import SettingsBusiness from '../components/settings/SettingsBusiness.vue'
import SettingsExtension from '../components/settings/SettingsExtension.vue'
import SettingsGeneral from '../components/settings/SettingsGeneral.vue'
import SettingsRoles from '../components/settings/SettingsRoles.vue'
import SettingsUsers from '../components/settings/SettingsUsers.vue'
import { can } from '../lib/store'

const route = useRoute()
const router = useRouter()

const tabs = computed(() => [
  { id: 'general', label: 'Général', show: true },
  { id: 'account', label: 'Mon compte', show: true },
  { id: 'users', label: 'Utilisateurs', show: can('users') },
  { id: 'roles', label: 'Rôles et droits', show: can('users') },
  { id: 'business', label: 'Paramètres métier', show: true },
  { id: 'extension', label: 'Extension LinkedIn', show: true },
].filter((t) => t.show))

const tab = computed(() => tabs.value.find((t) => t.id === route.query.tab)?.id ?? 'general')
const setTab = (id: string) => router.replace({ query: { tab: id } })
</script>

<template>
  <div class="page">
    <div class="page-head"><div><h1>Réglages</h1><p class="sub">Personnalisez l'application, les droits et les paramètres métier de votre ESN.</p></div></div>
    <div class="tabs" role="tablist" style="align-self: flex-start; flex-wrap: wrap">
      <button v-for="t in tabs" :key="t.id" type="button" role="tab" :aria-selected="tab === t.id" :class="{ on: tab === t.id }" @click="setTab(t.id)">{{ t.label }}</button>
    </div>
    <SettingsGeneral v-if="tab === 'general'" />
    <SettingsAccount v-else-if="tab === 'account'" />
    <SettingsUsers v-else-if="tab === 'users'" />
    <SettingsRoles v-else-if="tab === 'roles'" />
    <SettingsBusiness v-else-if="tab === 'business'" />
    <SettingsExtension v-else />
  </div>
</template>
