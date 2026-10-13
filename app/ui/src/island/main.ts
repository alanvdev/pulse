import { listen } from '@tauri-apps/api/event';
import { allowAll, answer, dailySummary, decide, holdQuestion, skip, getConfig, getSnapshot, forgetSessions, islandDrag, islandFocus, muteSession, onConfig, onFx, onState, openDashboard, setConfig, setFocus, setIslandHitbox, type Config, type DaySummary, type Fx, type Snapshot } from '../shared/bridge';
import { SP_MOVE, SP_OPEN, T } from '../shared/motion';
import { mountComando } from './comando';
import { mountOrbe, type OrbeCtx } from './orbe';
import { bindQuestions } from './questions';
import { growRect, toNotice, type Notice } from './model';
import { focusEdge } from '../shared/micro';
import { COLORS } from '../shared/format';
import { shake, tickSecs } from './shared';

const board = document.getElementById('board')!;
document.documentElement.style.setProperty('--sp', SP_MOVE.e);
let cfg: Config, snap: Snapshot;
let notice: Notice | null = null, noticeT = 0;
let day: DaySummary | null = null, dayAt = 0;
type Mode = { render(): void; light(kind: string): void; destroy(): void; open?: () => void; openMenu?: () => void };
let mode: Mode | null = null, modeName = '';
/** La isla pidió el teclado (paleta, "Otra…", menú del Orbe). Solo entonces lo devuelve sola; un clic del usuario no cuenta. */
let kbOwned = false;

/** Zona de clics: lo que ocupa `el` ahora unido a lo que ocupará al terminar el resorte. morph() ya dejó el tamaño final
 *  en style.width/height; se ancla como el CSS (arriba fijo; Orbe a la derecha o a la izquierda por su borde, el resto
 *  centrado). Sin eso, durante los ~600 ms del resorte la zona medía lo de antes y el resumen recién abierto se cerraba
 *  al bajar el mouse, o el clic en Permitir pasaba a la ventana de abajo. El tablero (arrastre del Orbe) no tiene
 *  tamaño en style: se manda su rectángulo tal cual. */
const hitRect = (el: HTMLElement) => growRect(el.getBoundingClientRect(), parseFloat(el.style.width), parseFloat(el.style.height), el.classList.contains('right') ? 'right' : el.classList.contains('left') ? 'left' : 'center');
// Una sola medición pendiente: la última llamada manda (un envío viejo no reabre la zona tras un hitbox(null)).
let hbRaf = 0, hbT = 0, hbEl: HTMLElement | null = null;
function hitbox(el: HTMLElement | null) {
  hbEl = el;
  const send = () => { if (!el || !el.isConnected) return setIslandHitbox(null); setIslandHitbox(hitRect(el)); };
  cancelAnimationFrame(hbRaf); clearTimeout(hbT);
  hbRaf = requestAnimationFrame(send);
  if (el) hbT = window.setTimeout(send, T(SP_OPEN.d) + 40);   // otra vez cuando el resorte termina: ajusta al tamaño final
}

// Linux: la ventana cambia de ancho si cambia el área libre del monitor (dock, otro monitor); el contenido se recoloca y la
// zona de clics tiene que seguirlo.
window.addEventListener('resize', () => { if (hbEl) hitbox(hbEl); });
// la cápsula se ajustó a contenido que creció después de aparecer (ver morph): la zona de clics también
board.addEventListener('fit', () => { if (hbEl) hitbox(hbEl); });

const ctx: OrbeCtx = {
  snap: () => snap, cfg: () => cfg, notice: () => notice,
  day: () => day,
  refreshDay: () => { if (Date.now() - dayAt < 30_000) return; dayAt = Date.now(); dailySummary().then(d => { day = d; render(); }).catch(() => {}); },
  decide: (id, allow) => { decide(id, allow); }, answer: (id, a) => { answer(id, a); }, skip: id => { skip(id); }, hold: (id, on) => { holdQuestion(id, on); }, allowAll: () => { allowAll(); }, setFocus: on => { setFocus(on); },
  muteSession: (id, minutes) => { muteSession(id, minutes); }, forgetSessions: ids => { forgetSessions(ids); }, openDashboard: view => { openDashboard(view); },
  islandFocus: on => { kbOwned = on; islandFocus(on); }, hitbox,
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
const render = () => { mode?.render(); if (kbOwned && document.hasFocus() && !board.querySelector('.qcard, .pal, .o-menu')) { kbOwned = false; islandFocus(false); } };

function show(n: Notice) { notice = n; clearTimeout(noticeT); noticeT = window.setTimeout(() => { notice = null; render(); }, T(n.life)); render(); }

/** Ctrl+Alt+Y sobre un comando delicado en Linux (no se puede mantener): lo dice la propia tarjeta, que tapa los avisos.
 *  Sin tarjeta a la vista (recogida con Esc), va como aviso. */
function nudgeHold(n: Notice | null) {
  const btns = board.querySelectorAll<HTMLElement>('.layer:last-child .btn[data-danger="1"]');
  if (!btns.length) { if (n) show(n); return; }
  btns.forEach(b => { const l = b.closest('.card')?.querySelector<HTMLElement>('.bar-l'); if (l) l.textContent = 'Mantén presionado «Mantener»'; shake(b); });
}

function onEffect(fx: Fx) {
  const later = (f: () => void) => setTimeout(f, T(cfg.island.beam ? 520 : 0));
  const n = toNotice(fx);
  if (fx.kind === 'notice' && fx.text.startsWith('hold|')) { nudgeHold(n); return; }
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

/* Ctrl + arrastrar: mueve la isla (tarjeta, pastilla u Orbe) a cualquier lugar, en cualquier monitor. Ctrl + doble clic la regresa.
   Va en fase de captura para ganarle al arrastre del Orbe entre lados y a los botones de las tarjetas. */
let dragging = false, dragMoved = false, dragRaf = 0, swallowClick = false;
board.addEventListener('pointerdown', e => {
  if (!e.ctrlKey || e.button !== 0) return;
  e.preventDefault(); e.stopPropagation();
  dragging = true; dragMoved = false; board.setPointerCapture(e.pointerId); document.body.classList.add('moving');
  islandDrag('start');
}, true);
board.addEventListener('pointermove', e => {
  if (!dragging) return;
  e.stopPropagation(); dragMoved = true;
  if (!dragRaf) dragRaf = requestAnimationFrame(() => { dragRaf = 0; islandDrag('move'); });
}, true);
const dragEnd = (e: PointerEvent) => {
  if (!dragging) return;
  e.stopPropagation(); dragging = false; document.body.classList.remove('moving');
  // El clic llega en el mismo evento nativo que este pointerup; el timeout 0 lo deja pasar y luego limpia la marca.
  swallowClick = true; setTimeout(() => { swallowClick = false; }, 0);
  cancelAnimationFrame(dragRaf); dragRaf = 0;
  if (dragMoved) islandDrag('move').then(() => islandDrag('end')); else islandDrag('end');
};
board.addEventListener('pointerup', dragEnd, true);
board.addEventListener('pointercancel', dragEnd, true);
// El clic que termina un arrastre no es un clic sobre la tarjeta (no aprueba ni elige nada). Se mira el arrastre y no
// solo Ctrl: lo normal es soltar Ctrl un instante antes que el botón, y ese clic llega sin ctrlKey.
board.addEventListener('click', e => { if (swallowClick || e.ctrlKey) { swallowClick = false; e.preventDefault(); e.stopPropagation(); } }, true);
board.addEventListener('dblclick', e => { if (e.ctrlKey) { e.preventDefault(); e.stopPropagation(); islandDrag('reset'); } }, true);

(async () => {
  [cfg, snap] = await Promise.all([getConfig(), getSnapshot()]);
  mount();
  bindQuestions(board, id => snap.pending.find(p => p.id === id), ctx);
  onState(s => {
    const edge = focusEdge(snap.focus, s.focus), hadSaved = snap.deferred.length > 0;
    snap = s; render();
    // el foco se nota también aquí: haz de luz lavanda y una línea que dice qué pasa con los avisos
    if (edge === 'enter') { mode?.light('focus'); show({ id: Date.now(), icon: 'dark_mode', color: COLORS.focus, what: 'En foco · los avisos esperan', life: 2500 }); }
    if (edge === 'exit' && !hadSaved && !notice?.items) show({ id: Date.now(), icon: 'light_mode', color: COLORS.focus, what: 'Saliste del foco', life: 1600 });
  });
  onConfig(c => { cfg = c; if (c.island.mode !== modeName) mount(); else render(); });
  onFx(onEffect);
  listen('pulse://palette', () => { if (modeName === 'comando') mode?.open?.(); else mode?.openMenu?.(); });
  listen<{ progress: number }>('pulse://hold', e => setHold(e.payload.progress));
  setInterval(() => { render(); tickSecs(board, snap.pending); }, 1000);
})();
