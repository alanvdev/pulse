// Modo Comando, portado de `Comando(board)` en docs/artboards/isla.html.
import type { Config, Snapshot } from '../shared/bridge';
import { COLORS, esc } from '../shared/format';
import { morph, reduceMotion, T, type Morph, type MorphOpts } from '../shared/motion';
import { beam, fuzzy, ignite, releaseCard, trPill, type Tr } from './shared';

export interface Ctx {
  snap(): Snapshot; cfg(): Config; transient(): Tr | null;
  decide(id: number, allow: boolean): void; allowAll(): void; setFocus(on: boolean): void; muteSession(id: string, minutes: number): void; openDashboard(): void;
  islandFocus(on: boolean): void; hitbox(el: HTMLElement | null): void;
}
type Item = { g: string; icon: string; c: string; text: string; meta: string; run: () => void };

export function mountComando(board: HTMLElement, ctx: Ctx) {
  const sh = document.createElement('div'); sh.className = 'shell glass c-shell'; sh.style.opacity = '0';
  board.append(sh);
  const m: Morph = { el: sh, w: 0, h: 0, sig: '', op: 0 };
  let open = false, q = '', sel = 0, items: Item[] = [];
  const $ = <T extends Element = HTMLElement>(s: string) => sh.querySelector<T>(s);
  const $$ = (s: string) => [...sh.querySelectorAll<HTMLElement>(s)];

  function openPal() { open = true; q = ''; sel = 0; ctx.islandFocus(true); render(); requestAnimationFrame(() => $<HTMLInputElement>('input')?.focus()); }
  function close() { open = false; ctx.islandFocus(false); render(); }
  function build(): Item[] {
    const L: Item[] = [], s = ctx.snap(), now = Date.now();
    s.pending.forEach(p => { L.push({ g: 'Pendiente', icon: 'check', c: COLORS.done, text: `Permitir ${p.command}`, meta: p.project, run: () => ctx.decide(p.id, true) }); L.push({ g: 'Pendiente', icon: 'close', c: '#ff8c84', text: `Denegar ${p.command}`, meta: p.project, run: () => ctx.decide(p.id, false) }); });
    L.push({ g: 'Acciones', icon: 'dark_mode', c: COLORS.focus, text: s.focus ? 'Salir del modo foco' : 'Modo foco por 1 hora', meta: '', run: () => ctx.setFocus(!s.focus) });
    const allMuted = s.sessions.length > 0 && s.sessions.every(x => x.muted_until_ms > now);
    L.push({ g: 'Acciones', icon: allMuted ? 'volume_up' : 'volume_off', c: '#9aa1ab', text: allMuted ? 'Quitar silencio' : 'Silenciar todo 30 min', meta: '', run: () => s.sessions.forEach(x => ctx.muteSession(x.id, allMuted ? 0 : 30)) });
    L.push({ g: 'Acciones', icon: 'done_all', c: COLORS.done, text: 'Aprobar todos los permisos', meta: s.pending.length ? String(s.pending.length) : '', run: ctx.allowAll });
    L.push({ g: 'Acciones', icon: 'dashboard', c: '#cfd4db', text: 'Abrir tablero', meta: '', run: ctx.openDashboard });
    return L;
  }
  function palHTML() {
    const all = build(); items = [];
    let g = '', rows = '';
    all.forEach(it => { const f = fuzzy(it.text, q); if (!f.ok) return; const n = items.length; items.push(it); if (it.g && it.g !== g) { g = it.g; rows += `<div class="pal-g">${g}</div>`; } rows += `<div class="pal-it" role="option" data-n="${n}" style="--c:${it.c}"><span class="ic"><span class="ms">${it.icon}</span></span><span>${f.html}</span><small>${esc(it.meta)}</small></div>`; });
    sel = Math.min(sel, Math.max(0, items.length - 1));
    return `<div class="pal"><div class="pal-in"><span class="ms">search</span><input type="text" value="${esc(q)}" placeholder="Busca un permiso o una acción" aria-label="Buscar en Pulse"><kbd>Esc</kbd></div><div class="pal-list" role="listbox"><i class="pal-hl"></i>${rows || '<div class="pal-empty">Nada coincide.</div>'}</div><div class="pal-foot"><span><kbd>↑</kbd><kbd>↓</kbd>moverte</span><span><kbd>↵</kbd>ejecutar</span><span><kbd>Esc</kbd>cerrar</span></div></div>`;
  }
  function moveHL(instant?: boolean) {
    const hl = $('.pal-hl'), row = $(`.pal-it[data-n="${sel}"]`); if (!hl) return;
    if (!row) { hl.style.opacity = '0'; return; }
    hl.style.opacity = '1';
    if (instant) { hl.style.transition = 'none'; hl.style.transform = `translateY(${row.offsetTop}px)`; void hl.offsetWidth; hl.style.transition = ''; } else hl.style.transform = `translateY(${row.offsetTop}px)`;
    $$('.pal-it').forEach(r => r.setAttribute('aria-selected', String(+r.dataset.n! === sel)));
    row.scrollIntoView({ block: 'nearest' });
  }
  function run(n: number) {
    const it = items[n]; if (!it) return;
    const row = $(`.pal-it[data-n="${n}"]`); if (row && !reduceMotion()) row.animate([{ background: 'rgba(255,255,255,.18)' }, { background: 'rgba(255,255,255,0)' }], { duration: T(260) });
    setTimeout(() => { open = false; ctx.islandFocus(false); it.run(); render(); }, T(120));
  }
  function bindRows() { $$('.pal-it').forEach(r => { r.onmouseenter = () => { sel = +r.dataset.n!; moveHL(); }; r.onclick = () => run(+r.dataset.n!); }); }
  function render(forcePal?: boolean) {
    const s = ctx.snap(), cfg = ctx.cfg().island, tr = ctx.transient();
    let html = '', sig = 'x', o: MorphOpts = { radius: (_w, h) => h > 50 ? '14px' : '12px', hidden: true, dy: -4 };
    if (open) {
      if (m.sig.startsWith('pal') && !forcePal) { // refresca la lista sin perder el foco
        const tmp = document.createElement('div'); tmp.innerHTML = palHTML(); const list = $('.pal-list'); if (list) { list.innerHTML = tmp.querySelector('.pal-list')!.innerHTML; bindRows(); moveHL(true); } ctx.hitbox(sh); return;
      }
      html = palHTML(); sig = 'pal' + Math.random(); o.hidden = false;
    } else if (s.pending.length) { const it = s.pending[0], danger = it.danger && cfg.hold; html = `<div class="c-toast" data-act="palette"><span class="ic" style="--c:${COLORS.permission}"><span class="ms">front_hand</span></span><span class="t"><b>${esc(it.project)}</b> <span class="muted">quiere ejecutar</span> <span class="mono" style="font-size:12px;color:#ffd590">${esc(it.command)}</span></span>${cfg.keys ? `<span class="row muted" style="gap:5px;font-size:11.5px;margin-left:4px"><kbd data-k="y">Y</kbd>${danger ? 'mantener' : ''}<kbd data-k="n">N</kbd><kbd>↵</kbd></span>` : ''}${cfg.countdown ? '<i class="cd" style="left:14px;right:14px"></i>' : ''}${danger ? '<i class="hold"></i>' : ''}</div>`; sig = 'p' + it.id; o = { ...o, hidden: false, item: it }; }
    else if (tr) { if (tr.k === 'release') html = releaseCard(tr.items ?? []); else { const p = trPill(tr); html = `<div class="c-toast"><span class="ic" style="--c:${p.c}"><span class="ms ${['done', 'ok'].includes(tr.k) ? 'draw' : ''}">${p.ic}</span></span><span class="t">${p.txt}</span><i class="tm"></i></div>`; } sig = 't' + tr.id; o = { ...o, hidden: false, life: 2600 }; }
    const r = morph(m, sig, html, o);
    if (r.changed) ctx.hitbox(html ? sh : null);
    if (open) {
      const inp = $<HTMLInputElement>('input'); if (!inp) return;
      inp.addEventListener('input', () => { q = inp.value; sel = 0; render(); });
      inp.addEventListener('keydown', e => {
        e.stopPropagation();
        if (e.key === 'ArrowDown') { e.preventDefault(); sel = Math.min(items.length - 1, sel + 1); moveHL(); }
        else if (e.key === 'ArrowUp') { e.preventDefault(); sel = Math.max(0, sel - 1); moveHL(); }
        else if (e.key === 'Enter') { e.preventDefault(); run(sel); }
        else if (e.key === 'Escape') { e.preventDefault(); close(); }
      });
      bindRows(); requestAnimationFrame(() => moveHL(true));
      setTimeout(() => { const i = $<HTMLInputElement>('input'); if (i && document.activeElement !== i) { i.focus(); i.setSelectionRange(i.value.length, i.value.length); } }, 40);
    }
  }
  sh.addEventListener('click', e => { if ((e.target as HTMLElement).closest('[data-act="palette"]')) openPal(); });
  sh.addEventListener('focusout', e => { if (open && !sh.contains(e.relatedTarget as Node)) setTimeout(() => { if (open && !sh.contains(document.activeElement)) close(); }, 120); });
  window.addEventListener('blur', () => { if (open) close(); });
  return {
    render: () => render(), open: openPal, isOpen: () => open,
    light(kind: string) { const c = (COLORS as Record<string, string>)[kind] || '#fff'; beam(board, c, board.clientWidth / 2, ctx.cfg().island.beam); setTimeout(() => ignite(sh, c), T(540)); },
    destroy() { if (open) ctx.islandFocus(false); ctx.hitbox(null); sh.remove(); },
  };
}
