import { boot } from './store'
import type { EntityKind, HistoryEntry } from './types'

const MONTHS = ['janv.', 'févr.', 'mars', 'avr.', 'mai', 'juin', 'juil.', 'août', 'sept.', 'oct.', 'nov.', 'déc.']

/** Date `AAAA-MM-JJ` (ou horodatage) selon le format choisi dans les réglages. */
export function fmtDate(iso: string | null | undefined): string {
  if (!iso) return '—'
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(iso)
  if (!m) return iso
  const [, y, mo, d] = m
  switch (boot.value?.settings.date_format) {
    case 'yyyy-MM-dd': return `${y}-${mo}-${d}`
    case 'MM/dd/yyyy': return `${mo}/${d}/${y}`
    case 'd MMM yyyy': return `${Number(d)} ${MONTHS[Number(mo) - 1]} ${y}`
    default: return `${d}/${mo}/${y}`
  }
}

export function fmtDateTime(iso: string | null | undefined): string {
  if (!iso) return '—'
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  const time = d.toLocaleTimeString(boot.value?.settings.locale ?? 'fr-FR', { hour: '2-digit', minute: '2-digit' })
  return `${fmtDate(localIsoDate(d))} · ${time}`
}

export function localIsoDate(d: Date): string {
  const p = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`
}

export const todayIso = () => localIsoDate(new Date())

export function addDays(days: number): string {
  const d = new Date()
  d.setDate(d.getDate() + days)
  return localIsoDate(d)
}

/** « il y a 3 h », « hier »… pour l'activité récente. */
export function fmtAgo(iso: string): string {
  const d = new Date(iso)
  const diff = (Date.now() - d.getTime()) / 1000
  if (diff < 60) return "à l'instant"
  if (diff < 3600) return `il y a ${Math.floor(diff / 60)} min`
  if (diff < 86400) return `il y a ${Math.floor(diff / 3600)} h`
  if (diff < 172800) return 'hier'
  if (diff < 86400 * 14) return `il y a ${Math.floor(diff / 86400)} j`
  return fmtDate(iso)
}

export function fmtMoney(v: number | null | undefined): string {
  if (v == null) return '—'
  const s = boot.value?.settings
  return new Intl.NumberFormat(s?.locale ?? 'fr-FR', { style: 'currency', currency: s?.currency ?? 'EUR', maximumFractionDigits: 2 }).format(v)
}

export function fmtNumber(v: number | null | undefined, unit = ''): string {
  if (v == null) return '—'
  const n = new Intl.NumberFormat(boot.value?.settings.locale ?? 'fr-FR', { maximumFractionDigits: 2 }).format(v)
  return unit ? `${n} ${unit}` : n
}

export function fmtSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} o`
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} Ko`
  return `${(bytes / 1024 / 1024).toFixed(1)} Mo`
}

export const personName = (first: string | null | undefined, last: string) => (first ? `${first} ${last}` : last)

export const initials = (name: string) =>
  name.split(/\s+/).filter(Boolean).slice(0, 2).map((p) => p[0]!.toUpperCase()).join('')

/** Parse un champ numérique facultatif : vide → null. */
export function num(v: string | number | null | undefined): number | null {
  if (v === '' || v == null) return null
  const n = Number(String(v).replace(',', '.'))
  return Number.isFinite(n) ? n : null
}

export const orNull = (v: string | null | undefined) => (v && v.trim() ? v.trim() : null)

// ───────────────────────── Navigation entre objets ─────────────────────────

export function entityRoute(kind: EntityKind, id: number): string {
  switch (kind) {
    case 'company': return `/crm/companies/${id}`
    case 'contact': return `/crm/contacts/${id}`
    case 'need': return `/needs/${id}`
    case 'resource': return `/resources/${id}`
    case 'project': return `/projects/${id}`
    default: return '/'
  }
}

/** Fiche à ouvrir pour une entrée d'historique (un positionnement s'ouvre sur son besoin). */
export function historyRoute(h: HistoryEntry): string | null {
  if (h.action === 'deleted') return null
  if (h.entity === 'positioning' || h.entity === 'delivery') return h.link_entity && h.link_id ? entityRoute(h.link_entity, h.link_id) : null
  return entityRoute(h.entity, h.entity_id)
}

const ENTITY_LABEL: Record<EntityKind, string> = {
  company: 'Société', contact: 'Contact', need: 'Besoin', positioning: 'Positionnement',
  resource: 'Ressource', project: 'Projet', delivery: 'Prestation',
}
export const entityLabel = (k: EntityKind) => ENTITY_LABEL[k]

const ACTION_LABEL: Record<string, string> = {
  created: 'créé', updated: 'modifié', state_changed: 'changement d\'état', archived: 'archivé', restored: 'restauré',
  imported: 'importé depuis LinkedIn', converted: 'converti en projet', deleted: 'supprimé',
}
export const actionLabel = (a: string) => ACTION_LABEL[a] ?? a

export const ACTIVITY_LABEL = { call: 'Appel', meeting: 'Rendez-vous', email: 'E-mail', follow_up: 'Relance', note: 'Note' } as const

export const RESOURCE_KIND_LABEL = { candidate: 'Candidat', consultant: 'Consultant' } as const
export const FILE_KIND_LABEL = { cv: 'CV', skills_file: 'Dossier de compétences', other: 'Autre document' } as const
