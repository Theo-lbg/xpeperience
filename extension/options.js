const $ = (id) => document.getElementById(id)

function message(kind, text) {
  const p = document.createElement('p')
  p.className = `notice ${kind}`
  p.setAttribute('role', kind === 'error' ? 'alert' : 'status')
  p.textContent = text
  $('feedback').replaceChildren(p)
}

async function init() {
  const cfg = await loadConfig()
  $('address').value = cfg.address
  $('token').value = cfg.token
}

function read() {
  $('address-err').textContent = ''
  $('token-err').textContent = ''
  const address = normalizeAddress($('address').value)
  const token = $('token').value.trim()
  let ok = true
  if (!address) {
    $('address-err').textContent = 'Adresse invalide : http://127.0.0.1:47821 (application de bureau) ou https://votre-serveur.'
    ok = false
  }
  if (!token.startsWith('bxp_')) {
    $('token-err').textContent = 'Collez le jeton créé dans Boond by XPEHO (il commence par bxp_).'
    ok = false
  }
  return ok ? { address, token } : null
}

async function test(cfg) {
  try {
    const s = await call('/api/ext/status', { config: cfg })
    message('ok', `Connexion réussie : ${s.user} sur ${s.app_name}.`)
    return true
  } catch (e) {
    message('error', e.message)
    return false
  }
}

$('test').addEventListener('click', async () => {
  const cfg = read()
  if (!cfg) return
  if (!(await ensureAccess(cfg.address))) return message('error', 'Autorisez l’accès à ce serveur pour tester la connexion.')
  await test(cfg)
})

$('form').addEventListener('submit', async (e) => {
  e.preventDefault()
  const cfg = read()
  if (!cfg) return
  // Doit être appelé directement depuis le clic : Chrome exige un geste de l'utilisateur.
  if (!(await ensureAccess(cfg.address))) return message('error', 'Sans votre autorisation d’accéder à ce serveur, l’extension ne peut pas s’y connecter.')
  await chrome.storage.local.set(cfg)
  if (!(await test(cfg))) return
  message('ok', 'Réglages enregistrés. Connexion vérifiée.')
})

init()
