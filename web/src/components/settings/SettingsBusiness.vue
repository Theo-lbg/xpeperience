<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { ApiError, api } from '../../lib/api'
import { useForm } from '../../lib/forms'
import { boot, can, confirmAction, notify, refsOf, reloadBoot } from '../../lib/store'
import type { RefItem, RefListCode, Settings } from '../../lib/types'
import Field from '../Field.vue'
import Icon from '../Icon.vue'
import Modal from '../Modal.vue'

const canEdit = can('settings', 'update')
const b = boot.value!
const COLORS = ['green', 'blue', 'orange', 'red', 'purple', 'teal', 'yellow', 'gray']
const list = ref<RefListCode>('need_state')
const meta = computed(() => b.lists.find((l) => l.list === list.value)!)
const items = computed(() => refsOf(list.value, true))

const editing = ref<{ item?: RefItem } | null>(null)
const f = reactive({ label: '', category: '' as string, color: '' as string, active: true })
const { errors, message, busy, submit } = useForm()

function open(item?: RefItem) {
  Object.assign(f, { label: item?.label ?? '', category: item?.category ?? meta.value.categories[0]?.code ?? '', color: item?.color ?? '', active: item?.active ?? true })
  errors.value = {}
  message.value = ''
  editing.value = { item }
}
async function saveItem() {
  const it = editing.value?.item
  const body = { label: f.label, category: meta.value.categories.length ? f.category : null, color: f.color || null, active: f.active }
  const ok = await submit(() => (it ? api.put(`/api/refs/item/${it.id}`, body) : api.post(`/api/refs/${list.value}`, body)), ['label', 'category', 'color'])
  if (ok) {
    notify(it ? 'Élément modifié.' : 'Élément ajouté.')
    editing.value = null
    await reloadBoot()
  }
}
async function removeItem(it: RefItem) {
  if (!(await confirmAction('Supprimer cet élément ?', `« ${it.label} » sera supprimé. S'il est déjà utilisé, désactivez-le plutôt.`, 'Supprimer', true))) return
  try {
    await api.del(`/api/refs/item/${it.id}`)
    notify('Élément supprimé.')
    await reloadBoot()
  } catch (e) {
    notify(e instanceof ApiError ? e.message : 'Suppression impossible.', 'error')
  }
}
async function move(it: RefItem, dir: -1 | 1) {
  const ids = items.value.map((i) => i.id)
  const i = ids.indexOf(it.id)
  const j = i + dir
  if (j < 0 || j >= ids.length) return
  ;[ids[i], ids[j]] = [ids[j]!, ids[i]!]
  await api.post(`/api/refs/${list.value}/order`, { ids })
  await reloadBoot()
}
const catLabel = (c: string | null) => meta.value.categories.find((x) => x.code === c)?.label ?? ''

// ── Règles de gestion ──
const rules = reactive({ business: { ...b.settings.business }, required_fields: JSON.parse(JSON.stringify(b.settings.required_fields)) as Record<string, string[]> })
const ENTITY_LABEL: Record<string, string> = { company: 'Société', contact: 'Contact', need: 'Besoin', resource: 'Ressource', project: 'Projet', delivery: 'Prestation' }
const entities = Object.keys(ENTITY_LABEL)
const isReq = (e: string, fld: string) => rules.required_fields[e]?.includes(fld) ?? false
function toggleReq(e: string, fld: string) {
  const set = new Set(rules.required_fields[e] ?? [])
  set.has(fld) ? set.delete(fld) : set.add(fld)
  rules.required_fields = { ...rules.required_fields, [e]: [...set] }
}
const rulesForm = useForm()
async function saveRules() {
  const ok = await rulesForm.submit(() => api.put<Settings>('/api/settings', { ...b.settings, ...rules }))
  if (ok) {
    await reloadBoot()
    notify('Règles de gestion enregistrées.')
  }
}
</script>

<template>
  <div class="stack">
    <section class="card stack">
      <div>
        <h2>États, types et listes</h2>
        <p class="muted small" style="margin-top: 4px">Adaptez les listes à votre façon de travailler. La catégorie d'un état (en cours, gagné, perdu…) pilote les règles métier ; son libellé reste libre.</p>
      </div>
      <div class="row wrap">
        <select v-model="list" class="select" style="width: auto; min-width: 260px" aria-label="Liste à modifier">
          <option v-for="l in b.lists" :key="l.list" :value="l.list">{{ l.label }}</option>
        </select>
        <button v-if="canEdit" class="btn primary small" type="button" @click="open()"><Icon name="plus" /> Ajouter</button>
      </div>
      <div class="table-wrap">
        <table class="table">
          <tbody>
            <tr v-for="(it, i) in items" :key="it.id" :class="{ 'archived-row': !it.active }">
              <td style="width: 90px" class="nowrap">
                <template v-if="canEdit">
                  <button class="btn ghost small icon" type="button" aria-label="Monter" :disabled="i === 0" @click="move(it, -1)"><Icon name="down" style="transform: rotate(180deg)" /></button>
                  <button class="btn ghost small icon" type="button" aria-label="Descendre" :disabled="i === items.length - 1" @click="move(it, 1)"><Icon name="down" /></button>
                </template>
              </td>
              <td><span class="pill" :class="it.color ?? 'gray'">{{ it.label }}</span></td>
              <td class="muted">{{ catLabel(it.category) }}</td>
              <td><span v-if="!it.active" class="faint small">Désactivé</span></td>
              <td class="right nowrap">
                <template v-if="canEdit">
                  <button class="btn ghost small icon" type="button" aria-label="Modifier" @click="open(it)"><Icon name="edit" /></button>
                  <button class="btn ghost small icon" type="button" aria-label="Supprimer" @click="removeItem(it)"><Icon name="trash" /></button>
                </template>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <form class="card stack" @submit.prevent="saveRules" novalidate>
      <div><h2>Règles de gestion</h2><p class="muted small" style="margin-top: 4px">Décidez ce qui se passe quand un positionnement est gagné, et quels champs facultatifs deviennent obligatoires.</p></div>
      <div class="form-grid">
        <Field label="Quand un positionnement passe « gagné »" for="rw">
          <select id="rw" v-model="rules.business.positioning_won_action" class="select" :disabled="!canEdit">
            <option value="ask">Proposer de créer le projet</option><option value="auto">Créer le projet automatiquement</option><option value="none">Ne rien faire</option>
          </select>
        </Field>
        <label class="check" style="align-self: end; padding-bottom: 8px"><input v-model="rules.business.mark_need_won" type="checkbox" :disabled="!canEdit" /><span>Passer le besoin à « gagné » quand son projet est créé</span></label>
      </div>
      <div class="req-grid">
        <fieldset v-for="e in entities" :key="e" class="req">
          <legend>{{ ENTITY_LABEL[e] }}</legend>
          <label v-for="o in b.optional_fields.filter((x) => x.entity === e)" :key="o.field" class="check small">
            <input type="checkbox" :checked="isReq(e, o.field)" :disabled="!canEdit" @change="toggleReq(e, o.field)" /><span>{{ o.label }}</span>
          </label>
        </fieldset>
      </div>
      <p class="alert">Le coût journalier moyen (CJM) n'apparaît pas ici volontairement : il reste toujours facultatif et ne bloque jamais un positionnement, une signature ou la création d'un projet.</p>
      <p v-if="rulesForm.message.value" class="alert error" role="alert">{{ rulesForm.message.value }}</p>
      <div v-if="canEdit"><button class="btn primary" :disabled="rulesForm.busy.value">Enregistrer les règles</button></div>
    </form>
  </div>

  <Modal v-if="editing" :title="editing.item ? 'Modifier l’élément' : 'Nouvel élément'" size="narrow" :busy="busy" @close="editing = null">
    <form id="ref-form" class="stack" @submit.prevent="saveItem" novalidate>
      <Field label="Libellé" required :error="errors.label" for="rl"><input id="rl" v-model="f.label" class="input" autofocus /></Field>
      <Field v-if="meta.categories.length" label="Catégorie" :error="errors.category" hint="Détermine le comportement de l'application." for="rc">
        <select id="rc" v-model="f.category" class="select"><option v-for="c in meta.categories" :key="c.code" :value="c.code">{{ c.label }}</option></select>
      </Field>
      <Field label="Couleur" :error="errors.color">
        <div class="row wrap"><button v-for="c in ['', ...COLORS]" :key="c" type="button" class="pill" :class="[c || 'gray', { sel: f.color === c }]" @click="f.color = c">{{ c || 'aucune' }}</button></div>
      </Field>
      <label v-if="editing.item" class="check"><input v-model="f.active" type="checkbox" /><span>Actif (proposé dans les formulaires)</span></label>
      <p v-if="message" class="alert error" role="alert">{{ message }}</p>
    </form>
    <template #footer>
      <button class="btn" type="button" :disabled="busy" @click="editing = null">Annuler</button>
      <button class="btn primary" form="ref-form" :disabled="busy">Enregistrer</button>
    </template>
  </Modal>
</template>

<style scoped>
.req-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(230px, 1fr)); gap: 14px; }
.req { border: 1px solid var(--line); border-radius: 12px; padding: 10px 14px 12px; display: flex; flex-direction: column; gap: 8px; margin: 0; }
.req legend { font-weight: 650; padding: 0 6px; }
.pill.sel { outline: 2px solid var(--black); outline-offset: 1px; }
button.pill { border: 0; cursor: pointer; }
</style>
