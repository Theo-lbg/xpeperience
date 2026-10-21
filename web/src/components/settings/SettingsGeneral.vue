<script setup lang="ts">
import { reactive, ref } from 'vue'
import { api } from '../../lib/api'
import { fmtDate } from '../../lib/format'
import { useForm } from '../../lib/forms'
import { boot, can, notify, reloadBoot } from '../../lib/store'
import type { Preferences, Settings } from '../../lib/types'
import Field from '../Field.vue'

const canEdit = can('settings', 'update')
const b = boot.value!
const form = reactive({
  app_name: b.settings.app_name, locale: b.settings.locale, date_format: b.settings.date_format, currency: b.settings.currency,
  reminders: { ...b.settings.reminders },
})
const prefs = reactive<Preferences>({ ...b.me.preferences })
const { errors, message, busy, submit } = useForm()
const logoKey = ref(Date.now())
const logoBusy = ref(false)

async function save() {
  const ok = await submit(() => api.put<Settings>('/api/settings', { ...b.settings, ...form }))
  if (ok) {
    await reloadBoot()
    notify('Réglages enregistrés.')
  }
}

async function savePrefs() {
  await api.put('/api/me/preferences', prefs)
  await reloadBoot()
  notify('Préférences d’affichage enregistrées.')
}

async function uploadLogo(list: FileList | null) {
  const file = list?.[0]
  if (!file) return
  logoBusy.value = true
  try {
    await api.putFile('/api/settings/logo', file)
    notify('Logo mis à jour. Rechargez la page pour le voir dans le menu.')
    logoKey.value = Date.now()
  } catch (e: any) {
    notify(e.message ?? 'Envoi impossible.', 'error')
  } finally {
    logoBusy.value = false
  }
}
async function removeLogo() {
  await api.del('/api/settings/logo')
  logoKey.value = Date.now()
  notify('Logo personnalisé retiré.')
}
</script>

<template>
  <div class="grid-2">
    <form class="card stack" @submit.prevent="save" novalidate>
      <h2>Application</h2>
      <Field label="Nom de l'application" :error="errors.app_name" for="gn"><input id="gn" v-model="form.app_name" class="input" :disabled="!canEdit" /></Field>
      <Field label="Langue et région" :error="errors.locale" for="gl">
        <select id="gl" v-model="form.locale" class="select" :disabled="!canEdit"><option v-for="[code, label] in b.locales" :key="code" :value="code">{{ label }}</option></select>
      </Field>
      <div class="form-grid">
        <Field label="Format de date" :error="errors.date_format" :hint="`Exemple : ${(() => { const s = b.settings.date_format; return fmtDate('2026-03-07') })()}`" for="gd">
          <select id="gd" v-model="form.date_format" class="select" :disabled="!canEdit"><option v-for="f in b.date_formats" :key="f" :value="f">{{ f }}</option></select>
        </Field>
        <Field label="Devise" :error="errors.currency" for="gc"><select id="gc" v-model="form.currency" class="select" :disabled="!canEdit"><option v-for="c in b.currencies" :key="c" :value="c">{{ c }}</option></select></Field>
      </div>

      <h2 style="margin-top: 8px">Notifications et rappels</h2>
      <label class="check"><input v-model="form.reminders.enabled" type="checkbox" :disabled="!canEdit" /><span>Afficher mes rappels à venir sur le tableau de bord</span></label>
      <div class="form-grid">
        <Field label="Relance par défaut dans (jours)" :error="errors['reminders.default_follow_up_days']" for="gr"><input id="gr" v-model.number="form.reminders.default_follow_up_days" class="input" type="number" min="0" max="365" :disabled="!canEdit" /></Field>
        <Field label="Rappels « à venir » : fenêtre (jours)" :error="errors['reminders.upcoming_days']" for="gu"><input id="gu" v-model.number="form.reminders.upcoming_days" class="input" type="number" min="1" max="90" :disabled="!canEdit" /></Field>
      </div>
      <p v-if="message" class="alert error" role="alert">{{ message }}</p>
      <div v-if="canEdit" class="row"><button class="btn primary" :disabled="busy">{{ busy ? 'Enregistrement…' : 'Enregistrer' }}</button></div>
      <p v-else class="faint small">Seuls les utilisateurs autorisés peuvent modifier ces réglages.</p>
    </form>

    <div class="stack">
      <section class="card stack">
        <h2>Logo XPEHO</h2>
        <p class="muted small">Le logo s'affiche en haut du menu de gauche. Par défaut, celui du dossier <code>web/public/logo.png</code> est utilisé ; vous pouvez le remplacer ici.</p>
        <div class="logo-prev"><img :key="logoKey" :src="`/api/branding/logo?v=${logoKey}`" alt="Logo personnalisé" @error="($event.target as HTMLImageElement).src = '/logo.png'" /></div>
        <div v-if="canEdit" class="row wrap">
          <label class="btn small primary" :style="{ opacity: logoBusy ? 0.6 : 1 }">Choisir un logo<input type="file" hidden accept="image/png,image/jpeg,image/webp" :disabled="logoBusy" @change="uploadLogo(($event.target as HTMLInputElement).files)" /></label>
          <button class="btn small" type="button" @click="removeLogo">Revenir au logo fourni</button>
          <span class="faint small">PNG, JPEG ou WebP · 2 Mo max</span>
        </div>
      </section>

      <section class="card stack">
        <h2>Préférences d'affichage <span class="faint small" style="font-weight: 500">(pour vous)</span></h2>
        <Field label="Densité" for="pd">
          <select id="pd" v-model="prefs.density" class="select"><option value="comfortable">Confortable</option><option value="compact">Compacte</option></select>
        </Field>
        <Field label="Vue des besoins par défaut" for="pv">
          <select id="pv" v-model="prefs.needs_view" class="select"><option value="list">Liste</option><option value="kanban">Kanban</option></select>
        </Field>
        <div><button class="btn" type="button" @click="savePrefs">Enregistrer mes préférences</button></div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.logo-prev { background: #fafbf8; border: 1px dashed var(--line-strong); border-radius: 12px; padding: 18px; display: grid; place-items: center; min-height: 90px; }
.logo-prev img { max-height: 54px; max-width: 220px; }
</style>
