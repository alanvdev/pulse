// Piezas compartidas de la isla (Comando y Orbe): tarjeta de permiso, avisos, "mientras estabas en foco" y efectos de luz.
import type { Config, Pending } from '../shared/bridge';
import { esc } from '../shared/format';
import { isUrgent } from '../shared/micro';
import { reduceMotion, T } from '../shared/motion';
import { isLinux } from '../shared/platform';
import { queueInfo, releaseRow, verb, type Notice } from './model';

const rgb = (h: string) => { const n = parseInt(h.slice(1), 16); return [n >> 16 & 255, n >> 8 & 255, n & 255]; };
const rgba = (h: string, a: number) => `rgba(${rgb(h).join(',')},${a})`;
const norm = (s: string) => s.normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase();

/** Coincidencia aproximada de la paleta: letras en orden, sin acentos; las coincidencias van en negrita. */
export function fuzzy(text: string, query: string): { ok: boolean; html: string } {
  if (!query) return { ok: true, html: esc(text) };
  const t = norm(text), qq = norm(query); let i = 0, out = '';
  for (let k = 0; k < text.length; k++) { if (i < qq.length && t[k] === qq[i]) { out += `<b>${esc(text[k])}</b>`; i++; } else out += esc(text[k]); }
  return { ok: i === qq.length, html: out };
}

export const keys = (k: string[]) => `<span class="keys">${k.map(x => `<kbd class="kbd">${esc(x)}</kbd>`).join('')}</span>`;
const secsLeft = (p: Pending, now = Date.now()) => `${Math.max(0, Math.ceil((p.paused ? p.left_ms : p.deadline_ms - now) / 1000))}s`;

/** Tarjeta de permiso (spec §3.1 y §3.3): cabecera, comando (3 líneas, el resto se desplaza), descripción, quién sigue y barra de acciones.
 *  En Linux el atajo llega como un golpe sin "soltar" (lo registra GNOME) y no se puede mantener: en un comando delicado
 *  no se muestra Ctrl+Alt+Y, se aprueba manteniendo el botón. */
export function permCard(p: Pending, cfg: Config, all: Pending[], width = 480) {
  const danger = p.danger && cfg.island.hold, k = cfg.island.keys, q = queueInfo(all);
  return `<div class="card" style="width:${width}px" data-pid="${p.id}">
    <div class="card-b">
      <div class="head"><span class="dot halo"></span><b>${esc(p.project)}</b>${verb(p.tool, true)}<span class="mono head-r">${q ? `${q.pos}${cfg.island.countdown ? ' · ' : ''}` : ''}${cfg.island.countdown ? `<span class="secs">${secsLeft(p)}</span>` : ''}</span></div>
      <div class="cmd" tabindex="0" aria-label="Comando completo">${esc(p.command)}</div>
      ${p.description ? `<div class="desc">${esc(p.description)}</div>` : ''}
      ${q ? `<div class="next">${esc(q.next)}${q.nextCmd ? ` <span class="mono">${esc(q.nextCmd)}</span>` : ''}</div>` : ''}
    </div>
    <div class="bar"><span class="bar-l">${danger ? 'Delicado' : 'Pulse'}</span>
      <button type="button" class="btn" data-act="deny" data-id="${p.id}" style="margin-left:auto"><span>Denegar</span>${k ? keys(['Ctrl', 'Alt', 'N']) : ''}</button>
      <button type="button" class="btn pri" data-act="allow" data-id="${p.id}" data-danger="${danger ? 1 : 0}" data-hold><span>${danger ? 'Mantener' : 'Permitir'}</span>${k && !(danger && isLinux) ? keys(['Ctrl', 'Alt', 'Y']) : ''}</button>
    </div></div>`;
}

/** Aviso de una línea con su tiempo en la línea fina de abajo (`.tm`, la anima morph con `life`). */
export function noticeHTML(n: Notice) {
  if (n.items) return releaseCard(n);
  return `<div class="nt"><span class="ms${n.icon === 'check' ? ' draw' : ''}" style="color:${n.color}">${n.icon}</span><span class="t">${n.who ? `<b>${esc(n.who)}</b> ` : ''}<span class="tx2">${esc(n.what)}</span>${n.extra ? ` <span class="mono">${esc(n.extra)}</span>` : ''}</span><i class="tm"></i></div>`;
}
/** "Mientras estabas en foco" (spec §3.3): icono, proyecto, qué pasó y hora de cada evento guardado. */
export function releaseCard(n: Notice) {
  const items = n.items ?? [];
  return `<div class="card" style="width:400px"><div class="card-b rel-b"><div class="head"><span class="ms">dark_mode</span><b>Mientras estabas en foco</b><span class="mono head-r">${items.length}</span></div>${items.map((d, i) => { const r = releaseRow(d); return `<div class="rel" style="--i:${i}"><span class="ms" style="color:${r.color}">${r.icon}</span><span>${esc(r.project)}</span><small>${esc(r.text)}</small></div>`; }).join('')}</div><i class="tm"></i></div>`;
}

/** Comando largo: tres líneas a la vista, desvanecido abajo mientras quede algo por ver, y la barra lo dice. */
export function fitCommand(root: ParentNode) {
  root.querySelectorAll<HTMLElement>('.layer:last-child .cmd').forEach(c => {
    const long = c.scrollHeight > c.clientHeight + 2, lab = c.closest('.card')?.querySelector<HTMLElement>('.bar-l');
    if (long && lab) lab.textContent = 'Desplaza para ver todo';
    const upd = () => c.classList.toggle('more', c.scrollTop + c.clientHeight < c.scrollHeight - 2);
    upd(); c.onscroll = upd;
  });
}
/** Segundos restantes de cada tarjeta, sin volver a dibujarla. */
export function tickSecs(root: ParentNode, pending: Pending[]) {
  root.querySelectorAll<HTMLElement>('[data-pid]').forEach(card => { const p = pending.find(x => x.id === Number(card.dataset.pid)); const s = card.querySelector('.secs'); if (p && s) { s.textContent = secsLeft(p); s.classList.toggle('urgent', isUrgent(p, Date.now())); } });
}

export function beam(board: HTMLElement, c: string, xpx: number, enabled = true) {
  if (!enabled || reduceMotion()) return;
  const b = board.querySelector<HTMLElement>('.beam')!, W = board.clientWidth, tg = Math.max(1, Math.min(99, xpx / W * 100));
  const g = (p: string) => `radial-gradient(80px 1.5px at ${p} 0, ${c}, transparent), radial-gradient(170px 28px at ${p} 0, ${rgba(c, .3)}, transparent)`;
  b.style.background = [g('var(--p)'), g(`calc(${tg * 2}% - var(--p))`)].join(',');
  b.animate([{ '--p': '0%', opacity: 0 }, { '--p': `${tg * .3}%`, opacity: 1, offset: .22 }, { '--p': `${tg}%`, opacity: 1, offset: .9 }, { '--p': `${tg}%`, opacity: 0 }], { duration: T(600), easing: 'cubic-bezier(.55,0,.75,.2)', fill: 'forwards' });
}
export const ignite = (el: HTMLElement | null, c: string) => { if (!el || reduceMotion()) return; el.animate([{ boxShadow: `0 0 0 1px ${rgba(c, .7)}, 0 8px 30px -6px ${rgba(c, .45)}` }, { boxShadow: `0 0 0 1px ${rgba(c, 0)}, 0 8px 30px -6px ${rgba(c, 0)}` }], { duration: T(1000), easing: 'cubic-bezier(.16,1,.3,1)', composite: 'add' }); };
export const beat = (el: HTMLElement | null) => { if (!el || reduceMotion()) return; el.animate([{ scale: 1 }, { scale: 1.04, offset: .14 }, { scale: 1, offset: .3 }, { scale: 1.03, offset: .46 }, { scale: 1 }], { duration: T(900), easing: 'ease-out', composite: 'add' }); };
export const shake = (el: HTMLElement | null) => { if (!el || reduceMotion()) return; el.animate([{ translate: '0 0' }, { translate: '-3px 0' }, { translate: '3px 0' }, { translate: '-2px 0' }, { translate: '0 0' }], { duration: T(320), composite: 'add' }); };
