// Piezas compartidas de la isla, portadas de docs/artboards/isla.html.
import type { Config, Deferred, Pending, Session } from '../shared/bridge';
import { COLORS, LBL, esc, stateColor } from '../shared/format';
import { reduceMotion, T } from '../shared/motion';

/** Aviso pasajero (equivale a `M.tr` del artboard). */
export type Tr = { k: 'done' | 'error' | 'ok' | 'no' | 'info' | 'release'; id: number; project?: string; text?: string; icon?: string; items?: Deferred[] };

const rgb = (h: string) => { const n = parseInt(h.slice(1), 16); return [n >> 16 & 255, n >> 8 & 255, n & 255]; };
const rgba = (h: string, a: number) => `rgba(${rgb(h).join(',')},${a})`;
const norm = (s: string) => s.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();

export function fuzzy(text: string, query: string): { ok: boolean; html: string } {
  if (!query) return { ok: true, html: esc(text) };
  const t = norm(text), qq = norm(query); let i = 0, out = '';
  for (let k = 0; k < text.length; k++) { if (i < qq.length && t[k] === qq[i]) { out += `<u>${esc(text[k])}</u>`; i++; } else out += esc(text[k]); }
  return { ok: i === qq.length, html: out };
}

/** "Quiere ejecutar" para la terminal, "Quiere usar" para el resto de herramientas. */
export const verb = (tool: string, lower = false) => { const v = tool === 'Bash' || tool === 'PowerShell' ? 'Quiere ejecutar' : 'Quiere usar'; return lower ? v.toLowerCase() : v; };

export const stHTML = (s: Session) => `<span class="st" style="--c:${stateColor(s.state)}">${LBL[s.state]}</span>`;

export function permCard(p: Pending, cfg: Config, total: number, s?: Session) {
  const danger = p.danger && cfg.island.hold, keys = cfg.island.keys;
  return `<div class="card">
    <div class="ch"><span class="nm">${esc(p.project)}</span>${total > 1 ? `<span class="muted mono" style="font-size:11.5px">1 de ${total}</span>` : s ? stHTML(s) : ''}</div>
    <div class="t" style="font-size:13.5px"><b>${verb(p.tool)}</b>${p.description ? `<div class="why">${esc(p.description)}</div>` : ''}</div>
    <div class="cmd full">${esc(p.command)}</div>
    ${danger ? `<div class="warn"><span class="ms">back_hand</span>Comando delicado: mantén presionado para permitir</div>` : ''}
    <div class="acts"><button class="b-no" type="button" data-act="deny" data-id="${p.id}"><span>Denegar</span>${keys ? '<kbd data-k="n">Ctrl+Alt+N</kbd>' : ''}</button><button class="b-yes" type="button" data-act="allow" data-id="${p.id}" data-danger="${danger ? 1 : 0}"><span class="fill"></span><span>${danger ? 'Mantener' : 'Permitir'}</span>${keys ? '<kbd data-k="y">Ctrl+Alt+Y</kbd>' : ''}</button></div>
    ${cfg.island.countdown ? '<i class="cd"></i>' : ''}</div>`;
}

export function trPill(tr: Tr) {
  const map: Record<string, [string, string]> = { done: ['check', COLORS.done], error: ['error', COLORS.error], ok: ['check', COLORS.done], no: ['close', '#ff8c84'], info: [tr.icon || 'info', '#cfd4db'], release: ['dark_mode', COLORS.focus] };
  const [ic, c] = map[tr.k] || ['info', '#cfd4db'];
  const who = tr.project ? `<b>${esc(tr.project)}</b> ` : '';
  const txt = tr.k === 'ok' ? (tr.project ? `Permitido <span class="muted mono" style="font-size:12px">${esc(tr.text)}</span>` : esc(tr.text)) : tr.k === 'no' ? `Denegado <span class="muted mono" style="font-size:12px">${esc(tr.text)}</span>` : tr.k === 'done' || tr.k === 'error' ? `${who}<span class="${tr.k === 'error' ? 'bad' : 'muted'}">${esc(tr.text)}</span>` : esc(tr.text);
  return { ic, c, txt };
}

export function releaseCard(items: Deferred[]) {
  const L: Record<string, string> = { done: 'terminó', error: 'falló una tool', idle: 'te espera' };
  return `<div class="card" style="min-width:280px;gap:8px"><div class="ch"><span class="t"><b>Mientras estabas en foco</b></span><span class="muted mono" style="font-size:11.5px">${items.length}</span></div>${items.map((d, i) => `<div class="row" style="font-size:12.5px;justify-content:space-between;opacity:0;animation:liIn .4s var(--e-out) ${T(140 + i * 70)}ms forwards"><span class="mono" style="font-size:12px">${esc(d.project)}</span><span class="st" style="--c:${(COLORS as Record<string, string>)[d.kind] ?? COLORS.idle}">${L[d.kind] ?? d.kind}</span></div>`).join('')}</div>`;
}

export function beam(board: HTMLElement, c: string, xpx: number, enabled = true) {
  if (!enabled || reduceMotion()) return;
  const b = board.querySelector<HTMLElement>('.beam')!, W = board.clientWidth, tg = Math.max(1, Math.min(99, xpx / W * 100));
  const g = (p: string) => `radial-gradient(80px 1.5px at ${p} 0, ${c}, transparent), radial-gradient(170px 28px at ${p} 0, ${rgba(c, .35)}, transparent)`;
  b.style.background = [g('var(--p)'), g(`calc(${tg * 2}% - var(--p))`)].join(',');
  b.animate([{ '--p': '0%', opacity: 0 }, { '--p': `${tg * .3}%`, opacity: 1, offset: .22 }, { '--p': `${tg}%`, opacity: 1, offset: .9 }, { '--p': `${tg}%`, opacity: 0 }], { duration: T(600), easing: 'cubic-bezier(.55,0,.75,.2)', fill: 'forwards' });
}
export const ignite = (el: HTMLElement | null, c: string) => { if (!el || reduceMotion()) return; el.animate([{ boxShadow: `0 0 0 1px ${rgba(c, .85)}, 0 8px 30px -6px ${rgba(c, .6)}` }, { boxShadow: `0 0 0 1px ${rgba(c, 0)}, 0 8px 30px -6px ${rgba(c, 0)}` }], { duration: T(1000), easing: 'cubic-bezier(.16,1,.3,1)', composite: 'add' }); };
export const beat = (el: HTMLElement | null) => { if (!el || reduceMotion()) return; el.animate([{ scale: 1 }, { scale: 1.04, offset: .14 }, { scale: 1, offset: .3 }, { scale: 1.03, offset: .46 }, { scale: 1 }], { duration: T(900), easing: 'ease-out', composite: 'add' }); };
export const shake = (el: HTMLElement | null) => { if (!el || reduceMotion()) return; el.animate([{ translate: '0 0' }, { translate: '-3px 0' }, { translate: '3px 0' }, { translate: '-2px 0' }, { translate: '0 0' }], { duration: T(320), composite: 'add' }); };
