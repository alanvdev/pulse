// Modo Orbe (spec §3.2, artboards Orbe y Casos): un punto de 24 px a la izquierda, al centro o a la derecha que se abre en avisos, menú y tarjetas.
import { dotKind, statusLine, esc, COLORS } from '../shared/format';
import { morph, reduceMotion, SP_MOVE, T, type Morph, type MorphOpts } from '../shared/motion';
import type { Ctx } from './comando';
import { focusLine, nearestZone, orbDot, type Side } from './model';
import { questionCard } from './questions';
import { beam, beat, fitCommand, ignite, noticeHTML, permCard, shake } from './shared';

export interface OrbeCtx extends Ctx { setSide(side: Side): void }

/** "1 te necesita · 1 trabaja": la primera parte en tx, el resto en tx3. */
const twoTone = (line: string) => { const [a, ...b] = line.split(' · '); return `<span>${esc(a)}</span>${b.length ? `<span class="tx3">· ${esc(b.join(' · '))}</span>` : ''}`; };

export function mountOrbe(board: HTMLElement, ctx: OrbeCtx) {
  const sh = document.createElement('div'); sh.className = 'shell glass o-shell'; sh.tabIndex = 0;
  const tip = document.createElement('div'); tip.className = 'o-tip';
  const zones = document.createElement('div'); zones.className = 'o-zones'; zones.setAttribute('aria-hidden', 'true'); zones.innerHTML = '<span></span><span></span><span></span>';
  board.append(zones, sh, tip);
  const m: Morph = { el: sh, w: 0, h: 0, sig: '', op: 1 };
  let side: Side = ctx.cfg().island.orb_side, hover = false, menu = false, state = 'c';   // state: primera letra de la firma (c punto, h resumen, m menú, t aviso, p tarjeta)
  const place = () => { sh.classList.remove('left', 'right', 'center'); sh.classList.add(side); };
  place();
  sh.addEventListener('mouseenter', () => { hover = true; render(); });
  sh.addEventListener('mouseleave', () => { hover = false; menu = false; setTimeout(render, 10); });

  // arrastre (con las tres zonas marcadas) y clic
  let down = false, sx = 0, moved = false;
  const frac = (clientX: number) => { const b = board.getBoundingClientRect(); return (clientX - b.left) / b.width; };
  const hot = (z: Side | null) => [...zones.children].forEach((c, i) => c.classList.toggle('hot', (['left', 'center', 'right'] as Side[])[i] === z));
  sh.addEventListener('pointerdown', e => { if ((e.target as HTMLElement).closest('button') || state === 'p') return; /* en una tarjeta, el clic es de la tarjeta */ down = true; moved = false; sx = e.clientX; sh.setPointerCapture(e.pointerId); });
  sh.addEventListener('pointermove', e => {
    if (!down) return;
    const dx = e.clientX - sx;
    if (Math.abs(dx) > 5 && !moved && (state === 'c' || state === 'h')) { moved = true; menu = false; sh.classList.add('drag'); zones.classList.add('on'); ctx.hitbox(board); render(); }
    if (moved) { sh.style.transform = `translateX(${dx}px) scale(1.12)`; const r = sh.getBoundingClientRect(); hot(nearestZone(frac(r.left + r.width / 2))); }
  });
  const up = (e: PointerEvent) => {
    if (!down) return; down = false;
    if (moved) {
      const r = sh.getBoundingClientRect(), ns = nearestZone(frac(r.left + r.width / 2)), before = r;
      sh.style.transform = ''; sh.classList.remove('drag'); zones.classList.remove('on'); hot(null);
      side = ns; ctx.setSide(ns); place();
      const after = sh.getBoundingClientRect();
      if (!reduceMotion()) sh.animate([{ transform: `translateX(${before.left - after.left}px) scale(1.12)` }, { transform: 'none' }], { duration: T(SP_MOVE.d), easing: SP_MOVE.e });
      setTimeout(() => ctx.hitbox(sh), T(SP_MOVE.d));
    } else if (!(e.target as HTMLElement).closest('button')) { menu = !menu; render(); }
  };
  sh.addEventListener('pointerup', up); sh.addEventListener('pointercancel', up);
  sh.addEventListener('keydown', e => {
    // con una tarjeta abierta, Enter o Espacio no la cambian por el menú (se perdería lo elegido en una pregunta)
    if ((e.key === 'Enter' || e.key === ' ') && e.target === sh && state !== 'p' && state !== 't') { e.preventDefault(); menu = !menu; render(); }
    if (e.key === 'Escape' && state === 'p' && !menu) { e.preventDefault(); ctx.islandFocus(false); }
    if (e.key === 'Escape' && menu) { e.preventDefault(); menu = false; render(); }
  });
  sh.addEventListener('click', e => {
    const a = (e.target as HTMLElement).closest<HTMLElement>('[data-act]'); if (!a) return;
    const s = ctx.snap(), now = Date.now(), id = Number(a.dataset.id);
    switch (a.dataset.act) {
      case 'focus': ctx.setFocus(!s.focus); break;
      case 'muteall': { const all = s.sessions.length > 0 && s.sessions.every(x => x.muted_until_ms > now); s.sessions.forEach(x => ctx.muteSession(x.id, all ? 0 : 30)); break; }
      case 'allowall': ctx.allowAll(); break;
      case 'open': ctx.openDashboard(); break;
      case 'allow': if (a.dataset.danger !== '1') ctx.decide(id, true); break;
      case 'deny': ctx.decide(id, false); break;
    }
  });

  const bead = (k: ReturnType<typeof orbDot>) => `<div class="o-bead">${k === 'moon' ? '<span class="ms">dark_mode</span>' : `<span class="dot ${k === 'none' ? 'calm' : k}"></span>`}</div>`;
  function render() {
    const s = ctx.snap(), cfg = ctx.cfg(), n = ctx.notice(), now = Date.now(), top = s.pending[0];
    // "Lugar del Orbe" cambiado en Mosaico: se mueve ya. morph() no ve un cambio de lado, así que la zona de clics se manda aquí.
    if (!down && cfg.island.orb_side !== side) { side = cfg.island.orb_side; place(); ctx.hitbox(sh); }
    const allMuted = s.sessions.length > 0 && s.sessions.every(x => x.muted_until_ms > now);
    let html: string, sig: string, stacked = false;
    const o: MorphOpts = { radius: (_w, h) => h <= 26 ? '12px' : h > 44 ? '14px' : '10px' };
    // la pregunta no se rehace si llega otro pendiente: lo elegido vive en la tarjeta
    if (top && !menu) { html = top.kind === 'question' ? questionCard(top, cfg.island.countdown) : permCard(top, cfg, s.pending, 390); sig = top.kind === 'question' ? `pq${top.id}` : `p${top.id}:${s.pending.length}`; stacked = top.kind !== 'question' && s.pending.length > 1; }
    else if (menu) {
      const btn = (ic: string, label: string, act: string, on: boolean) => `<button type="button" class="o-btn" data-act="${act}" data-tip="${label}" aria-label="${label}" aria-pressed="${on}"><span class="ms">${ic}</span></button>`;
      html = `<div class="o-menu">${btn('dark_mode', 'Modo foco', 'focus', s.focus)}${btn('notifications_paused', 'Silenciar 30 min', 'muteall', allMuted)}${btn('done_all', 'Aprobar todo', 'allowall', false)}${btn('open_in_new', 'Abrir Pulse', 'open', false)}</div>`;
      sig = 'm' + s.focus + allMuted;
    }
    else if (n) { html = noticeHTML(n); sig = 't' + n.id; o.life = n.life; }
    else if (hover) {
      const dots = s.sessions.slice(0, 6).map(x => `<span class="dot ${s.pending.some(p => p.session_id === x.id) ? 'halo' : dotKind(x.state)}"></span>`).join('');
      html = s.focus ? `<div class="o-peek"><span class="ms">dark_mode</span>${twoTone(focusLine(s))}</div>` : `<div class="o-peek">${dots}<span class="o-line">${twoTone(statusLine(s))}</span><kbd class="kbd">Clic</kbd></div>`;
      sig = 'h' + statusLine(s) + s.focus + s.deferred.length + s.sessions.map(x => x.state).join('');
    }
    else { const k = orbDot(s); html = bead(k); sig = 'c' + k; }
    ctx.shown(top && !menu && top.kind === 'permission' ? top.id : null);
    sh.dataset.k = sig.startsWith('c') ? 'dot' : 'open';
    state = sig[0];
    // es botón solo como punto o resumen; abierto contiene botones propios que el lector de pantalla debe alcanzar
    if (state === 'c' || state === 'h') { sh.setAttribute('role', 'button'); sh.setAttribute('aria-expanded', 'false'); } else { sh.removeAttribute('role'); sh.removeAttribute('aria-expanded'); }
    sh.classList.toggle('none', sig === 'cnone');
    sh.classList.toggle('stacked', stacked);
    sh.setAttribute('aria-label', `Pulse: ${s.focus ? focusLine(s) : statusLine(s)}`);
    const r = morph(m, sig, html, o);
    if (r.changed) { ctx.hitbox(sh); fitCommand(sh); }
    sh.querySelectorAll<HTMLElement>('[data-tip]').forEach(b => {
      b.onmouseenter = b.onfocus = () => { const rr = b.getBoundingClientRect(), br = board.getBoundingClientRect(); tip.textContent = b.dataset.tip!; tip.style.left = (rr.left - br.left + rr.width / 2) + 'px'; tip.style.translate = '-50% 0'; tip.classList.add('on'); };
      b.onmouseleave = b.onblur = () => tip.classList.remove('on');
      b.onclick = () => { tip.classList.remove('on'); menu = false; setTimeout(render, 30); };
    });
  }
  return {
    render, openMenu() {
      // Ctrl+Alt+Espacio o «Responder permiso» con algo esperando: el teclado va a esa tarjeta, no al menú que la taparía
      if (ctx.snap().pending.length) { menu = false; ctx.islandFocus(true); render(); requestAnimationFrame(() => sh.querySelector<HTMLElement>('.layer:last-child :is(.qopt, .cmd)')?.focus({ preventScroll: true })); return; }
      menu = true; ctx.islandFocus(true); render(); requestAnimationFrame(() => sh.querySelector<HTMLElement>('.o-btn')?.focus());
    },
    light(kind: string) {
      const r = sh.getBoundingClientRect(), b = board.getBoundingClientRect(), c = (COLORS as Record<string, string>)[kind] || COLORS.start;
      beam(board, c, r.left - b.left + r.width / 2, ctx.cfg().island.beam);
      setTimeout(() => { ignite(sh, c); if (kind === 'permission') beat(sh); if (kind === 'error') shake(sh); }, T(540));
    },
    destroy() { ctx.hitbox(null); sh.remove(); tip.remove(); zones.remove(); },
  };
}
