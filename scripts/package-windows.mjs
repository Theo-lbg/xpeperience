// Construit « XpePerience-<version>-windows-x64.zip » : l'application Windows prête à double-cliquer.
//
//   node scripts/package-windows.mjs                  (sous Windows : construit l'interface, compile, emballe)
//   node scripts/package-windows.mjs --skip-build --exe chemin/vers/xpeperience.exe   (emballage seul, pour tester)
//
// Résultat : dist/XpePerience-<version>-windows-x64.zip contenant XpePerience/XpePerience.exe et LISEZMOI.txt.
// L'application n'est pas signée : Windows SmartScreen l'affichera comme « éditeur inconnu » (voir LISEZMOI.txt).
//
// Aucune dépendance : le .zip est écrit ici (Node ≥ 22.2 pour zlib.crc32).
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { crc32, deflateRawSync } from 'node:zlib'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const args = process.argv.slice(2)
const skipBuild = args.includes('--skip-build')
const exeArg = args.includes('--exe') ? args[args.indexOf('--exe') + 1] : null

const README = [
  'XpePerience — application Windows',
  '',
  '1. Copiez le dossier où vous voulez (le Bureau, Documents…), puis double-cliquez sur XpePerience.exe.',
  '2. Au premier lancement, Windows peut afficher « Windows a protégé votre ordinateur » : l’application n’est',
  '   pas signée par un certificat payant. Cliquez sur « Informations complémentaires », puis sur',
  '   « Exécuter quand même ». Cette étape n’est nécessaire qu’une seule fois.',
  '3. Créez le compte administrateur. Vos données restent sur cet ordinateur.',
  '',
  'Vos données : %APPDATA%\\fr.xpeho.xpeperience   (pour les sauvegarder, copiez ce dossier)',
  'Désinstaller : supprimez XpePerience.exe. Pour effacer aussi vos données, supprimez le dossier ci-dessus.',
  'Prérequis : Windows 10 ou 11 à jour (composant WebView2 de Microsoft Edge, déjà présent sur les machines à jour).',
  '',
  'Extension LinkedIn et serveur d’équipe : voir le fichier README du projet.',
  'Intégrité : comparez l’empreinte SHA-256 de ce .zip avec SHA256SUMS.txt, joint à la release.',
  '',
].join('\r\n')

function run(cmd, cmdArgs, cwd) {
  console.log(`▸ ${cmd} ${cmdArgs.join(' ')}`)
  const r = spawnSync(cmd, cmdArgs, { cwd, stdio: 'inherit', shell: process.platform === 'win32' })
  if (r.status !== 0) {
    console.error(`Échec : ${cmd} ${cmdArgs.join(' ')}`)
    process.exit(r.status ?? 1)
  }
}

/** Écrit un .zip (compression deflate, noms en UTF-8) à partir de [{ name, data }]. */
export function zip(entries, date = new Date()) {
  const dosTime = (date.getHours() << 11) | (date.getMinutes() << 5) | (date.getSeconds() >> 1)
  const dosDate = ((date.getFullYear() - 1980) << 9) | ((date.getMonth() + 1) << 5) | date.getDate()
  const parts = []
  const central = []
  let offset = 0
  for (const { name, data } of entries) {
    const nameBuf = Buffer.from(name, 'utf8')
    const packed = deflateRawSync(data, { level: 9 })
    const crc = crc32(data)
    const local = Buffer.alloc(30)
    local.writeUInt32LE(0x04034b50, 0)
    local.writeUInt16LE(20, 4) // version nécessaire
    local.writeUInt16LE(0x0800, 6) // noms en UTF-8
    local.writeUInt16LE(8, 8) // deflate
    local.writeUInt16LE(dosTime, 10)
    local.writeUInt16LE(dosDate, 12)
    local.writeUInt32LE(crc, 14)
    local.writeUInt32LE(packed.length, 18)
    local.writeUInt32LE(data.length, 22)
    local.writeUInt16LE(nameBuf.length, 26)
    parts.push(local, nameBuf, packed)

    const dir = Buffer.alloc(46)
    dir.writeUInt32LE(0x02014b50, 0)
    dir.writeUInt16LE(20, 4) // créé par
    dir.writeUInt16LE(20, 6) // version nécessaire
    dir.writeUInt16LE(0x0800, 8)
    dir.writeUInt16LE(8, 10)
    dir.writeUInt16LE(dosTime, 12)
    dir.writeUInt16LE(dosDate, 14)
    dir.writeUInt32LE(crc, 16)
    dir.writeUInt32LE(packed.length, 20)
    dir.writeUInt32LE(data.length, 24)
    dir.writeUInt16LE(nameBuf.length, 28)
    dir.writeUInt32LE(offset, 42)
    central.push(dir, nameBuf)
    offset += local.length + nameBuf.length + packed.length
  }
  const centralBuf = Buffer.concat(central)
  const end = Buffer.alloc(22)
  end.writeUInt32LE(0x06054b50, 0)
  end.writeUInt16LE(entries.length, 8)
  end.writeUInt16LE(entries.length, 10)
  end.writeUInt32LE(centralBuf.length, 12)
  end.writeUInt32LE(offset, 16)
  return Buffer.concat([...parts, centralBuf, end])
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const version = /^version\s*=\s*"([^"]+)"/m.exec(readFileSync(join(root, 'Cargo.toml'), 'utf8'))?.[1]
  if (!version) throw new Error('Version introuvable dans Cargo.toml')

  if (!skipBuild) {
    if (process.platform !== 'win32') {
      console.error('Ce script doit être lancé sous Windows (ou avec --skip-build --exe <fichier> pour un essai).')
      process.exit(1)
    }
    run('npm', ['ci', '--no-audit', '--no-fund'], join(root, 'web'))
    run('npm', ['run', 'build'], join(root, 'web'))
    run('cargo', ['build', '--release', '--locked', '-p', 'xpeperience-desktop'], root)
  }

  const exe = resolve(exeArg ?? join(root, 'target', 'release', 'xpeperience.exe'))
  if (!existsSync(exe)) {
    console.error(`Exécutable introuvable : ${exe}`)
    process.exit(1)
  }

  const out = join(root, 'dist', `XpePerience-${version}-windows-x64.zip`)
  mkdirSync(dirname(out), { recursive: true })
  writeFileSync(
    out,
    zip([
      { name: 'XpePerience/XpePerience.exe', data: readFileSync(exe) },
      { name: 'XpePerience/LISEZMOI.txt', data: Buffer.from('﻿' + README, 'utf8') }, // BOM : Bloc-notes lit l'UTF-8
    ]),
  )
  console.log(`Terminé : ${out}`)
}
