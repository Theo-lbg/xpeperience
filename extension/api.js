// Client de l'API de XpePerience. L'extension ne parle qu'à l'application locale.

const DEFAULT_ADDRESS = 'http://127.0.0.1:47821'

async function loadConfig() {
  const { address, token } = await chrome.storage.local.get(['address', 'token'])
  return { address: address || DEFAULT_ADDRESS, token: token || '' }
}

const isLocal = (url) => url.hostname === '127.0.0.1' || url.hostname === 'localhost'

/**
 * Adresse acceptée : l'application de bureau (http, boucle locale) ou un serveur en HTTPS.
 * Le HTTP en clair vers une autre machine est refusé : le jeton circulerait sans chiffrement.
 */
function normalizeAddress(raw) {
  let url
  try {
    url = new URL(raw.trim())
  } catch {
    return null
  }
  if (url.username || url.password) return null
  if (url.protocol === 'http:' && isLocal(url)) return url.origin
  if (url.protocol === 'https:') return url.origin
  return null
}

/** Chrome demande à l'utilisateur d'autoriser l'accès à un serveur distant (une seule adresse, jamais « tous les sites »). */
async function ensureAccess(address) {
  const url = new URL(address)
  if (url.protocol === 'http:' && isLocal(url)) return true
  const origins = [`${url.origin}/*`]
  if (await chrome.permissions.contains({ origins })) return true
  return chrome.permissions.request({ origins })
}

class ApiError extends Error {
  constructor(status, message, body) {
    super(message)
    this.status = status
    this.body = body
  }
}

async function call(path, { method = 'GET', body, config } = {}) {
  const cfg = config || (await loadConfig())
  if (!cfg.token) throw new ApiError(401, 'Aucun jeton configuré.')
  let res
  try {
    res = await fetch(cfg.address + path, {
      method,
      headers: { Authorization: `Bearer ${cfg.token}`, ...(body ? { 'Content-Type': 'application/json' } : {}) },
      body: body ? JSON.stringify(body) : undefined,
    })
  } catch {
    throw new ApiError(0, 'XpePerience ne répond pas. Vérifiez que l’application est ouverte.')
  }
  const data = await res.json().catch(() => null)
  if (!res.ok) {
    const message =
      res.status === 401 ? 'Jeton refusé ou révoqué. Créez-en un nouveau dans Réglages › Extension LinkedIn.' : data?.message || 'Une erreur est survenue.'
    throw new ApiError(res.status, message, data)
  }
  return data
}
