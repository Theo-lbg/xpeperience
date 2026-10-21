<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { ApiError, api } from '../../lib/api'
import { useForm } from '../../lib/forms'
import { boot, can, confirmAction, notify, reloadBoot } from '../../lib/store'
import type { Action, Area, Permissions, Role } from '../../lib/types'
import Field from '../Field.vue'
import Icon from '../Icon.vue'
import Modal from '../Modal.vue'

const roles = ref<Role[]>([])
const editing = ref<{ r?: Role } | null>(null)
const name = ref('')
const description = ref('')
const perms = ref<Permissions>({})
const { errors, message, busy, submit } = useForm()
const areas = boot.value!.areas
const ACTION_LABEL: Record<Action, string> = { read: 'Consulter', create: 'Créer', update: 'Modifier', delete: 'Supprimer' }

async function load() {
  roles.value = await api.get<Role[]>('/api/roles')
}
onMounted(load)

function open(r?: Role) {
  name.value = r?.name ?? ''
  description.value = r?.description ?? ''
  perms.value = JSON.parse(JSON.stringify(r?.permissions ?? {}))
  errors.value = {}
  message.value = ''
  editing.value = { r }
}

const has = (a: Area, x: Action) => perms.value[a]?.includes(x) ?? false
function toggle(a: Area, x: Action) {
  const set = new Set(perms.value[a] ?? [])
  if (set.has(x)) {
    set.delete(x)
    if (x === 'read') set.clear() // sans lecture, plus aucun droit sur le domaine
  } else {
    set.add(x)
    set.add('read')
  }
  perms.value = { ...perms.value, [a]: [...set] as Action[] }
}

async function save() {
  const r = editing.value?.r
  const body = { name: name.value, description: description.value || null, permissions: perms.value }
  const ok = await submit(() => (r ? api.put(`/api/roles/${r.id}`, body) : api.post('/api/roles', body)), ['name'])
  if (ok) {
    notify(r ? 'Rôle modifié.' : 'Rôle créé.')
    editing.value = null
    await load()
    await reloadBoot()
  }
}

async function remove(r: Role) {
  if (!(await confirmAction('Supprimer ce rôle ?', `« ${r.name} » sera supprimé. Possible seulement si plus aucun utilisateur ne l'utilise.`, 'Supprimer', true))) return
  try {
    await api.del(`/api/roles/${r.id}`)
    notify('Rôle supprimé.')
    await load()
  } catch (e) {
    notify(e instanceof ApiError ? e.message : 'Suppression impossible.', 'error')
  }
}
const summary = (r: Role) => (r.is_admin ? 'Tous les droits' : areas.filter((a) => r.permissions[a.area]?.length).map((a) => a.label.split(' (')[0]).join(', ') || 'Aucun droit')
</script>

<template>
  <section class="card flush">
    <div class="card-head">
      <div><h2>Rôles et droits</h2><p class="muted small" style="margin-top: 4px">Les droits sont vérifiés par le serveur, pas seulement dans l'interface.</p></div>
      <button v-if="can('users', 'create')" class="btn primary small" type="button" @click="open()"><Icon name="plus" /> Rôle</button>
    </div>
    <div class="table-wrap">
      <table class="table">
        <thead><tr><th>Rôle</th><th>Accès</th><th class="right">Utilisateurs</th><th /></tr></thead>
        <tbody>
          <tr v-for="r in roles" :key="r.id">
            <td><span class="name">{{ r.name }}</span><span v-if="r.is_admin" class="pill plain green" style="margin-left: 8px">Administrateur</span><div class="sub">{{ r.description }}</div></td>
            <td class="muted">{{ summary(r) }}</td>
            <td class="right">{{ r.users_count }}</td>
            <td class="right nowrap">
              <button v-if="can('users', 'update')" class="btn small ghost icon" type="button" aria-label="Modifier" @click="open(r)"><Icon name="edit" /></button>
              <button v-if="can('users', 'delete') && !r.is_admin" class="btn small ghost icon" type="button" aria-label="Supprimer" @click="remove(r)"><Icon name="trash" /></button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>

  <Modal v-if="editing" :title="editing.r ? `Rôle : ${editing.r.name}` : 'Nouveau rôle'" size="wide" :busy="busy" @close="editing = null">
    <form id="role-form" class="stack" @submit.prevent="save" novalidate>
      <div class="form-grid">
        <Field label="Nom du rôle" required :error="errors.name" for="rn"><input id="rn" v-model="name" class="input" autofocus /></Field>
        <Field label="Description" for="rd"><input id="rd" v-model="description" class="input" /></Field>
      </div>
      <p v-if="editing.r?.is_admin" class="alert">Le rôle administrateur garde tous les droits : seuls son nom et sa description peuvent changer.</p>
      <div v-else class="table-wrap">
        <table class="table">
          <thead><tr><th>Domaine</th><th v-for="x in (['read', 'create', 'update', 'delete'] as Action[])" :key="x" class="right">{{ ACTION_LABEL[x] }}</th></tr></thead>
          <tbody>
            <tr v-for="a in areas" :key="a.area">
              <td>{{ a.label }}</td>
              <td v-for="x in (['read', 'create', 'update', 'delete'] as Action[])" :key="x" class="right">
                <input v-if="a.actions.includes(x)" type="checkbox" class="cb" :checked="has(a.area, x)" :aria-label="`${ACTION_LABEL[x]} — ${a.label}`" @change="toggle(a.area, x)" />
                <span v-else class="faint">·</span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <p class="faint small">« Coûts et contrats RH » protège le CJM : sans « Consulter », le CJM n'est jamais envoyé à l'interface. Les droits d'écriture incluent la consultation.</p>
      <p v-if="message" class="alert error" role="alert">{{ message }}</p>
    </form>
    <template #footer>
      <button class="btn" type="button" :disabled="busy" @click="editing = null">Annuler</button>
      <button class="btn primary" form="role-form" :disabled="busy">Enregistrer</button>
    </template>
  </Modal>
</template>

<style scoped>
.cb { width: 18px; height: 18px; accent-color: var(--green-strong); }
</style>
