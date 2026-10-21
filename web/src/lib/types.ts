// Types alignés sur le modèle Rust (crates/boond-domain/src/model.rs).

export type Id = number
export interface Named { id: Id; name: string }

export type Area = 'crm' | 'needs' | 'positionings' | 'resources' | 'costs' | 'projects' | 'settings' | 'users'
export type Action = 'read' | 'create' | 'update' | 'delete'
export type Permissions = Partial<Record<Area, Action[]>>

export interface User {
  id: Id; email: string; first_name: string; last_name: string; role_id: Id; role_name: string
  is_admin: boolean; active: boolean; created_at: string; last_login_at: string | null
}
export interface Preferences { density: 'comfortable' | 'compact'; needs_view: 'list' | 'kanban'; sidebar_collapsed: boolean }
export interface Me { user: User; permissions: Permissions; preferences: Preferences }
export interface Role { id: Id; name: string; description: string | null; is_admin: boolean; permissions: Permissions; users_count: number }
export interface ApiToken { id: Id; name: string; prefix: string; user_id: Id; user_name: string; created_at: string; last_used_at: string | null; revoked: boolean }

export type RefListCode =
  | 'company_type' | 'sector' | 'need_state' | 'need_type' | 'need_origin'
  | 'positioning_state' | 'project_type' | 'project_state' | 'delivery_type' | 'contract_type'
export interface RefItem { id: Id; list: RefListCode; label: string; category: string | null; color: string | null; position: number; active: boolean }
export interface RefListMeta { list: RefListCode; label: string; categories: { code: string; label: string }[] }

export interface Settings {
  app_name: string; locale: string; date_format: string; currency: string
  reminders: { enabled: boolean; default_follow_up_days: number; upcoming_days: number }
  business: { positioning_won_action: 'ask' | 'auto' | 'none'; mark_need_won: boolean }
  extension: { enabled: boolean; allow_company_creation: boolean; add_import_note: boolean }
  required_fields: Record<string, string[]>
}

export interface Bootstrap {
  me: Me; settings: Settings; refs: RefItem[]; lists: RefListMeta[]; users: Named[]
  areas: { area: Area; label: string; actions: Action[] }[]
  optional_fields: { entity: string; field: string; label: string }[]
  locales: [string, string][]; date_formats: string[]; currencies: string[]
  port: number; address: string; version: string
}

export interface Company {
  id: Id; name: string; type_id: Id | null; sector_id: Id | null; website: string | null; phone: string | null
  email: string | null; address: string | null; postal_code: string | null; city: string | null; country: string | null
  notes: string | null; owner: Named | null; archived: boolean; created_at: string; updated_at: string
  contacts_count: number; open_needs_count: number
}
export interface Contact {
  id: Id; company: Named | null; first_name: string | null; last_name: string; job_title: string | null
  email: string | null; phone: string | null; mobile: string | null; linkedin_url: string | null; location: string | null
  notes: string | null; owner: Named | null; source: 'manual' | 'linkedin'; imported_at: string | null
  archived: boolean; created_at: string; updated_at: string
}
export type ActivityKind = 'call' | 'meeting' | 'email' | 'follow_up' | 'note'
export interface Activity {
  id: Id; kind: ActivityKind; occurred_at: string; comment: string; due_date: string | null; done_at: string | null
  company: Named | null; contact: Named | null; need: Named | null; resource: Named | null
  positioning_id: Id | null; project: Named | null; owner: Named | null; created_at: string
}
export interface Need {
  id: Id; title: string; company: Named | null; contact: Named | null; description: string | null; skills: string[]
  type_id: Id | null; state_id: Id; origin_id: Id | null; budget: number | null; duration_months: number | null
  planned_days: number | null; start_date: string | null; location: string | null; owner: Named | null
  project: Named | null; archived: boolean; created_at: string; updated_at: string; positionings_count: number
}
export type ResourceKind = 'candidate' | 'consultant'
export interface Resource {
  id: Id; kind: ResourceKind; first_name: string; last_name: string; title: string | null; email: string | null
  phone: string | null; location: string | null; linkedin_url: string | null; skills: string[]
  experience_years: number | null; experience: string | null; availability_date: string | null
  availability_note: string | null; contract_type_id: Id | null; daily_cost: number | null
  effective_daily_cost: number | null; daily_cost_source: 'contract' | 'manual' | null; notes: string | null
  owner: Named | null; archived: boolean; created_at: string; updated_at: string
  on_mission_until: string | null; on_mission: boolean; open_positionings: number
}
export interface HrContract {
  id: Id; resource_id: Id; contract_type_id: Id | null; start_date: string; end_date: string | null
  daily_cost: number | null; notes: string | null; created_at: string
}
export interface ResourceFile { id: Id; resource_id: Id; kind: 'cv' | 'skills_file' | 'other'; filename: string; mime: string; size: number; uploaded_at: string }
export interface Positioning {
  id: Id; need: Named; company: Named | null; resource: Named; resource_kind: ResourceKind; resource_title: string | null
  state_id: Id; start_date: string | null; end_date: string | null; daily_rate: number | null; days: number | null
  daily_cost: number | null; notes: string | null; project: Named | null; created_at: string; updated_at: string
}
export interface PositioningSaved { positioning: Positioning; suggest_project: boolean; created_project: Named | null }
export interface MatchReason { kind: string; positive: boolean; text: string }
export interface Suggestion { resource: Resource; score: number; reasons: MatchReason[] }
export interface Project {
  id: Id; reference: string | null; title: string; company: Named | null; contact: Named | null; type_id: Id | null
  state_id: Id | null; manager: Named | null; start_date: string | null; end_date: string | null
  forecast_end_date: string | null; dates_from_deliveries: boolean; need: Named | null; notes: string | null
  archived: boolean; created_at: string; updated_at: string; resources: Named[]; deliveries_count: number
}
export interface Extension {
  id: Id; delivery_id: Id; status: 'signed' | 'forecast'; start_date: string; end_date: string
  days: number | null; daily_rate: number | null; notes: string | null; created_at: string
}
export interface Delivery {
  id: Id; project: Named; company: Named | null; resource: Named | null; type_id: Id | null; title: string | null
  start_date: string; end_date: string | null; daily_rate: number | null; sold_days: number | null
  daily_cost: number | null; notes: string | null; extensions: Extension[]
  effective_end_date: string | null; forecast_end_date: string | null; created_at: string; updated_at: string
}
export type EntityKind = 'company' | 'contact' | 'need' | 'positioning' | 'resource' | 'project' | 'delivery'
export interface FieldChange { field: string; from: string | null; to: string | null }
export interface HistoryEntry {
  id: Id; entity: EntityKind; entity_id: Id; entity_label: string; action: string; changes: FieldChange[]
  link_entity: EntityKind | null; link_id: Id | null; user: Named | null; at: string
}
export interface Dashboard {
  counts: { companies: number; contacts: number; open_needs: number; resources: number; candidates: number; consultants: number }
  recent: HistoryEntry[]; reminders: Activity[]
}
export interface SearchHit { kind: EntityKind; id: Id; title: string; subtitle: string | null }
export interface DuplicateContact { id: Id; name: string; company: string | null; job_title: string | null; reason: string }
