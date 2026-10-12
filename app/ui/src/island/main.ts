import { listen } from '@tauri-apps/api/event';
import { allowAll, answer, decide, holdQuestion, skip, getConfig, getSnapshot, islandFocus, muteSession, onConfig, onFx, onState, openDashboard, setConfig, setFocus, setIslandHitbox, type Config, type Fx, type Snapshot } from '../shared/bridge';
import { SP_MOVE, SP_OPEN, T } from '../shared/motion';
import { mountComando } from './comando';
import { mountOrbe, type OrbeCtx } from './orbe';
import { bindQuestions } from './questions';
import { toNotice, type Notice } from './model';
import { tickSecs } from './shared';

const board = document.getElementById('board')!;
document.documentElement.style.setProperty('--sp', SP_MOVE.e);
let cfg: Config, snap: Snapshot;
let notice: Notice | null = null, noticeT = 0;
type Mode = { render(): void; light(kind: string): void; destroy(): void; open?: () => void; openMenu?: () => void };
let mode: Mode | null = null, modeName = '';

function hitbox(el: HTMLElement | null) {
  const send = () => { if (!el || !el.isConnected) return setIslandHitbox(null); const r = el.getBoundingClientRect(); setIslandHitbox({ x: r.left, y: r.top, w: r.width, h: r.height }); };
  requestAnimationFrame(send);
  if (el) setTimeout(send, T(SP_OPEN.d) + 40);   // otra vez cuando el resorte termina
}

const ctx: OrbeCtx = {
  snap: () => snap, cfg: () => cfg, notice: () => notice,
  decide: (id, allow) => { decide(id, allow); }, answer: (id, a) => { answer(id, a); }, skip: id => { skip(id); }, hold: (id, on) => { holdQuestion(id, on); }, allowAll: () => { allowAll(); }, setFocus: on => { setFocus(on); },
  muteSession: (id, minutes) => { muteSession(id, minutes); }, openDashboard: () => { openDashboard(); },
  islandFocus: on => { islandFocus(on); }, hitbox,
  setSide: side => { cfg = { ...cfg, island: { ...cfg.island, orb_side: side } }; setConfig(cfg); },
};

function mount() {
  mode?.destroy();
  modeName = cfg.island.mode;
  mode = modeName === 'orbe' ? mountOrbe(board, ctx) : mountComando(board, ctx);
  mode.render();
}
// Si la isla tiene el teclado pero ya no hay nada donde escribir (la pregunta se respondió en otro lado, se acabó el
// tiempo, se cerró la paleta), se le devuelve a la ventana donde estabas.
const render = () => { mode?.render(); if (document.hasFocus() && !board.querySelector('.qcard, .pal')) islandFocus(false); };

function show(n: Notice) { notice = n; clearTimeout(noticeT); noticeT = window.setTimeout(() => { notice = null; render(); }, T(n.life)); render(); }

function onEffect(fx: Fx) {
  const later = (f: () => void) => setTimeout(f, T(cfg.island.beam ? 520 : 0));
  const n = toNotice(fx);
  if (fx.kind === 'resolved' || fx.kind === 'notice') { if (n) show(n); return; }
  if (fx.kind === 'release') { mode?.light('focus'); if (n) later(() => show(n)); return; }
  if (!fx.visual) return;
  mode?.light(fx.kind);
  if (n) later(() => show(n));
}

/* mantener presionado para permitir comandos delicados (clic en la tarjeta del Orbe) */
let holdRaf = 0, holdT0 = 0, holdId = 0;
const setHold = (p: number) => board.querySelectorAll<HTMLElement>('.btn[data-hold]').forEach(b => b.style.setProperty('--hold', String(p)));
function holdBegin(id: number) { if (holdRaf) return; holdId = id; holdT0 = performance.now(); const step = (t: number) => { const p = Math.min(1, (t - holdT0) / 900); setHold(p); if (p >= 1) { holdRaf = 0; setHold(0); decide(holdId, true); return; } holdRaf = requestAnimationFrame(step); }; holdRaf = requestAnimationFrame(step); }
function holdEnd() { if (!holdRaf) return; cancelAnimationFrame(holdRaf); holdRaf = 0; setHold(0); }
document.addEventListener('pointerdown', e => { const b = (e.target as HTMLElement).closest<HTMLElement>('.btn[data-danger="1"]'); if (b) { e.preventDefault(); holdBegin(Number(b.dataset.id)); } });
['pointerup', 'pointercancel'].forEach(t => document.addEventListener(t, holdEnd));

(async () => {
  [cfg, snap] = await Promise.all([getConfig(), getSnapshot()]);
  mount();
  bindQuestions(board, id => snap.pending.find(p => p.id === id), ctx);
  onState(s => { snap = s; render(); });
  onConfig(c => { cfg = c; if (c.island.mode !== modeName) mount(); else render(); });
  onFx(onEffect);
  listen('pulse://palette', () => { if (modeName === 'comando') mode?.open?.(); else mode?.openMenu?.(); });
  listen<{ progress: number }>('pulse://hold', e => setHold(e.payload.progress));
  setInterval(() => { render(); tickSecs(board, snap.pending); }, 1000);
})();
