<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { api } from '../../lib/api'
import { fmtDateTime } from '../../lib/format'
import { useForm } from '../../lib/forms'
import { can, me, notify, reloadBoot } from '../../lib/store'
import type { Role, User } from '../../lib/types'
import EmptyState from '../EmptyState.vue'
import Field from '../Field.vue'
import Icon from '../Icon.vue'
import Modal from '../Modal.vue'

const users = ref<User[]>([])
const roles = ref<Role[]>([])
const editing = ref<{ u?: User } | null>(null)
const resetting = ref<User | null>(null)
const form = reactive({ email: '', first_name: '', last_name: '', role_id: 0, active: true, password: '' })
const pw = ref('')
const { errors, message, busy, submit } = useForm()

async function load() {
  ;[users.value, roles.value] = await Promise.all([api.get<User[]>('/api/users'), api.get<Role[]>('/api/roles')])
}
onMounted(load)

function open(u?: User) {
  Object.assign(form, u ? { email: u.email, first_name: u.first_name, last_name: u.last_name, role_id: u.role_id, active: u.active, password: '' } : { email: '', first_name: '', last_name: '', role_id: roles.value.find((r) => !r.is_admin)?.id ?? roles.value[0]?.id ?? 0, active: true, password: '' })
  errors.value = {}
  message.value = ''
  editing.value = { u }
}

async function save() {
  const u = editing.value?.u
  const ok = await submit(() => (u ? api.put(`/api/users/${u.id}`, form) : api.post('/api/users', form)), Object.keys(form))
  if (ok) {
    notify(u ? 'Utilisateur modifié.' : 'Utilisateur créé.')
    editing.value = null
    await load()
    await reloadBoot()
  }
}

async function resetPassword() {
  const ok = await submit(() => api.post(`/api/users/${resetting.value!.id}/password`, { password: pw.value }))
  if (ok !== undefined || !message.value) {
    if (!message.value && !Object.keys(errors.value).length) {
      notify('Mot de passe réinitialisé. Ses sessions ont été fermées.')
      resetting.value = null
      pw.value = ''
    }
  }
}
</script>

<template>
  <section class="card flush">
    <div class="card-head"><h2>Utilisateurs ({{ users.length }})</h2><button v-if="can('users', 'create')" class="btn primary small" type="button" @click="open()"><Icon name="plus" /> Utilisateur</button></div>
    <EmptyState v-if="!users.length" compact title="Aucun utilisateur" />
    <div v-else class="table-wrap">
      <table class="table">
        <thead><tr><th>Nom</th><th>E-mail</th><th>Rôle</th><th>Dernière connexion</th><th>Statut</th><th /></tr></thead>
        <tbody>
          <tr v-for="u in users" :key="u.id" :class="{ 'archived-row': !u.active }">
            <td class="name">{{ u.first_name }} {{ u.last_name }}<span v-if="u.id === me?.id" class="faint small"> (vous)</span></td>
            <td>{{ u.email }}</td>
            <td><span class="pill plain" :class="u.is_admin ? 'green' : 'gray'">{{ u.role_name }}</span></td>
            <td class="muted">{{ fmtDateTime(u.last_login_at) }}</td>
            <td><span class="pill" :class="u.active ? 'green' : 'gray'">{{ u.active ? 'Actif' : 'Désactivé' }}</span></td>
            <td class="right nowrap">
              <template v-if="can('users', 'update')">
                <button class="btn small ghost" type="button" @click="resetting = u; pw = ''; errors = {}; message = ''">Mot de passe</button>
                <button class="btn small ghost icon" type="button" aria-label="Modifier" @click="open(u)"><Icon name="edit" /></button>
              </template>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>

  <Modal v-if="editing" :title="editing.u ? 'Modifier l’utilisateur' : 'Nouvel utilisateur'" :busy="busy" @close="editing = null">
    <form id="user-form" class="form-grid" @submit.prevent="save" novalidate>
      <Field label="Prénom" required :error="errors.first_name" for="uf"><input id="uf" v-model="form.first_name" class="input" autofocus /></Field>
      <Field label="Nom" required :error="errors.last_name" for="ul"><input id="ul" v-model="form.last_name" class="input" /></Field>
      <Field class="full" label="Adresse e-mail" required :error="errors.email" for="ue"><input id="ue" v-model="form.email" class="input" type="email" /></Field>
      <Field label="Rôle" :error="errors.role_id" for="ur"><select id="ur" v-model="form.role_id" class="select"><option v-for="r in roles" :key="r.id" :value="r.id">{{ r.name }}</option></select></Field>
      <Field v-if="editing.u" label="Statut" for="us"><select id="us" v-model="form.active" class="select"><option :value="true">Actif</option><option :value="false">Désactivé</option></select></Field>
      <Field v-else label="Mot de passe initial" required :error="errors.password" hint="Au moins 8 caractères." for="up"><input id="up" v-model="form.password" class="input" type="password" autocomplete="new-password" /></Field>
      <p v-if="message" class="full alert error" role="alert">{{ message }}</p>
    </form>
    <template #footer>
      <button class="btn" type="button" :disabled="busy" @click="editing = null">Annuler</button>
      <button class="btn primary" form="user-form" :disabled="busy">Enregistrer</button>
    </template>
  </Modal>

  <Modal v-if="resetting" :title="`Nouveau mot de passe — ${resetting.first_name}`" size="narrow" :busy="busy" @close="resetting = null">
    <Field label="Nouveau mot de passe" :error="errors.password" hint="Ses sessions ouvertes seront fermées." for="rp"><input id="rp" v-model="pw" class="input" type="password" autocomplete="new-password" autofocus /></Field>
    <p v-if="message" class="alert error" role="alert" style="margin-top: 12px">{{ message }}</p>
    <template #footer>
      <button class="btn" type="button" :disabled="busy" @click="resetting = null">Annuler</button>
      <button class="btn primary" type="button" :disabled="busy" @click="resetPassword">Réinitialiser</button>
    </template>
  </Modal>
</template>
