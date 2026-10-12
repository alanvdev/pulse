import { writeFileSync, mkdirSync } from 'node:fs';
import { deflateSync } from 'node:zlib';
// Logo de Pulse (DESIGN.md): anillo con los colores apagados de Grafito sobre un azulejo grafito de esquinas redondeadas.
// Uso: node scripts/make-icon.mjs, y luego `npx --prefix app tauri icon assets/icon.png -o app/src-tauri/icons`.
const S = 1024, R1 = 246, R2 = 338, CORNER = S * .22, TILE = [17, 19, 17];   // --surface #111311
const stops = [[234,237,234],[227,176,92],[143,195,160],[143,176,214],[234,237,234]];   // tx, perm, done, sub, tx
const px = Buffer.alloc(S * S * 4);
const clamp = v => Math.max(0, Math.min(1, v));
for (let y = 0; y < S; y++) for (let x = 0; x < S; x++) {
  const o = (y * S + x) * 4;
  // azulejo: cuadrado redondeado con borde suavizado
  const qx = Math.max(Math.abs(x + .5 - S / 2) - (S / 2 - CORNER), 0), qy = Math.max(Math.abs(y + .5 - S / 2) - (S / 2 - CORNER), 0);
  const ta = clamp(CORNER - Math.hypot(qx, qy) + .5);
  // anillo con degradado alrededor
  const dx = x - S / 2 + .5, dy = y - S / 2 + .5, r = Math.hypot(dx, dy), a = clamp(Math.min(r - R1, R2 - r) / 2 + .5);
  const t = ((Math.atan2(dy, dx) / (2 * Math.PI)) + 1) % 1 * 4, i = Math.floor(t), f = t - i, c = stops[i].map((v, k) => Math.round(v + (stops[i + 1][k] - v) * f));
  for (let k = 0; k < 3; k++) px[o + k] = Math.round(c[k] * a + TILE[k] * (1 - a));
  px[o + 3] = Math.round(ta * 255);
}
const raw = Buffer.alloc((S * 4 + 1) * S);
for (let y = 0; y < S; y++) px.copy(raw, y * (S * 4 + 1) + 1, y * S * 4, (y + 1) * S * 4);
const T = Array.from({ length: 256 }, (_, n) => { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; return c >>> 0; });
const crc = b => { let c = ~0; for (const v of b) c = T[(c ^ v) & 255] ^ (c >>> 8); return (~c) >>> 0; };
const chunk = (t, d) => { const l = Buffer.alloc(4); l.writeUInt32BE(d.length); const td = Buffer.concat([Buffer.from(t), d]); const c = Buffer.alloc(4); c.writeUInt32BE(crc(td)); return Buffer.concat([l, td, c]); };
const ih = Buffer.alloc(13); ih.writeUInt32BE(S, 0); ih.writeUInt32BE(S, 4); ih[8] = 8; ih[9] = 6;
mkdirSync('assets', { recursive: true });
writeFileSync('assets/icon.png', Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]), chunk('IHDR', ih), chunk('IDAT', deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]));
console.log('assets/icon.png listo');
