-- Schéma initial de XpePerience.
-- Dates : 'AAAA-MM-JJ'. Horodatages : RFC 3339 UTC. Montants : euros (REAL), toujours facultatifs.

CREATE TABLE roles (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE COLLATE NOCASE,
    description TEXT,
    is_admin    INTEGER NOT NULL DEFAULT 0,
    permissions TEXT NOT NULL DEFAULT '{}',
    created_at  TEXT NOT NULL
);

CREATE TABLE users (
    id            INTEGER PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE COLLATE NOCASE,
    first_name    TEXT NOT NULL,
    last_name     TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role_id       INTEGER NOT NULL REFERENCES roles(id),
    active        INTEGER NOT NULL DEFAULT 1,
    preferences   TEXT NOT NULL DEFAULT '{}',
    created_at    TEXT NOT NULL,
    last_login_at TEXT
);

CREATE TABLE sessions (
    token_hash TEXT PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE TABLE api_tokens (
    id           INTEGER PRIMARY KEY,
    user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    prefix       TEXT NOT NULL,
    token_hash   TEXT NOT NULL UNIQUE,
    created_at   TEXT NOT NULL,
    last_used_at TEXT,
    revoked_at   TEXT
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE ref_items (
    id       INTEGER PRIMARY KEY,
    list     TEXT NOT NULL,
    label    TEXT NOT NULL,
    category TEXT,
    color    TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    active   INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX ref_items_list ON ref_items(list, position);

CREATE TABLE companies (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    type_id     INTEGER REFERENCES ref_items(id),
    sector_id   INTEGER REFERENCES ref_items(id),
    website     TEXT,
    phone       TEXT,
    email       TEXT,
    address     TEXT,
    postal_code TEXT,
    city        TEXT,
    country     TEXT,
    notes       TEXT,
    owner_id    INTEGER REFERENCES users(id) ON DELETE SET NULL,
    archived_at TEXT,
    created_by  INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE contacts (
    id           INTEGER PRIMARY KEY,
    company_id   INTEGER REFERENCES companies(id),
    first_name   TEXT,
    last_name    TEXT NOT NULL,
    job_title    TEXT,
    email        TEXT,
    phone        TEXT,
    mobile       TEXT,
    linkedin_url TEXT,
    location     TEXT,
    notes        TEXT,
    owner_id     INTEGER REFERENCES users(id) ON DELETE SET NULL,
    source       TEXT NOT NULL DEFAULT 'manual',
    imported_at  TEXT,
    archived_at  TEXT,
    created_by   INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);
CREATE INDEX contacts_company ON contacts(company_id);

CREATE TABLE resources (
    id                INTEGER PRIMARY KEY,
    kind              TEXT NOT NULL CHECK (kind IN ('candidate', 'consultant')),
    first_name        TEXT NOT NULL,
    last_name         TEXT NOT NULL,
    title             TEXT,
    email             TEXT,
    phone             TEXT,
    location          TEXT,
    linkedin_url      TEXT,
    skills            TEXT NOT NULL DEFAULT '[]',
    experience_years  REAL,
    experience        TEXT,
    availability_date TEXT,
    availability_note TEXT,
    contract_type_id  INTEGER REFERENCES ref_items(id),
    daily_cost        REAL,
    notes             TEXT,
    owner_id          INTEGER REFERENCES users(id) ON DELETE SET NULL,
    archived_at       TEXT,
    created_by        INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);

CREATE TABLE resource_files (
    id          INTEGER PRIMARY KEY,
    resource_id INTEGER NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL,
    filename    TEXT NOT NULL,
    stored_name TEXT NOT NULL UNIQUE,
    mime        TEXT NOT NULL,
    size        INTEGER NOT NULL,
    uploaded_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    uploaded_at TEXT NOT NULL
);

CREATE TABLE projects (
    id                INTEGER PRIMARY KEY,
    reference         TEXT,
    title             TEXT NOT NULL,
    company_id        INTEGER REFERENCES companies(id),
    contact_id        INTEGER REFERENCES contacts(id),
    type_id           INTEGER REFERENCES ref_items(id),
    state_id          INTEGER REFERENCES ref_items(id),
    manager_id        INTEGER REFERENCES users(id) ON DELETE SET NULL,
    start_date        TEXT,
    end_date          TEXT,
    forecast_end_date TEXT,
    need_id           INTEGER,
    notes             TEXT,
    archived_at       TEXT,
    created_by        INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);
CREATE INDEX projects_company ON projects(company_id);

CREATE TABLE needs (
    id              INTEGER PRIMARY KEY,
    title           TEXT NOT NULL,
    company_id      INTEGER REFERENCES companies(id),
    contact_id      INTEGER NOT NULL REFERENCES contacts(id),
    description     TEXT,
    skills          TEXT NOT NULL DEFAULT '[]',
    type_id         INTEGER REFERENCES ref_items(id),
    state_id        INTEGER NOT NULL REFERENCES ref_items(id),
    origin_id       INTEGER REFERENCES ref_items(id),
    budget          REAL,
    duration_months REAL,
    planned_days    REAL,
    start_date      TEXT,
    location        TEXT,
    owner_id        INTEGER REFERENCES users(id) ON DELETE SET NULL,
    project_id      INTEGER REFERENCES projects(id) ON DELETE SET NULL,
    archived_at     TEXT,
    created_by      INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);
CREATE INDEX needs_company ON needs(company_id);
CREATE INDEX needs_contact ON needs(contact_id);

CREATE TABLE positionings (
    id          INTEGER PRIMARY KEY,
    need_id     INTEGER NOT NULL REFERENCES needs(id) ON DELETE CASCADE,
    resource_id INTEGER NOT NULL REFERENCES resources(id),
    state_id    INTEGER NOT NULL REFERENCES ref_items(id),
    start_date  TEXT,
    end_date    TEXT,
    daily_rate  REAL,
    days        REAL,
    daily_cost  REAL,
    notes       TEXT,
    project_id  INTEGER REFERENCES projects(id) ON DELETE SET NULL,
    created_by  INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    UNIQUE (need_id, resource_id)
);
CREATE INDEX positionings_resource ON positionings(resource_id);

CREATE TABLE deliveries (
    id          INTEGER PRIMARY KEY,
    project_id  INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    resource_id INTEGER REFERENCES resources(id),
    type_id     INTEGER REFERENCES ref_items(id),
    title       TEXT,
    start_date  TEXT NOT NULL,
    end_date    TEXT,
    daily_rate  REAL,
    sold_days   REAL,
    daily_cost  REAL,
    notes       TEXT,
    positioning_id INTEGER REFERENCES positionings(id) ON DELETE SET NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
CREATE INDEX deliveries_project ON deliveries(project_id);
CREATE INDEX deliveries_resource ON deliveries(resource_id);

CREATE TABLE delivery_extensions (
    id          INTEGER PRIMARY KEY,
    delivery_id INTEGER NOT NULL REFERENCES deliveries(id) ON DELETE CASCADE,
    status      TEXT NOT NULL CHECK (status IN ('signed', 'forecast')),
    start_date  TEXT NOT NULL,
    end_date    TEXT NOT NULL,
    days        REAL,
    daily_rate  REAL,
    notes       TEXT,
    created_at  TEXT NOT NULL
);
CREATE INDEX delivery_extensions_delivery ON delivery_extensions(delivery_id);

CREATE TABLE activities (
    id             INTEGER PRIMARY KEY,
    kind           TEXT NOT NULL,
    occurred_at    TEXT NOT NULL,
    comment        TEXT NOT NULL DEFAULT '',
    due_date       TEXT,
    done_at        TEXT,
    company_id     INTEGER REFERENCES companies(id) ON DELETE CASCADE,
    contact_id     INTEGER REFERENCES contacts(id) ON DELETE CASCADE,
    need_id        INTEGER REFERENCES needs(id) ON DELETE CASCADE,
    resource_id    INTEGER REFERENCES resources(id) ON DELETE CASCADE,
    positioning_id INTEGER REFERENCES positionings(id) ON DELETE CASCADE,
    project_id     INTEGER REFERENCES projects(id) ON DELETE CASCADE,
    owner_id       INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_by     INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at     TEXT NOT NULL
);
CREATE INDEX activities_company ON activities(company_id);
CREATE INDEX activities_contact ON activities(contact_id);
CREATE INDEX activities_need ON activities(need_id);
CREATE INDEX activities_due ON activities(due_date) WHERE done_at IS NULL;

CREATE TABLE history (
    id           INTEGER PRIMARY KEY,
    entity       TEXT NOT NULL,
    entity_id    INTEGER NOT NULL,
    entity_label TEXT NOT NULL,
    action       TEXT NOT NULL,
    changes      TEXT NOT NULL DEFAULT '[]',
    link_entity  TEXT,
    link_id      INTEGER,
    user_id      INTEGER REFERENCES users(id) ON DELETE SET NULL,
    at           TEXT NOT NULL
);
CREATE INDEX history_entity ON history(entity, entity_id);
CREATE INDEX history_at ON history(at);
