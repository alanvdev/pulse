// Hooks de React sobre el puente con el núcleo (solo Bolsillo y Mosaico).
import { useCallback, useEffect, useRef, useState } from 'react';
import { dailySummary, getConfig, getSnapshot, onConfig, onState, setConfig, type Config, type DaySummary, type Snapshot } from './bridge';
import { followTheme } from './theme';

const EMPTY: Snapshot = { sessions: [], pending: [], feed: [], focus: false, focus_auto: false, deferred: [] };

export function useSnapshot(): Snapshot {
  const [s, set] = useState<Snapshot>(EMPTY);
  useEffect(() => { let off: (() => void) | undefined; getSnapshot().then(set); onState(set).then(f => (off = f)); return () => off?.(); }, []);
  return s;
}

/** Configuración actual y una función para guardarla (escribe config.json y avisa a todas las ventanas). */
export function useConfig(): [Config | null, (c: Config) => void] {
  const [c, set] = useState<Config | null>(null);
  useEffect(() => { let off: (() => void) | undefined; getConfig().then(set); onConfig(set).then(f => (off = f)); return () => off?.(); }, []);
  const save = useCallback((next: Config) => { set(next); setConfig(next); }, []);
  return [c, save];
}

/** Re-render periódico para los tiempos relativos. */
export function useNow(ms = 1000): number {
  const [now, set] = useState(Date.now());
  useEffect(() => { const t = setInterval(() => set(Date.now()), ms); return () => clearInterval(t); }, [ms]);
  return now;
}

/** Resumen del día desde la bitácora. Se refresca cada `every` ms y con cada cambio de estado, con un mínimo de 10 s entre llamadas. */
export function useDaySummary(snap: Snapshot, every = 60_000): DaySummary | null {
  const [d, set] = useState<DaySummary | null>(null);
  const [last, setLast] = useState(0);
  const load = useCallback(() => { setLast(Date.now()); dailySummary().then(x => set(x ?? null)).catch(() => {}); }, []);
  useEffect(() => { load(); const t = setInterval(load, every); return () => clearInterval(t); }, [load, every]);
  useEffect(() => { if (Date.now() - last > 10_000) load(); }, [snap]); // eslint-disable-line react-hooks/exhaustive-deps
  return d;
}

/** Mantener 900 ms para aprobar comandos delicados (misma lógica holdBegin/holdEnd de isla.html). */
export function useHold(onDone: () => void) {
  const raf = useRef(0), t0 = useRef(0), [p, setP] = useState(0);
  const begin = () => { if (raf.current) return; t0.current = performance.now(); const step = (t: number) => { const x = Math.min(1, (t - t0.current) / 900); setP(x); if (x >= 1) { raf.current = 0; setP(0); onDone(); return; } raf.current = requestAnimationFrame(step); }; raf.current = requestAnimationFrame(step); };
  const end = () => { if (!raf.current) return; cancelAnimationFrame(raf.current); raf.current = 0; setP(0); };
  return { p, begin, end };
}

/** Aplica `appearance.theme` a esta ventana y lo sigue mientras cambie. */
export function useTheme(cfg: Config | null) {
  const pref = cfg?.appearance?.theme;
  useEffect(() => followTheme(pref), [pref]);
}
