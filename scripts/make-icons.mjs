// Génère les icônes de l'application XpePerience : carré arrondi vert XPEHO avec un « B » noir.
//
//   node scripts/make-icons.mjs [dossier]      (défaut : desktop/icons)
//
// Produit : icon.png (512), 32x32.png, 128x128.png, icon.ico (Windows : 16, 32, 48, 256 px)
// et, sous macOS uniquement, icon.icns (via l'outil système `iconutil`).
// Aucune dépendance : les PNG sont dessinés et encodés ici.
import { execFileSync } from 'node:child_process'
import { mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { deflateSync } from 'node:zlib'

const crcTable = Array.from({ length: 256 }, (_, n) => {
  let c = n
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
  return c >>> 0
})
const crc = (buf) => {
  let c = 0xffffffff
  for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8)
  return (c ^ 0xffffffff) >>> 0
}
const chunk = (type, data) => {
  const t = Buffer.from(type)
  const len = Buffer.alloc(4)
  len.writeUInt32BE(data.length)
  const c = Buffer.alloc(4)
  c.writeUInt32BE(crc(Buffer.concat([t, data])))
  return Buffer.concat([len, t, data, c])
}

const inRect = (u, v, x0, y0, x1, y1) => u >= x0 && u <= x1 && v >= y0 && v <= y1
const ring = (u, v, cx, cy, ro, ri, right) => {
  const d = Math.hypot(u - cx, v - cy)
  return (right ? u >= cx : true) && d <= ro && d >= ri
}
const glyph = (u, v) =>
  inRect(u, v, 0.3, 0.2, 0.4, 0.8) ||
  inRect(u, v, 0.3, 0.2, 0.56, 0.3) ||
  inRect(u, v, 0.3, 0.45, 0.58, 0.55) ||
  inRect(u, v, 0.3, 0.7, 0.6, 0.8) ||
  ring(u, v, 0.56, 0.375, 0.175, 0.075, true) ||
  ring(u, v, 0.58, 0.625, 0.175, 0.075, true)

export function png(size) {
  const raw = Buffer.alloc((size * 4 + 1) * size)
  const r = size * 0.225
  for (let y = 0; y < size; y++) {
    raw[y * (size * 4 + 1)] = 0
    for (let x = 0; x < size; x++) {
      const dx = Math.max(r - x - 0.5, x + 0.5 - (size - r), 0)
      const dy = Math.max(r - y - 0.5, y + 0.5 - (size - r), 0)
      let px = [0, 0, 0, 0]
      if (dx * dx + dy * dy <= r * r) {
        // Sur-échantillonnage 3×3 pour des bords lisses.
        let hits = 0
        for (let sy = 0; sy < 3; sy++) for (let sx = 0; sx < 3; sx++) if (glyph((x + (sx + 0.5) / 3) / size, (y + (sy + 0.5) / 3) / size)) hits++
        const t = hits / 9
        px = [Math.round(179 * (1 - t) + 17 * t), Math.round(210 * (1 - t) + 19 * t), Math.round(107 * (1 - t) + 18 * t), 255]
      }
      raw.set(px, y * (size * 4 + 1) + 1 + x * 4)
    }
  }
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(size, 0)
  ihdr.writeUInt32BE(size, 4)
  ihdr[8] = 8
  ihdr[9] = 6
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', deflateSync(raw)), chunk('IEND', Buffer.alloc(0))])
}

/** Fichier .ico contenant des images PNG (accepté par Windows depuis Vista). */
export function ico(sizes) {
  const images = sizes.map((s) => ({ size: s, data: png(s) }))
  const head = Buffer.alloc(6)
  head.writeUInt16LE(1, 2) // type : icône
  head.writeUInt16LE(images.length, 4)
  let offset = 6 + 16 * images.length
  const entries = images.map(({ size, data }) => {
    const e = Buffer.alloc(16)
    e[0] = size >= 256 ? 0 : size // 0 = 256 px
    e[1] = size >= 256 ? 0 : size
    e.writeUInt16LE(1, 4) // plans
    e.writeUInt16LE(32, 6) // bits par pixel
    e.writeUInt32LE(data.length, 8)
    e.writeUInt32LE(offset, 12)
    offset += data.length
    return e
  })
  return Buffer.concat([head, ...entries, ...images.map((i) => i.data)])
}

if (process.argv[1] && import.meta.url.endsWith(process.argv[1].split('/').pop())) {
  const out = process.argv[2] ?? 'desktop/icons'
  mkdirSync(out, { recursive: true })
  writeFileSync(join(out, 'icon.png'), png(512))
  writeFileSync(join(out, '32x32.png'), png(32))
  writeFileSync(join(out, '128x128.png'), png(128))
  writeFileSync(join(out, 'icon.ico'), ico([16, 32, 48, 256]))
  if (process.platform === 'darwin') {
    const set = join(out, 'icon.iconset')
    mkdirSync(set, { recursive: true })
    for (const [name, size] of [['16x16', 16], ['16x16@2x', 32], ['32x32', 32], ['32x32@2x', 64], ['128x128', 128], ['128x128@2x', 256], ['256x256', 256], ['256x256@2x', 512], ['512x512', 512], ['512x512@2x', 1024]]) {
      writeFileSync(join(set, `icon_${name}.png`), png(size))
    }
    execFileSync('iconutil', ['-c', 'icns', set, '-o', join(out, 'icon.icns')])
    rmSync(set, { recursive: true })
  }
  console.log(`Icônes écrites dans ${out}`)
}
