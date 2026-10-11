import { writeFileSync, mkdirSync } from 'node:fs';
import { deflateSync } from 'node:zlib';
const S = 1024, R1 = 300, R2 = 400, stops = [[230,238,255],[255,176,46],[79,224,160],[92,200,255],[230,238,255]];
const px = Buffer.alloc(S * S * 4);
for (let y = 0; y < S; y++) for (let x = 0; x < S; x++) {
  const dx = x - S / 2 + .5, dy = y - S / 2 + .5, r = Math.hypot(dx, dy), a = Math.max(0, Math.min(1, Math.min(r - R1, R2 - r) / 2 + .5));
  const t = ((Math.atan2(dy, dx) / (2 * Math.PI)) + 1) % 1 * 4, i = Math.floor(t), f = t - i, c = stops[i].map((v, k) => Math.round(v + (stops[i + 1][k] - v) * f));
  const o = (y * S + x) * 4; px[o] = c[0]; px[o + 1] = c[1]; px[o + 2] = c[2]; px[o + 3] = Math.round(a * 255);
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
