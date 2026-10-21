// Test du lecteur de profil sur un faux DOM : node extension/reader.test.mjs
import { readFileSync } from 'node:fs'
import assert from 'node:assert/strict'
const src = readFileSync(new URL('./reader.js', import.meta.url), 'utf8')

function run({ h1, headline, location, aria, path = '/in/sam-profil-fictif/', title = '', ogDesc = '' }) {
  const by = (sel) => {
    if (/h1/.test(sel) && h1 != null) return { textContent: h1 }
    if (/text-body-medium/.test(sel) && headline != null) return { textContent: headline }
    if (/text-body-small/.test(sel) && location != null) return { textContent: location }
    if (/meta/.test(sel) && ogDesc) return { getAttribute: () => ogDesc }
    return null
  }
  const document = {
    title,
    querySelector: by,
    querySelectorAll: () => (aria ? [{ getAttribute: () => aria }] : []),
  }
  const window = { location: { pathname: path } }
  return new Function('document', 'window', src + '; return readLinkedInProfile()')(document, window)
}

// Profil FR complet
let p = run({ h1: 'Camille  Dupont-Martin', headline: 'Directrice des systèmes d’information chez Société Fictive Alpha', location: 'Lille, Hauts-de-France, France', aria: 'Entreprise actuelle : Société Fictive Alpha. Cliquez pour accéder à la section Expérience' })
assert.deepEqual(p, { first_name: 'Camille', last_name: 'Dupont-Martin', job_title: 'Directrice des systèmes d’information', company_name: 'Société Fictive Alpha', location: 'Lille, Hauts-de-France, France', linkedin_url: 'https://www.linkedin.com/in/sam-profil-fictif' })

// Intitulé EN sans bouton société, nom composé, pronoms et diplôme retirés
p = run({ h1: 'Jean De La Fontaine (He/Him), PhD', headline: 'CTO @ Entreprise Fictive Beta | Cloud', path: '/in/jean-fictif/?trk=x' })
assert.equal(p.first_name, 'Jean'); assert.equal(p.last_name, 'De La Fontaine')
assert.equal(p.job_title, 'CTO'); assert.equal(p.company_name, 'Entreprise Fictive Beta')
assert.equal(p.linkedin_url, 'https://www.linkedin.com/in/jean-fictif')

// Rien d'exploitable sur la page : champs vides, aucune exception, pas d'URL hors /in/
p = run({ path: '/feed/', title: '' })
assert.deepEqual(p, { first_name: '', last_name: '', job_title: '', company_name: '', location: '', linkedin_url: '' })

// Titre du document en secours
p = run({ title: '(3) Robin Exemple | LinkedIn' })
assert.equal(p.first_name, 'Robin'); assert.equal(p.last_name, 'Exemple')
console.log('reader OK (4 scénarios)')
