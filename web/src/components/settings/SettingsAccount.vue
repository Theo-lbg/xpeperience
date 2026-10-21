<script setup lang="ts">
import { reactive } from 'vue'
import { api } from '../../lib/api'
import { fmtDateTime } from '../../lib/format'
import { useForm } from '../../lib/forms'
import { me, notify } from '../../lib/store'
import Field from '../Field.vue'

const form = reactive({ current_password: '', new_password: '', confirm: '' })
const { errors, message, busy, submit } = useForm()

async function save() {
  if (form.new_password !== form.confirm) {
    errors.value = { confirm: 'Les deux mots de passe ne correspondent pas.' }
    return
  }
  const ok = await submit(() => api.put('/api/me/password', { current_password: form.current_password, new_password: form.new_password }))
  if (ok) {
    Object.assign(form, { current_password: '', new_password: '', confirm: '' })
    notify('Mot de passe modifié. Vos autres sessions ont été fermées.')
  }
}
</script>

<template>
  <div class="grid-2">
    <section class="card stack">
      <h2>Mon compte</h2>
      <dl class="kv" v-if="me">
        <dt>Nom</dt><dd>{{ me.first_name }} {{ me.last_name }}</dd>
        <dt>E-mail</dt><dd>{{ me.email }}</dd>
        <dt>Rôle</dt><dd>{{ me.role_name }}</dd>
        <dt>Dernière connexion</dt><dd>{{ fmtDateTime(me.last_login_at) }}</dd>
      </dl>
    </section>
    <form class="card stack" @submit.prevent="save" novalidate>
      <h2>Changer de mot de passe</h2>
      <Field label="Mot de passe actuel" :error="errors.current_password" for="ap"><input id="ap" v-model="form.current_password" class="input" type="password" autocomplete="current-password" /></Field>
      <Field label="Nouveau mot de passe" :error="errors.new_password" hint="Au moins 8 caractères." for="an"><input id="an" v-model="form.new_password" class="input" type="password" autocomplete="new-password" /></Field>
      <Field label="Confirmer" :error="errors.confirm" for="ac"><input id="ac" v-model="form.confirm" class="input" type="password" autocomplete="new-password" /></Field>
      <p v-if="message" class="alert error" role="alert">{{ message }}</p>
      <div><button class="btn primary" :disabled="busy">Modifier le mot de passe</button></div>
    </form>
  </div>
</template>
