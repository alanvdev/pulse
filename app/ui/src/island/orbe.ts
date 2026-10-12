// Modo Orbe (versión mínima: punto de 6 px + satélite), portado de `Orbe(board)` en docs/artboards/isla.html.
import type { SessionState } from '../shared/bridge';
import { COLORS, stateColor } from '../shared/format';
import { morph, reduceMotion, SP_MOVE, T, type Morph, type MorphOpts } from '../shared/motion';
import type { Ctx } from './comando';
import { questionCard } from './questions';
import { beam, beat, ignite, noticeHTML, permCard, shake } from './shared';

export interface OrbeCtx extends Ctx { setSide(side: 'left' | 'center' | 'right'): void }

const agg = (states: SessionState[]): SessionState => { const r = { Idle: 0, Done: 1, Working: 2, IdleWait: 3, Waiting: 4 }; return states.reduce<SessionState>((a, s) => r[s] > r[a] ? s : a, 'Idle'); };

export function mountOrbe(board: HTMLElement, ctx: OrbeCtx) {
  const sh = document.createElement('div'); sh.className = 'shell glass o-shell'; sh.tabIndex = 0; sh.setAttribute('role', 'button'); sh.setAttribute('aria-label', 'Pulse');
  const tip = document.createElement('div'); tip.className = 'o-tip';
  board.append(sh, tip);
  const m: Morph = { el: sh, w: 0, h: 0, sig: '', op: 1 };
  let side = ctx.cfg().island.orb_side, hover = false, menu = false;
  const place = () => { sh.classList.remove('left', 'right', 'center'); sh.classList.add(side); };
  place();
  const onEnter = () => { hover = true; render(); };
  const onLeave = () => { hover = false; menu = false; setTimeout(render, 10); };
  sh.addEventListener('mouseenter', onEnter);
  sh.addEventListener('mouseleave', onLeave);
  // arrastre + clic
  let down = false, sx = 0, moved = false;
  sh.addEventListener('pointerdown', e => { if ((e.target as HTMLElement).closest('button')) return; down = true; moved = false; sx = e.clientX; sh.setPointerCapture(e.pointerId); });
  sh.addEventListener('pointermove', e => { if (!down) return; const dx = e.clientX - sx; if (Math.abs(dx) > 5 && !moved) { moved = true; menu = false; sh.classList.add('drag'); ctx.hitbox(board); render(); } if (moved) sh.style.transform = `translateX(${dx}px) scale(1.12)`; });
  const up = (e: PointerEvent) => {
    if (!down) return; down = false;
    if (moved) {
      const r = sh.getBoundingClientRect(), b = board.getBoundingClientRect(), x = (r.left + r.width / 2 - b.left) / b.width;
      const ns = x < .33 ? 'left' : x > .66 ? 'right' : 'center';
      const before = sh.getBoundingClientRect();
      sh.style.transform = ''; sh.classList.remove('drag'); side = ns; ctx.setSide(ns); place();
      const after = sh.getBoundingClientRect();
      if (!reduceMotion()) sh.animate([{ transform: `translateX(${before.left - after.left}px) scale(1.12)` }, { transform: 'none' }], { duration: T(SP_MOVE.d), easing: SP_MOVE.e });
      setTimeout(() => ctx.hitbox(sh), T(SP_MOVE.d));
    } else if (!(e.target as HTMLElement).closest('button')) { menu = !menu; render(); }
  };
  sh.addEventListener('pointerup', up); sh.addEventListener('pointercancel', up);
  sh.addEventListener('keydown', e => { if ((e.key === 'Enter' || e.key === ' ') && e.target === sh) { e.preventDefault(); menu = !menu; render(); } });
  sh.addEventListener('click', e => {
    const a = (e.target as HTMLElement).closest<HTMLElement>('[data-act]'); if (!a) return;
    const s = ctx.snap(), now = Date.now(), id = Number(a.dataset.id);
    switch (a.dataset.act) {
      case 'focus': ctx.setFocus(!s.focus); break;
      case 'muteall': { const all = s.sessions.length > 0 && s.sessions.every(x => x.muted_until_ms > now); s.sessions.forEach(x => ctx.muteSession(x.id, all ? 0 : 30)); break; }
      case 'allowall': ctx.allowAll(); break;
      case 'center': ctx.openDashboard(); break;
      case 'allow': if (a.dataset.danger !== '1') ctx.decide(id, true); break;
      case 'deny': ctx.decide(id, false); break;
    }
  });
  const core = (st: SessionState, focus: boolean) => `<div class="o-core ${st === 'Working' ? 'work' : ''} ${st === 'Waiting' ? 'wait' : ''}" style="--c:${focus ? COLORS.focus : st === 'Idle' ? '#9aa1ab' : stateColor(st)}"><span class="o-orbit"><i></i></span><i class="o-dot"></i></div>`;
  function render() {
    const s = ctx.snap(), cfg = ctx.cfg(), n = ctx.notice(), now = Date.now();
    const st = agg(s.sessions.map(x => x.state));
    const allMuted = s.sessions.length > 0 && s.sessions.every(x => x.muted_until_ms > now);
    let html: string, sig: string; const o: MorphOpts = { radius: (_w, h) => h > 40 ? '16px' : '99px' };
    if (s.pending.length && !menu) { const it = s.pending[0]; html = it.kind === 'question' ? questionCard(it, cfg.island.countdown) : permCard(it, cfg, s.pending, 390); sig = 'p' + it.id + s.pending.length; }
    else if (menu) { html = `<div class="o-menu">${core(st, s.focus)}${([['dark_mode', 'Modo foco', 'focus', s.focus], ['volume_off', 'Silenciar 30 min', 'muteall', allMuted], ['done_all', 'Aprobar todo', 'allowall', false], ['dashboard', 'Abrir tablero', 'center', false]] as const).map(([ic, l, a, on]) => `<button type="button" data-act="${a}" data-tip="${l}" aria-label="${l}" aria-pressed="${on}"><span class="ms">${ic}</span></button>`).join('')}</div>`; sig = 'm' + s.focus + allMuted; }
    else if (n) { html = noticeHTML(n); sig = 't' + n.id; o.life = n.life; }
    else if (hover) { const w = s.sessions.filter(x => x.state === 'Working').length, need = s.sessions.filter(x => x.state === 'IdleWait').length; html = `<div class="o-peek">${core(st, s.focus)}<span class="t">${s.focus ? `<b>En foco</b> <span class="muted">${s.deferred.length} guardados</span>` : w ? `<b>${w}</b> trabajando${need ? ` <span class="muted">· ${need} te espera</span>` : ''}` : need ? `<b>${need}</b> te espera` : '<span class="muted">Todo en calma</span>'}</span></div>`; sig = 'h' + s.sessions.map(x => x.state).join('') + s.focus + s.deferred.length; }
    else { html = core(st, s.focus); sig = 'c' + st + s.focus; }
    sh.dataset.k = sig.startsWith('c') ? 'dot' : 'open';
    const r = morph(m, sig, html, o);
    if (r.changed) ctx.hitbox(sh);
    sh.querySelectorAll<HTMLElement>('[data-tip]').forEach(b => { b.onmouseenter = () => { const rr = b.getBoundingClientRect(), br = board.getBoundingClientRect(); tip.textContent = b.dataset.tip!; tip.style.left = (rr.left - br.left + rr.width / 2) + 'px'; tip.style.translate = '-50% 0'; tip.classList.add('on'); }; b.onmouseleave = () => tip.classList.remove('on'); b.addEventListener('click', () => { tip.classList.remove('on'); menu = false; setTimeout(render, 30); }); });
  }
  return {
    render, openMenu() { menu = true; render(); },
    light(kind: string) {
      const r = sh.getBoundingClientRect(), b = board.getBoundingClientRect(), c = (COLORS as Record<string, string>)[kind] || '#fff';
      beam(board, c, r.left - b.left + r.width / 2, ctx.cfg().island.beam);
      setTimeout(() => { ignite(sh, c); if (kind === 'permission') beat(sh); if (kind === 'error') shake(sh); }, T(540));
    },
    destroy() { ctx.hitbox(null); sh.remove(); tip.remove(); },
  };
}
