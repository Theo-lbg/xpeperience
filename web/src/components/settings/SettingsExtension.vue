<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { api } from '../../lib/api'
import { fmtDateTime } from '../../lib/format'
import { useForm } from '../../lib/forms'
import { boot, can, confirmAction, notify, reloadBoot } from '../../lib/store'
import type { ApiToken, Settings } from '../../lib/types'
import Icon from '../Icon.vue'
import Modal from '../Modal.vue'

const b = boot.value!
const canEdit = can('settings', 'update')
const ext = reactive({ ...b.settings.extension })
const { message, busy, submit } = useForm()
const tokens = ref<ApiToken[]>([])
const created = ref<{ secret: string; name: string } | null>(null)
const name = ref('Mon navigateur')
const copied = ref(false)
const address = b.address

async function load() {
  tokens.value = await api.get<ApiToken[]>('/api/tokens')
}
onMounted(load)

async function saveExt() {
  const ok = await submit(() => api.put<Settings>('/api/settings', { ...b.settings, extension: ext }))
  if (ok) {
    await reloadBoot()
    notify('Paramètres de l’extension enregistrés.')
  }
}
async function createToken() {
  const res = await api.post<{ secret: string; token: ApiToken }>('/api/tokens', { name: name.value })
  created.value = { secret: res.secret, name: res.token.name }
  copied.value = false
  await load()
}
async function revoke(t: ApiToken) {
  if (!(await confirmAction('Révoquer ce jeton ?', `Le navigateur « ${t.name} » ne pourra plus importer de contacts tant que vous ne l'aurez pas relié à nouveau.`, 'Révoquer', true))) return
  await api.del(`/api/tokens/${t.id}`)
  notify('Jeton révoqué.')
  await load()
}
async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text)
    copied.value = true
  } catch {
    notify('Copie impossible : sélectionnez le texte et copiez-le à la main.', 'error')
  }
}
</script>

<template>
  <div class="grid-2">
    <div class="stack">
      <form class="card stack" @submit.prevent="saveExt">
        <div><h2>Import depuis LinkedIn</h2><p class="muted small" style="margin-top: 4px">L'extension ajoute un bouton « Ajouter à XpePerience » sur les profils LinkedIn. Rien n'est lu ni envoyé avant votre clic, et chaque import passe par un aperçu que vous validez.</p></div>
        <label class="check"><input v-model="ext.enabled" type="checkbox" :disabled="!canEdit" /><span>Autoriser l'import de contacts depuis LinkedIn</span></label>
        <label class="check"><input v-model="ext.allow_company_creation" type="checkbox" :disabled="!canEdit" /><span>Autoriser la création d'une société depuis l'extension</span></label>
        <label class="check"><input v-model="ext.add_import_note" type="checkbox" :disabled="!canEdit" /><span>Ajouter une note « importé depuis LinkedIn » sur la fiche</span></label>
        <p v-if="message" class="alert error" role="alert">{{ message }}</p>
        <div v-if="canEdit"><button class="btn primary" :disabled="busy">Enregistrer</button></div>
      </form>

      <section class="card stack">
        <h2>Installer l'extension</h2>
        <ol class="steps">
          <li>Ouvrez <code>chrome://extensions</code> (ou <code>edge://extensions</code>) et activez le <strong>mode développeur</strong>.</li>
          <li>Cliquez sur <strong>Charger l'extension non empaquetée</strong> et choisissez le dossier <code>extension/</code> du projet.</li>
          <li>Ouvrez les options de l'extension, collez l'adresse <code>{{ address }}</code> et le jeton créé ci-contre. Pour un serveur distant, Chrome demande d'autoriser l'accès à cette adresse : acceptez.</li>
          <li>Sur un profil LinkedIn, cliquez sur l'icône de l'extension puis sur <strong>Ajouter à XpePerience</strong>.</li>
        </ol>
        <p class="faint small">L'extension ne communique qu'avec l'adresse que vous avez saisie, jamais avec un autre service.</p>
      </section>
    </div>

    <section class="card stack">
      <div><h2>Jetons d'appairage</h2><p class="muted small" style="margin-top: 4px">Un jeton relie un navigateur à votre compte, avec vos droits. Révoquez-le si vous perdez l'appareil.</p></div>
      <div class="row">
        <input v-model="name" class="input" aria-label="Nom du jeton" placeholder="Nom du navigateur" />
        <button class="btn primary nowrap" type="button" @click="createToken"><Icon name="plus" /> Créer un jeton</button>
      </div>
      <p v-if="!tokens.length" class="faint">Aucun jeton pour le moment.</p>
      <div v-else class="list">
        <div v-for="t in tokens" :key="t.id" class="row">
          <Icon name="shield" style="width: 20px; color: var(--ink-3)" />
          <div class="grow">
            <strong>{{ t.name }}</strong> <span class="mono">{{ t.prefix }}…</span> <span v-if="t.revoked" class="pill gray plain">Révoqué</span>
            <div class="faint small">{{ t.user_name }} · créé le {{ fmtDateTime(t.created_at) }} · {{ t.last_used_at ? 'utilisé ' + fmtDateTime(t.last_used_at) : 'jamais utilisé' }}</div>
          </div>
          <button v-if="!t.revoked" class="btn small danger" type="button" @click="revoke(t)">Révoquer</button>
        </div>
      </div>
    </section>
  </div>

  <Modal v-if="created" title="Votre jeton d'appairage" @close="created = null">
    <div class="stack">
      <p class="alert warn">Copiez-le maintenant : il ne sera <strong>plus affiché</strong>. Collez-le dans les options de l'extension.</p>
      <div class="row"><code class="grow" style="padding: 10px 12px; word-break: break-all; user-select: all">{{ created.secret }}</code><button class="btn" type="button" @click="copy(created.secret)">{{ copied ? 'Copié ✓' : 'Copier' }}</button></div>
    </div>
    <template #footer><button class="btn primary" type="button" @click="created = null">J'ai copié le jeton</button></template>
  </Modal>
</template>

<style scoped>
.steps { margin: 0; padding-left: 20px; display: flex; flex-direction: column; gap: 8px; }
</style>
