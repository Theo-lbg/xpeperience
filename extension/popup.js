// Fenêtre de l'extension : bouton explicite → aperçu modifiable → validation → import.

const view = document.getElementById('view')
let status = null
let tab = null

function el(tag, attrs = {}, ...children) {
  const node = document.createElement(tag)
  for (const [k, v] of Object.entries(attrs)) {
    if (k === 'class') node.className = v
    else if (k.startsWith('on')) node.addEventListener(k.slice(2), v)
    else if (v !== false && v != null) node.setAttribute(k, v === true ? '' : v)
  }
  for (const c of children.flat()) if (c != null) node.append(c.nodeType ? c : document.createTextNode(c))
  return node
}

function show(...nodes) {
  view.replaceChildren(...nodes)
}

function notice(kind, text) {
  return el('p', { class: `notice ${kind}`, role: kind === 'error' ? 'alert' : 'status' }, text)
}

const isProfile = (url) => /^https:\/\/([a-z0-9-]+\.)?linkedin\.com\/in\//i.test(url || '')

async function start() {
  const cfg = await loadConfig()
  ;[tab] = await chrome.tabs.query({ active: true, currentWindow: true })

  if (!cfg.token) {
    return show(
      notice('info', 'Reliez l’extension à Boond by XPEHO : créez un jeton dans Réglages › Extension LinkedIn.'),
      el('button', { class: 'btn primary', onclick: () => chrome.runtime.openOptionsPage() }, 'Configurer l’extension'),
    )
  }
  show(el('p', { class: 'muted' }, 'Connexion à Boond by XPEHO…'))
  try {
    status = await call('/api/ext/status', { config: cfg })
  } catch (e) {
    return show(
      notice('error', e.message),
      el('button', { class: 'btn', onclick: start }, 'Réessayer'),
      el('button', { class: 'btn ghost', onclick: () => chrome.runtime.openOptionsPage() }, 'Réglages de l’extension'),
    )
  }
  if (!status.enabled) return show(notice('warn', 'L’import LinkedIn est désactivé dans les réglages de Boond by XPEHO.'))
  if (!status.can_import) return show(notice('warn', 'Votre rôle ne permet pas de créer des contacts.'))
  home()
}

function home() {
  const onProfile = isProfile(tab?.url)
  show(
    el('p', { class: 'muted' }, `Connecté : ${status.user}`),
    onProfile
      ? el('p', {}, 'Profil LinkedIn détecté. Les informations visibles seront lues uniquement après votre clic.')
      : notice('info', 'Ouvrez un profil LinkedIn (linkedin.com/in/…) pour préremplir le contact, ou saisissez-le à la main.'),
    el('button', { class: 'btn primary big', onclick: () => capture(onProfile) }, 'Ajouter à Boond by XPEHO'),
    !onProfile && el('p', { class: 'hint' }, 'Aucune donnée n’est lue en dehors des profils LinkedIn.'),
  )
}

async function capture(fromPage) {
  let profile = { first_name: '', last_name: '', job_title: '', company_name: '', location: '', linkedin_url: '' }
  let warning = ''
  if (fromPage) {
    try {
      const [res] = await chrome.scripting.executeScript({ target: { tabId: tab.id }, func: readLinkedInProfile })
      profile = { ...profile, ...res.result }
      if (!profile.last_name) warning = 'Le nom n’a pas pu être lu automatiquement : saisissez-le.'
      else if (!profile.job_title && !profile.company_name) warning = 'Certaines informations n’ont pas pu être lues : complétez-les si besoin.'
    } catch {
      warning = 'La page n’a pas pu être lue. Saisissez les informations à la main.'
    }
  }
  preview(profile, warning)
}

async function preview(profile, warning) {
  show(el('p', { class: 'muted' }, 'Recherche de doublons…'))
  let lookup = { duplicates: [], exact_company: null, companies: [] }
  if (profile.last_name) {
    try {
      lookup = await call('/api/ext/lookup', { method: 'POST', body: profile })
    } catch (e) {
      return show(notice('error', e.message), el('button', { class: 'btn', onclick: home }, 'Retour'))
    }
  }
  renderPreview(profile, warning, lookup)
}

function field(label, id, value, opts = {}) {
  return el(
    'label',
    { class: 'field', for: id },
    el('span', {}, label, opts.required ? el('b', { 'aria-hidden': 'true' }, ' *') : null),
    el('input', { id, type: 'text', value, autocomplete: 'off', ...(opts.required ? { required: true } : {}) }),
    el('small', { class: 'err', id: `${id}-err`, role: 'alert' }),
  )
}

function renderPreview(profile, warning, lookup) {
  const companyChoices = []
  if (lookup.exact_company) companyChoices.push({ value: `existing:${lookup.exact_company.id}`, label: `Société existante : ${lookup.exact_company.name}` })
  for (const c of lookup.companies) if (c.id !== lookup.exact_company?.id) companyChoices.push({ value: `existing:${c.id}`, label: `Société existante : ${c.name}` })
  const canCreate = status.allow_company_creation && profile.company_name && !lookup.exact_company
  if (canCreate) companyChoices.push({ value: 'new', label: `Créer la société « ${profile.company_name} »` })
  companyChoices.push({ value: 'none', label: 'Aucune société' })
  const defaultChoice = companyChoices[0].value

  const select = el('select', { id: 'company-choice' }, companyChoices.map((c) => el('option', { value: c.value }, c.label)))
  select.value = defaultChoice
  const newName = el('input', { id: 'company-name', type: 'text', value: profile.company_name, 'aria-label': 'Nom de la société' })
  const found = el('div', { class: 'found' })
  let extra = []

  const search = el('input', { type: 'search', placeholder: 'Chercher une autre société…', 'aria-label': 'Chercher une société' })
  let timer
  search.addEventListener('input', () => {
    clearTimeout(timer)
    timer = setTimeout(async () => {
      if (search.value.trim().length < 2) return
      try {
        extra = await call(`/api/ext/companies?q=${encodeURIComponent(search.value.trim())}`)
        for (const c of extra) {
          if (![...select.options].some((o) => o.value === `existing:${c.id}`)) select.add(new Option(`Société existante : ${c.name}`, `existing:${c.id}`), 0)
        }
        if (extra[0]) select.value = `existing:${extra[0].id}`
      } catch {
        /* la recherche reste facultative */
      }
    }, 250)
  })

  let duplicateMode = 'create'
  let duplicateId = null
  const dupBox = el('div', { class: 'dups' })
  if (lookup.duplicates.length) {
    duplicateMode = 'update'
    duplicateId = lookup.duplicates[0].id
    dupBox.append(
      notice('warn', 'Ce contact existe peut-être déjà dans Boond by XPEHO :'),
      ...lookup.duplicates.map((d, i) =>
        el(
          'label',
          { class: 'radio' },
          el('input', { type: 'radio', name: 'dup', value: String(d.id), ...(i === 0 ? { checked: true } : {}), onchange: () => { duplicateMode = 'update'; duplicateId = d.id } }),
          el('span', {}, el('b', {}, d.name), [d.job_title, d.company].filter(Boolean).length ? ` — ${[d.job_title, d.company].filter(Boolean).join(' · ')}` : '', el('small', {}, ` (${d.reason})`)),
        ),
      ),
      el('label', { class: 'radio' }, el('input', { type: 'radio', name: 'dup', value: 'new', onchange: () => { duplicateMode = 'create'; duplicateId = null } }), el('span', {}, 'Créer un nouveau contact malgré tout')),
    )
  }

  const fields = {
    first_name: field('Prénom', 'f-first', profile.first_name),
    last_name: field('Nom', 'f-last', profile.last_name, { required: true }),
    job_title: field('Poste', 'f-title', profile.job_title),
    location: field('Localisation', 'f-loc', profile.location),
    linkedin_url: field('Profil LinkedIn', 'f-url', profile.linkedin_url),
  }
  const get = (id) => document.getElementById(id).value.trim()
  const feedback = el('div', { id: 'feedback' })
  const submit = el('button', { class: 'btn primary', type: 'submit' }, lookup.duplicates.length ? 'Valider' : 'Confirmer l’import')

  const form = el(
    'form',
    {
      class: 'form',
      novalidate: true,
      onsubmit: async (ev) => {
        ev.preventDefault()
        feedback.replaceChildren()
        document.querySelectorAll('.err').forEach((n) => (n.textContent = ''))
        if (!get('f-last')) {
          document.getElementById('f-last-err').textContent = 'Indiquez au moins le nom.'
          return document.getElementById('f-last').focus()
        }
        const choice = select.value
        const company = choice.startsWith('existing:') ? { mode: 'existing', id: Number(choice.slice(9)) } : choice === 'new' ? { mode: 'new', name: get('company-name') } : { mode: 'none' }
        const body = {
          profile: { first_name: get('f-first'), last_name: get('f-last'), job_title: get('f-title'), company_name: get('company-name'), location: get('f-loc'), linkedin_url: get('f-url') },
          company,
          on_duplicate: duplicateMode,
          duplicate_id: duplicateId,
        }
        submit.disabled = true
        submit.textContent = 'Import en cours…'
        try {
          const res = await call('/api/ext/import', { method: 'POST', body })
          success(res)
        } catch (e) {
          submit.disabled = false
          submit.textContent = 'Réessayer'
          const bad = e.body?.fields || []
          const map = { first_name: 'f-first', last_name: 'f-last', job_title: 'f-title', location: 'f-loc', linkedin_url: 'f-url' }
          for (const f of bad) {
            const node = document.getElementById(`${map[f.field] || ''}-err`)
            if (node) node.textContent = f.message
          }
          feedback.replaceChildren(notice('error', bad.length ? 'Une correction est nécessaire (voir les champs en rouge).' : e.message))
        }
      },
    },
    warning ? notice('warn', warning) : null,
    el('p', { class: 'muted' }, 'Vérifiez et complétez avant d’enregistrer.'),
    ...Object.values(fields),
    el('label', { class: 'field' }, el('span', {}, 'Société'), select, newName, search),
    dupBox,
    feedback,
    el('div', { class: 'actions' }, el('button', { class: 'btn', type: 'button', onclick: home }, 'Annuler'), submit),
  )
  // Le champ de nom de société n'est utile que pour une création.
  const syncName = () => (newName.hidden = select.value !== 'new')
  select.addEventListener('change', syncName)
  syncName()
  show(form)
  document.getElementById(profile.last_name ? 'f-title' : 'f-last').focus()
}

function success(res) {
  const verb = res.status === 'updated' ? 'mis à jour' : 'ajouté'
  show(
    notice('ok', `${res.contact.name} a été ${verb} dans Boond by XPEHO${res.company ? ` (${res.company.name})` : ''}.`),
    el('button', { class: 'btn primary', onclick: () => chrome.tabs.create({ url: res.url }) }, 'Ouvrir la fiche'),
    el('button', { class: 'btn ghost', onclick: home }, 'Ajouter un autre contact'),
  )
}

start()
