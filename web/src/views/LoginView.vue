<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import Field from '../components/Field.vue'
import { ApiError, api } from '../lib/api'
import { loadSession, session } from '../lib/store'

const router = useRouter()
const route = useRoute()
const setup = computed(() => session.setupRequired)
const form = reactive({ first_name: '', last_name: '', email: '', password: '', setup_code: '' })
const errors = ref<Record<string, string>>({})
const message = ref('')
const busy = ref(false)
const logoOk = ref(true)
// Logo personnalisé (Réglages), sinon le logo fourni dans web/public/logo.png.
const customLogo = '/api/branding/logo'
const defaultLogo = '/logo.png'

async function submit() {
  busy.value = true
  errors.value = {}
  message.value = ''
  try {
    if (setup.value) await api.post('/api/auth/setup', form)
    else await api.post('/api/auth/login', { email: form.email, password: form.password })
    await loadSession()
    const next = typeof route.query.next === 'string' && route.query.next.startsWith('/') ? route.query.next : '/'
    router.replace(next)
  } catch (e) {
    if (e instanceof ApiError) {
      errors.value = e.fieldMap
      if (!e.fields.length) message.value = e.message
    } else message.value = 'Connexion impossible.'
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <main class="login">
    <form class="card box" @submit.prevent="submit" novalidate>
      <img :src="logoOk ? customLogo : defaultLogo" alt="XPEHO" class="logo" @error="logoOk = false" />
      <div>
        <h1>{{ setup ? 'Bienvenue' : 'Connexion' }}</h1>
        <p class="muted" style="margin-top: 6px">
          {{ setup ? 'Créez le compte administrateur pour démarrer XpePerience.' : `Connectez-vous à ${session.appName}.` }}
        </p>
      </div>
      <div v-if="setup" class="form-grid">
        <Field label="Prénom" :error="errors.first_name" for="fn"><input id="fn" v-model="form.first_name" class="input" :class="{ invalid: errors.first_name }" autocomplete="given-name" autofocus /></Field>
        <Field label="Nom" :error="errors.last_name" for="ln"><input id="ln" v-model="form.last_name" class="input" :class="{ invalid: errors.last_name }" autocomplete="family-name" /></Field>
      </div>
      <Field v-if="setup && session.setupCodeRequired" label="Code d'installation" :error="errors.setup_code" hint="Affiché dans les journaux du serveur au premier démarrage (docker compose logs xpeperience)." for="sc">
        <input id="sc" v-model="form.setup_code" class="input" :class="{ invalid: errors.setup_code }" autocomplete="off" spellcheck="false" autofocus />
      </Field>
      <Field label="Adresse e-mail" :error="errors.email" for="em"><input id="em" v-model="form.email" class="input" :class="{ invalid: errors.email }" type="email" autocomplete="username" :autofocus="!setup" /></Field>
      <Field label="Mot de passe" :error="errors.password" :hint="setup ? 'Au moins 8 caractères.' : undefined" for="pw">
        <input id="pw" v-model="form.password" class="input" :class="{ invalid: errors.password }" type="password" :autocomplete="setup ? 'new-password' : 'current-password'" />
      </Field>
      <p v-if="message" class="alert error" role="alert">{{ message }}</p>
      <button class="btn primary" style="height: 44px" :disabled="busy">{{ busy ? 'Patientez…' : setup ? 'Créer le compte et commencer' : 'Se connecter' }}</button>
    </form>
  </main>
</template>

<style scoped>
.login { min-height: 100%; display: grid; place-items: center; padding: 24px; background: linear-gradient(160deg, var(--green-soft), var(--bg) 55%); }
.box { width: min(440px, 100%); display: flex; flex-direction: column; gap: 18px; padding: 32px; border-radius: 24px; }
.logo { height: 38px; width: auto; align-self: flex-start; }
</style>
