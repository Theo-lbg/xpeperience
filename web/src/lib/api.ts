// Client HTTP : toutes les écritures portent l'en-tête qui protège contre les requêtes tierces.

export interface FieldError { field: string; message: string }

export class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
    public fields: FieldError[] = [],
    public details: any = null,
  ) {
    super(message)
  }

  /** Erreurs de champ sous forme `{ champ: message }`. */
  get fieldMap(): Record<string, string> {
    return Object.fromEntries(this.fields.map((f) => [f.field, f.message]))
  }
}

let onUnauthorized: () => void = () => {}
export function setUnauthorizedHandler(fn: () => void) {
  onUnauthorized = fn
}

function query(params?: Record<string, unknown>): string {
  if (!params) return ''
  const q = new URLSearchParams()
  for (const [k, v] of Object.entries(params)) {
    if (v === undefined || v === null || v === '') continue
    q.set(k, String(v))
  }
  const s = q.toString()
  return s ? `?${s}` : ''
}

async function request<T>(method: string, path: string, body?: unknown, init: RequestInit = {}): Promise<T> {
  const headers: Record<string, string> = { 'x-xpeperience-client': 'web', ...(init.headers as Record<string, string>) }
  let payload: BodyInit | undefined
  if (body instanceof Blob || body instanceof ArrayBuffer) payload = body as BodyInit
  else if (body !== undefined) {
    headers['content-type'] = 'application/json'
    payload = JSON.stringify(body)
  }
  let res: Response
  try {
    res = await fetch(path, { method, headers, body: payload, credentials: 'same-origin' })
  } catch {
    throw new ApiError(0, 'network', "Impossible de joindre l'application. Relancez XpePerience.")
  }
  if (res.status === 204) return undefined as T
  const text = await res.text()
  const data = text ? safeJson(text) : null
  if (!res.ok) {
    const err = new ApiError(res.status, data?.error ?? 'error', data?.message ?? 'Une erreur est survenue.', data?.fields ?? [], data?.details ?? null)
    if (res.status === 401 && !path.startsWith('/api/auth/')) onUnauthorized()
    throw err
  }
  return data as T
}

function safeJson(text: string): any {
  try {
    return JSON.parse(text)
  } catch {
    return null
  }
}

export const api = {
  get: <T>(path: string, params?: Record<string, unknown>) => request<T>('GET', path + query(params)),
  post: <T>(path: string, body: unknown = {}) => request<T>('POST', path, body),
  put: <T>(path: string, body: unknown = {}) => request<T>('PUT', path, body),
  del: <T = void>(path: string) => request<T>('DELETE', path),
  upload: <T>(path: string, file: File, headers: Record<string, string> = {}) =>
    request<T>('POST', path, file, { headers: { 'content-type': file.type, 'x-filename': encodeURIComponent(file.name), ...headers } }),
  putFile: <T>(path: string, file: File) => request<T>('PUT', path, file, { headers: { 'content-type': file.type } }),
}
