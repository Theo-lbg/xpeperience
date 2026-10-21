import { computed, reactive, ref } from 'vue'
import { api } from './api'
import type { Action, Area, Bootstrap, Id, Named, RefItem, RefListCode } from './types'

export const boot = ref<Bootstrap | null>(null)

export const session = reactive({ loaded: false, setupRequired: false, setupCodeRequired: false, authenticated: false, appName: 'Boond by XPEHO' })

export async function loadSession() {
  const s = await api.get<{ setup_required: boolean; setup_code_required: boolean; authenticated: boolean; app_name: string }>('/api/auth/status')
  session.setupRequired = s.setup_required
  session.setupCodeRequired = s.setup_code_required
  session.authenticated = s.authenticated
  session.appName = s.app_name
  boot.value = s.authenticated ? await api.get<Bootstrap>('/api/bootstrap') : null
  session.loaded = true
  if (boot.value) applyPreferences()
}

/** Recharge les réglages, listes et droits (après une modification dans les réglages). */
export async function reloadBoot() {
  boot.value = await api.get<Bootstrap>('/api/bootstrap')
  session.appName = boot.value.settings.app_name
  applyPreferences()
}

export function applyPreferences() {
  document.documentElement.dataset.density = boot.value?.me.preferences.density ?? 'comfortable'
  document.title = session.appName
}

export const me = computed(() => boot.value?.me.user)
export const settings = computed(() => boot.value!.settings)

export function can(area: Area, action: Action = 'read'): boolean {
  const m = boot.value?.me
  if (!m) return false
  return m.user.is_admin || (m.permissions[area]?.includes(action) ?? false)
}

// ───────────────────────── Listes paramétrables ─────────────────────────

export function refsOf(list: RefListCode, withInactive = false): RefItem[] {
  return (boot.value?.refs ?? []).filter((r) => r.list === list && (withInactive || r.active)).sort((a, b) => a.position - b.position)
}

export function refItem(id: Id | null | undefined): RefItem | undefined {
  return id == null ? undefined : boot.value?.refs.find((r) => r.id === id)
}

export const refLabel = (id: Id | null | undefined) => refItem(id)?.label ?? ''
export const refCategory = (id: Id | null | undefined) => refItem(id)?.category ?? null

export function firstRef(list: RefListCode, category?: string): Id | null {
  return refsOf(list).find((r) => !category || r.category === category)?.id ?? null
}

export const users = computed<Named[]>(() => boot.value?.users ?? [])

export function isRequired(entity: string, field: string): boolean {
  return boot.value?.settings.required_fields[entity]?.includes(field) ?? false
}

// ───────────────────────────── Notifications ─────────────────────────────

export interface Toast { id: number; text: string; kind: 'success' | 'error' | 'info'; to?: string; action?: string }
export const toasts = ref<Toast[]>([])
let toastId = 0

export function notify(text: string, kind: Toast['kind'] = 'success', extra: { to?: string; action?: string } = {}) {
  const id = ++toastId
  toasts.value.push({ id, text, kind, ...extra })
  setTimeout(() => dismissToast(id), kind === 'error' ? 8000 : 5000)
}

export function dismissToast(id: number) {
  toasts.value = toasts.value.filter((t) => t.id !== id)
}

// ───────────────────────────── Confirmation ─────────────────────────────

export interface ConfirmRequest {
  title: string; message: string; confirmLabel: string; danger: boolean
  resolve: (ok: boolean) => void
}
export const confirmRequest = ref<ConfirmRequest | null>(null)

export function confirmAction(title: string, message: string, confirmLabel = 'Confirmer', danger = false): Promise<boolean> {
  return new Promise((resolve) => {
    confirmRequest.value = { title, message, confirmLabel, danger, resolve }
  })
}
