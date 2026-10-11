import { getConfig, getSnapshot, onConfig, onFx, onState, type Config, type Snapshot } from '../shared/bridge';
import { pulse, residualOn, residualOff, residualKeys, slotX } from '../shared/pulse-engine';
import { reduceMotion } from '../shared/motion';

const me = Number(new URLSearchParams(location.search).get('m') ?? 0);
const layer = document.getElementById('fx')!;
let cfg: Config;

// Detección aproximada de fondo claro: se toma de la preferencia del sistema.
const light = () => matchMedia('(prefers-color-scheme: light)').matches;
const opts = () => ({ crisp: cfg.pulses.crisp, corners: cfg.pulses.corners, adapt: cfg.pulses.adapt, dither: cfg.pulses.dither, lightBackground: light(), reduce: reduceMotion() });
const mine = (monitor: number) => cfg.pulses.placement === 'all' || monitor === me;

function syncResiduals(s: Snapshot) {
  const want = new Set<string>();
  for (const p of s.pending) want.add(`${p.session_id}:permission`);
  for (const x of s.sessions) if (x.state === 'IdleWait') want.add(`${x.id}:idle`);
  for (const k of residualKeys()) if (!want.has(k)) residualOff(k);
  for (const k of want) {
    const kind = k.split(':')[1]; const pc = cfg.pulses.events[kind];
    if (pc?.persistent && cfg.pulses.enabled && !residualKeys().includes(k) && !s.focus) residualOn(k, layer, pc, reduceMotion());
  }
}

(async () => {
  cfg = await getConfig();
  onConfig(c => (cfg = c));
  onFx(fx => {
    if (!fx.visual || !mine(fx.monitor)) return;
    const pc = cfg.pulses.events[fx.kind === 'release' ? 'idle' : fx.kind]; if (!pc) return;
    const x = slotX(fx.slot);
    pulse(layer, fx.kind === 'release' ? { ...pc, shape: 'ola', intensity: 70, duration_ms: 2200 } : pc, { x, y: 140, side: x < 50 ? 'left' : 'right' }, opts());
  });
  onState(s => { if (cfg) syncResiduals(s); });
  syncResiduals(await getSnapshot());
})();
