<script setup lang="ts">
// Notifications et boîte de confirmation, montées une seule fois dans l'application.
import { useRouter } from 'vue-router'
import { confirmRequest, dismissToast, toasts } from '../lib/store'
import Modal from './Modal.vue'

const router = useRouter()

function answer(ok: boolean) {
  confirmRequest.value?.resolve(ok)
  confirmRequest.value = null
}
</script>

<template>
  <div class="toasts" aria-live="polite">
    <div v-for="t in toasts" :key="t.id" class="toast" :class="t.kind">
      <span class="dot" />
      <span>{{ t.text }}</span>
      <a v-if="t.to" :href="t.to" @click.prevent="router.push(t.to); dismissToast(t.id)">{{ t.action ?? 'Ouvrir' }}</a>
      <button type="button" aria-label="Fermer" @click="dismissToast(t.id)">×</button>
    </div>
  </div>
  <Modal v-if="confirmRequest" :title="confirmRequest.title" size="narrow" @close="answer(false)">
    <p class="muted">{{ confirmRequest.message }}</p>
    <template #footer>
      <button class="btn" type="button" @click="answer(false)">Annuler</button>
      <button class="btn" :class="confirmRequest.danger ? 'danger solid' : 'primary'" type="button" @click="answer(true)">
        {{ confirmRequest.confirmLabel }}
      </button>
    </template>
  </Modal>
</template>
