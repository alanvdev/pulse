import { getConfig, getSnapshot, onConfig, onFx, onState, type Config, type Snapshot } from '../shared/bridge';
import { pulse, residualOn, residualOff, residualKeys, slotX } from '../shared/pulse-engine';
import { reduceMotion } from '../shared/motion';
import { wantedResiduals } from '../shared/residuals';
import { focusEdge } from '../shared/micro';
import { COLORS } from '../shared/format';

const me = Number(new URLSearchParams(location.search).get('m') ?? 0);
const layer = document.getElementById('fx')!;
let cfg: Config;

// Detección aproximada de fondo claro: se toma de la preferencia del sistema.
const light = () => matchMedia('(prefers-color-scheme: light)').matches;
const opts = () => ({ crisp: cfg.pulses.crisp, corners: cfg.pulses.corners, adapt: cfg.pulses.adapt, dither: cfg.pulses.dither, lightBackground: light(), reduce: reduceMotion() });
// sin monitor (Linux no puede saber cuál usas): en todos
const mine = (monitor: number | null) => cfg.pulses.placement === 'all' || monitor == null || monitor === me;

let lastSnap: Snapshot | null = null;
// Las mismas condiciones para encender y para apagar: en foco, con los pulsos apagados o sin "persistente", el brillo
// que ya estaba se apaga (antes solo se dejaba de encender y seguía respirando en los bordes).
function syncResiduals(s: Snapshot) {
  lastSnap = s;
  const pc = (k: string) => cfg.pulses.events[k.split(':')[1]];
  const active = cfg.pulses.enabled && !s.focus;
  const want = new Set([...wantedResiduals(s, Date.now())].filter(k => active && !!pc(k)?.persistent));
  for (const k of residualKeys()) if (!want.has(k)) residualOff(k);
  for (const k of want) if (!residualKeys().includes(k)) residualOn(k, layer, pc(k)!, reduceMotion());
}

(async () => {
  cfg = await getConfig();
  onConfig(c => { cfg = c; if (lastSnap) syncResiduals(lastSnap); });
  onFx(fx => {
    if (!fx.visual || !mine(fx.monitor)) return;
    const pc = cfg.pulses.events[fx.kind === 'release' ? 'idle' : fx.kind]; if (!pc) return;
    const x = slotX(fx.slot);
    pulse(layer, fx.kind === 'release' ? { ...pc, shape: 'ola', intensity: 70, duration_ms: 2200 } : pc, { x, y: 140, side: x < 50 ? 'left' : 'right' }, opts());
  });
  onState(s => {
    // al entrar al foco, un barrido lavanda suave, una sola vez (los pulsos que esperan se apagan en syncResiduals)
    if (cfg?.pulses.enabled && focusEdge(lastSnap?.focus, s.focus) === 'enter')
      pulse(layer, { color: COLORS.focus, shape: 'barrido', intensity: 45, thickness: 100, duration_ms: 1600, persistent: false }, { x: 50, y: 50, side: 'left' }, opts());
    if (cfg) syncResiduals(s);
  });
  setInterval(() => { if (cfg && lastSnap) syncResiduals(lastSnap); }, 2000);   // apaga el brillo de "te espera" al cumplir su minuto
  syncResiduals(await getSnapshot());
})();
