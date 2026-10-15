// Modo Comando (spec §3.1, artboards Comando y Puntos): puntos de sesión, resumen al pasar el mouse, avisos, tarjeta de permiso y paleta.
import type { Config, DaySummary, Snapshot } from '../shared/bridge';
import { clock, COLORS, esc, LBL } from '../shared/format';
import { morph, reduceMotion, SP_OPEN, T, type Morph, type MorphOpts } from '../shared/motion';
import { hasChroma } from '../shared/platform';
import { dayLine, inactiveIds, PAL_EMPTY, sessionRows, showDots, type Notice, type Row } from './model';
import { questionCard } from './questions';
import { beam, fitCommand, fuzzy, ignite, keys, noticeHTML, permCard } from './shared';

export interface Ctx {
  snap(): Snapshot; cfg(): Config; notice(): Notice | null; day(): DaySummary | null; refreshDay(): void;
  decide(id: number, allow: boolean): void; answer(id: number, answers: Record<string, string>): void; skip(id: number): void; hold(id: number, on: boolean): void;
  allowAll(): void; setFocus(on: boolean): void; muteSession(id: string, minutes: number): void; forgetSessions(ids: string[]): void; openDashboard(view?: string): void;
  islandFocus(on: boolean): void; hitbox(el: HTMLElement | null): void; shown(id: number | null): void;
}
type Item = { k: string; g: string; icon: string; text: string; meta: string; run: () => void };
const SETTINGS = ([['isla', 'la isla'], ['pulsos', 'pulsos'], ['sonidos', 'sonidos'], ['foco', 'foco'], ['teclado', 'teclado'], ['apariencia', 'apariencia']] as [string, string][]).filter(([id]) => hasChroma || id !== 'teclado');
/** Posición de `el` dentro de `stop` (la capa), sumando offsets. */
const offsetIn = (el: HTMLElement, stop: HTMLElement) => { let x = 0, y = 0, n: HTMLElement | null = el; while (n && n !== stop) { x += n.offsetLeft; y += n.offsetTop; n = n.offsetParent as HTMLElement | null; } return [x, y]; };

export function mountComando(board: HTMLElement, ctx: Ctx) {
  const sh = document.createElement('div'); sh.className = 'shell glass c-shell'; sh.style.opacity = '0';
  // Los puntos viven fuera de las capas del morph para viajar de la cápsula a su fila sin desaparecer.
  const rail = document.createElement('div'); rail.className = 'rail'; rail.setAttribute('aria-hidden', 'true');
  sh.append(rail); board.append(sh);
  const m: Morph = { el: sh, w: 0, h: 0, sig: '', op: 0 };
  // `selK` sigue a la fila elegida: si llega o se resuelve un permiso, las filas se corren y un Enter con solo el índice
  // aprobaría un comando que no estaba seleccionado
  let open = false, q = '', sel = 0, selK = '', items: Item[] = [], hover = false, leaveT = 0, palList = '';
  const collapsed = new Set<number>();   // tarjetas recogidas con Esc: se vuelven a abrir desde el resumen
  const $ = <E extends Element = HTMLElement>(s: string) => sh.querySelector<E>(s);
  const $$ = (s: string) => [...sh.querySelectorAll<HTMLElement>(s)];

  /* ---------- paleta ---------- */
  function openPal() { open = true; q = ''; sel = 0; selK = ''; hover = false; ctx.islandFocus(true); render(); requestAnimationFrame(() => $<HTMLInputElement>('input')?.focus({ preventScroll: true })); }
  function close() { open = false; ctx.islandFocus(false); render(); }
  function build(): Item[] {
    const L: Item[] = [], s = ctx.snap(), now = Date.now(), hold = ctx.cfg().island.hold;
    // un comando delicado solo se aprueba manteniendo «Mantener» en su tarjeta: la paleta no lo permite con un Enter
    const perms = s.pending.filter(p => p.kind === 'permission'), quick = perms.filter(p => !(hold && p.danger)).length;
    perms.forEach(p => {
      if (!(hold && p.danger)) L.push({ k: `allow${p.id}`, g: 'Pendiente', icon: 'check', text: `Permitir ${p.command}`, meta: p.project, run: () => ctx.decide(p.id, true) });
      L.push({ k: `deny${p.id}`, g: 'Pendiente', icon: 'close', text: `Denegar ${p.command}`, meta: p.project, run: () => ctx.decide(p.id, false) });
    });
    L.push({ k: 'focus', g: 'Acciones', icon: 'dark_mode', text: s.focus ? 'Salir del modo foco' : 'Activar modo foco', meta: '', run: () => ctx.setFocus(!s.focus) });
    const allMuted = s.sessions.length > 0 && s.sessions.every(x => x.muted_until_ms > now);
    if (s.sessions.length) L.push({ k: 'mute', g: 'Acciones', icon: allMuted ? 'notifications_active' : 'notifications_paused', text: allMuted ? 'Quitar silencio' : 'Silenciar 30 min', meta: '', run: () => s.sessions.forEach(x => ctx.muteSession(x.id, allMuted ? 0 : 30)) });
    const quiet = inactiveIds(s);
    if (quiet.length) L.push({ k: 'forget', g: 'Acciones', icon: 'clear_all', text: quiet.length === 1 ? 'Quitar la sesión inactiva' : `Quitar ${quiet.length} sesiones inactivas`, meta: '', run: () => ctx.forgetSessions(quiet) });
    L.push({ k: 'allowall', g: 'Acciones', icon: 'done_all', text: 'Aprobar todos los permisos', meta: quick ? String(quick) : '', run: ctx.allowAll });
    L.push({ k: 'open', g: 'Acciones', icon: 'open_in_new', text: 'Abrir Pulse', meta: '', run: () => ctx.openDashboard() });
    s.sessions.forEach(x => L.push({ k: `s${x.id}`, g: 'Sesiones', icon: 'stacks', text: x.project, meta: LBL[x.state], run: () => ctx.openDashboard(`sesiones/${x.id}`) }));
    SETTINGS.forEach(([id, name]) => L.push({ k: `a${id}`, g: 'Ajustes', icon: 'tune', text: `Ajustes de ${name}`, meta: 'Ajustes', run: () => ctx.openDashboard(`ajustes/${id}`) }));
    return L;
  }
  function palHTML() {
    const all = build(); items = [];
    let g = '', rows = '';
    all.forEach(it => {
      const f = fuzzy(it.text, q); if (!f.ok) return;
      const n = items.length; items.push(it);
      if (it.g !== g) { g = it.g; rows += `<div class="pal-g">${g}</div>`; }
      rows += `<div class="pi" role="option" data-n="${n}" aria-selected="false"><span class="ms">${it.icon}</span><span class="pi-t">${f.html}</span><span class="pi-r"><small>${esc(it.meta)}</small><kbd class="kbd">↵</kbd></span></div>`;
    });
    // la fila elegida sigue elegida aunque cambie de lugar; si desapareció (se resolvió en otro lado), la selección pasa
    // a la primera acción y no a otro permiso que quedó en su lugar
    const at = selK ? items.findIndex(it => it.k === selK) : sel, calm = items.findIndex(it => it.g !== 'Pendiente');
    sel = at >= 0 ? at : calm >= 0 ? calm : 0;
    sel = Math.min(sel, Math.max(0, items.length - 1));
    selK = items[sel]?.k ?? '';
    return `<div class="pal"><div class="pal-in"><span class="ms">search</span><input type="text" value="${esc(q)}" placeholder="Busca un permiso, una sesión o una acción" aria-label="Buscar en Pulse">${keys(['Esc'])}</div><div class="pal-list" role="listbox" aria-label="Resultados"><i class="pal-hl"></i>${rows || `<div class="pal-empty">${PAL_EMPTY}</div>`}</div></div>`;
  }
  function moveHL(instant?: boolean) {
    const hl = $('.pal-hl'), row = $(`.pi[data-n="${sel}"]`); if (!hl) return;
    if (!row) { hl.style.opacity = '0'; return; }
    hl.style.opacity = '1';
    if (instant) { hl.style.transition = 'none'; hl.style.transform = `translateY(${row.offsetTop}px)`; void hl.offsetWidth; hl.style.transition = ''; } else hl.style.transform = `translateY(${row.offsetTop}px)`;
    $$('.pi').forEach(r => r.setAttribute('aria-selected', String(+r.dataset.n! === sel)));
    // solo la lista se desplaza (scrollIntoView movería también la cápsula mientras crece)
    const l = $('.pal-list')!;
    if (row.offsetTop < l.scrollTop) l.scrollTop = row.offsetTop; else if (row.offsetTop + row.offsetHeight > l.scrollTop + l.clientHeight) l.scrollTop = row.offsetTop + row.offsetHeight - l.clientHeight;
  }
  function run(n: number) {
    const it = items[n]; if (!it) return;
    setTimeout(() => { open = false; ctx.islandFocus(false); it.run(); render(); }, T(reduceMotion() ? 0 : 120));
  }
  function pick(n: number) { sel = n; selK = items[n]?.k ?? ''; moveHL(); }
  function bindRows() { $$('.pi').forEach(r => { r.onmouseenter = () => pick(+r.dataset.n!); r.onclick = () => run(+r.dataset.n!); }); }
  function bindPalette() {
    const inp = $<HTMLInputElement>('input'); if (!inp) return;
    inp.addEventListener('input', () => { q = inp.value; sel = 0; selK = ''; render(); });
    inp.addEventListener('keydown', e => {
      e.stopPropagation();
      if (e.key === 'ArrowDown') { e.preventDefault(); pick(Math.min(items.length - 1, sel + 1)); }
      else if (e.key === 'ArrowUp') { e.preventDefault(); pick(Math.max(0, sel - 1)); }
      else if (e.key === 'Enter') { e.preventDefault(); run(sel); }
      else if (e.key === 'Escape') { e.preventDefault(); close(); }
    });
    bindRows(); requestAnimationFrame(() => moveHL(true));
    setTimeout(() => { const i = $<HTMLInputElement>('input'); if (i && document.activeElement !== i) { i.focus({ preventScroll: true }); i.setSelectionRange(i.value.length, i.value.length); } }, 40);
  }

  /* ---------- puntos y resumen ---------- */
  /** "Terminó hace N s" cambia cada segundo: no entra en la firma (rehacer la capa cada segundo parpadea), se actualiza en su lugar. */
  const ago = (r: Row) => r.state === 'Done' && !r.need;
  const slot = (r: Row) => `<i class="slot" data-sid="${esc(r.id)}" data-dot="${r.dot}"></i>`;
  const dotsHTML = (rows: Row[], focus: boolean) => `<div class="c-dots">${rows.map(slot).join('')}${focus ? '<span class="ms c-moon" role="img" aria-label="En foco">dark_mode</span>' : ''}</div>`;
  const summaryHTML = (rows: Row[], now: number) => `<div class="sum"><div class="sum-rows">${rows.map((r, i) =>
    `<div class="srow${r.need ? ' need' : ''}" data-sid="${esc(r.id)}"${r.pendingId ? ` data-pid="${r.pendingId}"` : ''} style="--i:${i}">${slot(r)}<span class="sp">${esc(r.project)}</span><span class="what${r.mono ? ' mono' : ''}"${ago(r) ? ' data-ago' : ''}>${esc(r.what)}</span>${r.need ? '<kbd class="kbd">↵</kbd>' : `<span class="mono st"${r.state === 'Done' ? '' : ` data-since="${r.since}"`}>${r.state === 'Done' ? '' : clock(now - r.since)}</span>`}</div>`).join('')}</div>
    <div class="sum-foot"><span>${esc(dayLine(ctx.day()))}</span><span class="sum-k">Paleta ${keys(['Ctrl', 'Alt', 'Espacio'])}</span></div></div>`;
  /** Lleva cada punto del riel a su hueco en la capa nueva (cápsula ↔ filas) con el resorte de apertura. */
  function placeDots() {
    const layer = sh.querySelector<HTMLElement>('.layer:last-child'), slots = layer ? [...layer.querySelectorAll<HTMLElement>('.slot')] : [], seen = new Set<string>();
    for (const sl of slots) {
      const sid = sl.dataset.sid!; seen.add(sid);
      let d = [...rail.children].find(x => (x as HTMLElement).dataset.sid === sid) as HTMLElement | undefined;
      const fresh = !d;
      if (!d) { d = document.createElement('i'); d.dataset.sid = sid; rail.append(d); }
      d.className = `dot ${sl.dataset.dot}`;
      const [x, y] = offsetIn(sl, layer!), to = `translate(${x}px,${y}px)`, from = d.style.transform;
      d.style.transform = to;
      if (reduceMotion()) continue;
      if (!fresh && from && from !== to) d.animate([{ transform: from }, { transform: to }], { duration: T(SP_OPEN.d), easing: SP_OPEN.e });
      if (fresh) d.animate([{ opacity: 0, scale: .4 }, { opacity: 1, scale: 1 }], { duration: T(220), easing: 'ease-out' });
    }
    [...rail.children].forEach(d => { if (!seen.has((d as HTMLElement).dataset.sid!)) d.remove(); });
    rail.style.opacity = slots.length ? '1' : '0';
  }
  function openRow(row: HTMLElement) {
    hover = false;
    if (row.dataset.pid) { collapsed.delete(Number(row.dataset.pid)); render(); }
    else { render(); ctx.openDashboard(`sesiones/${row.dataset.sid}`); }
  }

  /* ---------- render ---------- */
  function render() {
    const s = ctx.snap(), cfg = ctx.cfg(), n = ctx.notice(), now = Date.now();
    for (const id of collapsed) if (!s.pending.some(p => p.id === id)) collapsed.delete(id);
    const live = s.pending.filter(p => !collapsed.has(p.id)), top = live[0], rows = sessionRows(s, now);
    let html = '', sig = 'x', stacked = false;
    const o: MorphOpts = { radius: (_w, h) => h > 44 ? '14px' : h > 30 ? '10px' : '8px', hidden: true, dy: -4 };
    if (open) {
      // Refresca la lista sin perder el foco del campo, y solo si cambió: el tic de cada segundo la rehacía, y un clic
      // que caía entre el mousedown y el mouseup se perdía. Se compara con lo generado (moveHL escribe estilos en .pal-hl).
      if (m.sig.startsWith('pal')) {
        const tmp = document.createElement('div'); tmp.innerHTML = palHTML();
        const fresh = tmp.querySelector('.pal-list')!.innerHTML, list = $('.pal-list');
        if (list && fresh !== palList) { palList = fresh; list.innerHTML = fresh; bindRows(); moveHL(true); ctx.hitbox(sh); }
        return;
      }
      // la lista recién armada es la referencia: así el primer tick no la reemplaza (y no se pierde un clic)
      html = palHTML(); { const t = document.createElement('div'); t.innerHTML = html; palList = t.querySelector('.pal-list')?.innerHTML ?? ''; } sig = 'pal' + Math.random(); o.hidden = false;
    }
    else if (top?.kind === 'question') { html = questionCard(top, cfg.island.countdown); sig = 'Q' + top.id; o.hidden = false; }
    else if (top) { html = permCard(top, cfg, live); sig = `P${top.id}:${live.length}`; o.hidden = false; stacked = live.length > 1; }
    else if (n) { html = noticeHTML(n); sig = 'n' + n.id; o.hidden = false; o.life = n.life; }
    else if (hover && rows.length) { html = summaryHTML(rows, now); sig = 'S' + rows.map(r => r.id + r.dot + (ago(r) ? 'done' + r.since : r.what)).join('|') + dayLine(ctx.day()); o.hidden = false; }
    else if (rows.length && (showDots(s, cfg) || collapsed.size)) { html = dotsHTML(rows, s.focus); sig = 'D' + rows.map(r => r.id + r.dot).join('|') + s.focus; o.hidden = false; }
    ctx.shown(!open && top?.kind === 'permission' ? top.id : null);
    sh.classList.toggle('stacked', stacked);
    const r = morph(m, sig, html, o);
    if (r.changed) { ctx.hitbox(html ? sh : null); fitCommand(sh); }
    else {
      sh.querySelectorAll<HTMLElement>('.layer:last-child [data-since]').forEach(e => { e.textContent = clock(now - Number(e.dataset.since)); });
      sh.querySelectorAll<HTMLElement>('.layer:last-child [data-ago]').forEach(e => { const w = rows.find(r => r.id === e.closest<HTMLElement>('.srow')?.dataset.sid)?.what; if (w && e.textContent !== w) e.textContent = w; });
    }
    placeDots();
    if (open && r.changed) bindPalette();
  }

  /* ---------- eventos ---------- */
  sh.addEventListener('click', e => {
    const t = e.target as HTMLElement, a = t.closest<HTMLElement>('[data-act]');
    if (a?.dataset.act === 'deny') { ctx.decide(Number(a.dataset.id), false); return; }
    if (a?.dataset.act === 'allow') { if (a.dataset.danger !== '1') ctx.decide(Number(a.dataset.id), true); return; }
    const row = t.closest<HTMLElement>('.srow'); if (row) openRow(row);
  });
  // al pasar el mouse por los puntos se abre el resumen; al salir se cierra en 250 ms
  sh.addEventListener('mouseenter', () => { clearTimeout(leaveT); if (!hover && m.sig.startsWith('D')) { hover = true; ctx.refreshDay(); render(); } });
  sh.addEventListener('mouseleave', () => { clearTimeout(leaveT); leaveT = window.setTimeout(() => { if (hover) { hover = false; render(); } }, 250); });
  const onKey = (e: KeyboardEvent) => {
    if (open || (e.target as HTMLElement).closest('.qcard')) return;   // la paleta y las preguntas manejan sus teclas
    if (e.key === 'Escape') {
      if (hover) { e.preventDefault(); hover = false; render(); return; }
      const card = (document.activeElement as HTMLElement | null)?.closest<HTMLElement>('[data-pid]');
      if (card) { e.preventDefault(); collapsed.add(Number(card.dataset.pid)); ctx.islandFocus(false); render(); }
    }
    if (e.key === 'Enter' && hover) { const row = $('.layer:last-child .srow.need'); if (row) { e.preventDefault(); openRow(row); } }
  };
  window.addEventListener('keydown', onKey);
  sh.addEventListener('focusout', e => { if (open && !sh.contains(e.relatedTarget as Node)) setTimeout(() => { if (open && !sh.contains(document.activeElement)) close(); }, 120); });
  const onBlur = () => { if (open) close(); };
  window.addEventListener('blur', onBlur);
  return {
    render, open: openPal,
    light(kind: string) { const c = (COLORS as Record<string, string>)[kind] || COLORS.start; beam(board, c, board.clientWidth / 2, ctx.cfg().island.beam); setTimeout(() => ignite(sh, c), T(540)); },
    destroy() { if (open) ctx.islandFocus(false); window.removeEventListener('keydown', onKey); window.removeEventListener('blur', onBlur); ctx.hitbox(null); sh.remove(); },
  };
}
