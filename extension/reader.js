// Lecture du profil LinkedIn affiché à l'écran.
//
// Cette fonction est injectée UNIQUEMENT quand l'utilisateur clique sur « Ajouter à Boond by XPEHO »
// (permission activeTab). Elle lit le texte déjà affiché sur la page : aucune requête vers LinkedIn,
// aucune navigation, aucun défilement, aucun autre profil. Tout champ introuvable reste vide et
// l'utilisateur le complète à la main dans l'aperçu.
//
// Elle doit rester autonome : elle est sérialisée puis exécutée dans la page.
function readLinkedInProfile() {
  const clean = (s) => (s || '').replace(/\s+/g, ' ').trim()
  const text = (el) => clean(el && el.textContent)
  const meta = (name) => clean(document.querySelector(`meta[property="${name}"], meta[name="${name}"]`)?.getAttribute('content'))

  // Nom : titre principal de la page, sinon titre du document (« (3) Prénom Nom | LinkedIn »).
  let name = text(document.querySelector('main h1, h1'))
  if (!name) name = clean(document.title.replace(/^\(\d+\)\s*/, '').split(/\s+[|·–-]\s+/)[0])
  name = name.replace(/,.*$/, '').replace(/\s*\([^)]*\)\s*$/, '').trim()
  const parts = name.split(' ').filter(Boolean)
  const first_name = parts.length > 1 ? parts[0] : ''
  const last_name = parts.length > 1 ? parts.slice(1).join(' ') : parts[0] || ''

  // Intitulé affiché sous le nom (« Poste chez Société »).
  let headline = text(document.querySelector('main .text-body-medium.break-words, .text-body-medium.break-words'))
  if (!headline) headline = meta('og:description').split(/\s+[·|]\s+/)[0]

  // Société actuelle : bouton « Entreprise actuelle : X » / « Current company: X ».
  let company_name = ''
  const aria = [...document.querySelectorAll('button[aria-label], a[aria-label]')]
    .map((el) => el.getAttribute('aria-label') || '')
    .find((l) => /^(current company|entreprise actuelle|société actuelle)/i.test(l.trim()))
  if (aria) company_name = clean((/[:：]\s*([^.]+?)(?:\.|$)/.exec(aria) || [])[1])

  let job_title = headline
  const sep = /\s+(?:chez|at|@|auprès de)\s+/i.exec(headline)
  if (sep) {
    job_title = clean(headline.slice(0, sep.index))
    if (!company_name) company_name = clean(headline.slice(sep.index + sep[0].length).split(/\s+[|·]\s+/)[0])
  }

  const location = text(document.querySelector('main .text-body-small.inline.t-black--light.break-words, .text-body-small.inline.t-black--light.break-words'))

  // Adresse canonique du profil, sans paramètres de suivi.
  const m = /^\/in\/([^/?#]+)/.exec(location_path())
  const linkedin_url = m ? `https://www.linkedin.com/in/${m[1]}` : ''

  function location_path() {
    return window.location.pathname
  }

  return { first_name, last_name, job_title, company_name, location, linkedin_url }
}
