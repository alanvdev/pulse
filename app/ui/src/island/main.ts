import { listen } from '@tauri-apps/api/event';
import { allowAll, answer, decide, skip, getConfig, getSnapshot, islandFocus, muteSession, onConfig, onFx, onState, openDashboard, setConfig, setFocus, setIslandHitbox, type Config, type Fx, type Snapshot } from '../shared/bridge';
import { SP_MOVE, SP_OPEN, T } from '../shared/motion';
import { mountComando } from './comando';
import { mountOrbe, type OrbeCtx } from './orbe';
import { bindQuestions } from './questions';
import type { Tr } from './shared';

const board = document.getElementById('board')!;
document.documentElement.style.setProperty('--sp', SP_MOVE.e);
let cfg: Config, snap: Snapshot;
let tr: Tr | null = null, trT = 0;
type Mode = { render(): void; light(kind: string): void; destroy(): void; open?: () => void; openMenu?: () => void };
let mode: Mode | null = null, modeName = '';

function hitbox(el: HTMLElement | null) {
  const send = () => { if (!el || !el.isConnected) return setIslandHitbox(null); const r = el.getBoundingClientRect(); setIslandHitbox({ x: r.left, y: r.top, w: r.width, h: r.height }); };
  requestAnimationFrame(send);
  if (el) setTimeout(send, T(SP_OPEN.d) + 40);   // otra vez cuando el resorte termina
}

const ctx: OrbeCtx = {
  snap: () => snap, cfg: () => cfg, transient: () => tr,
  decide: (id, allow) => { decide(id, allow); }, answer: (id, a) => { answer(id, a); }, skip: id => { skip(id); }, allowAll: () => { allowAll(); }, setFocus: on => { setFocus(on); },
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
const render = () => mode?.render();

function transient(t: Omit<Tr, 'id'>, ms: number) { tr = { ...t, id: Math.random() } as Tr; clearTimeout(trT); trT = window.setTimeout(() => { tr = null; render(); }, T(ms)); render(); }

function onEffect(fx: Fx) {
  const later = (f: () => void) => setTimeout(f, T(cfg.island.beam ? 520 : 0));
  if (fx.kind === 'resolved') {
    const [d, ...rest] = fx.text.split('|'), cmd = rest.join('|');
    if (d === 'answer') transient({ k: 'ok', text: `Respuesta enviada: ${cmd}` }, 1800);
    else if (d === 'skip') transient({ k: 'info', icon: 'terminal', text: 'Respóndela en la terminal' }, 1800);
    else if (d === 'terminal') transient({ k: 'info', icon: 'terminal', text: 'Respondido en la terminal' }, 1600);
    else if (d === 'none') transient({ k: 'info', icon: 'schedule', text: 'Sin respuesta: Claude te pregunta en la terminal' }, 2200);
    else transient({ k: d === 'allow' ? 'ok' : 'no', project: fx.project, text: cmd }, 1300);
    return;
  }
  if (fx.kind === 'notice') {
    const [what, ...rest] = fx.text.split('|');
    if (what === 'plan') transient({ k: 'info', icon: 'assignment', text: `${fx.project}: Claude terminó un plan, revísalo en la terminal` }, 4500);
    else transient({ k: 'info', icon: 'help', text: `${fx.project} te pregunta en la terminal: ${rest.join('|')}` }, 4500);
    return;
  }
  if (fx.kind === 'release') { mode?.light('focus'); later(() => transient({ k: 'release', items: JSON.parse(fx.text) }, 4200)); return; }
  if (!fx.visual) return;
  mode?.light(fx.kind);
  if (fx.kind === 'done') later(() => transient({ k: 'done', project: fx.project, text: 'terminó' }, 2800));
  if (fx.kind === 'error') later(() => transient({ k: 'error', project: fx.project, text: fx.text }, 2800));
}

/* mantener presionado para permitir comandos delicados (clic en la tarjeta del Orbe) */
let holdRaf = 0, holdT0 = 0, holdId = 0;
const setHold = (p: number) => board.querySelectorAll<HTMLElement>('.b-yes, .c-toast').forEach(b => b.style.setProperty('--hold', String(p)));
function holdBegin(id: number) { if (holdRaf) return; holdId = id; holdT0 = performance.now(); const step = (t: number) => { const p = Math.min(1, (t - holdT0) / 900); setHold(p); if (p >= 1) { holdRaf = 0; setHold(0); decide(holdId, true); return; } holdRaf = requestAnimationFrame(step); }; holdRaf = requestAnimationFrame(step); }
function holdEnd() { if (!holdRaf) return; cancelAnimationFrame(holdRaf); holdRaf = 0; setHold(0); }
document.addEventListener('pointerdown', e => { const b = (e.target as HTMLElement).closest<HTMLElement>('.b-yes[data-danger="1"]'); if (b) { e.preventDefault(); holdBegin(Number(b.dataset.id)); } });
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
  setInterval(render, 1000);
})();
