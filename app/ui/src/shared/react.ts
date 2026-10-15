// Hooks de React sobre el puente con el núcleo (solo Bolsillo y Mosaico).
import { useCallback, useEffect, useLayoutEffect, useRef, useState, type RefObject } from 'react';
import { dailySummary, getConfig, logToday, getSnapshot, onConfig, onFx, onState, setConfig, type Config, type DaySummary, type LogItem, type Snapshot } from './bridge';
import { countAt, flashColor, focusEdge } from './micro';
import { reduceMotion } from './motion';
import { followTheme } from './theme';
import type { SessionState } from './bridge';

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
  useEffect(() => () => cancelAnimationFrame(raf.current), []);   // al desmontar no debe aprobar nada
  // la ventana pierde el foco con la tecla aún abajo: el keyup llega a otra ventana y la aprobación seguiría sola
  useEffect(() => { const off = () => end(); window.addEventListener('blur', off); return () => window.removeEventListener('blur', off); });
  return { p, begin, end };
}

/** Aplica `appearance.theme` a esta ventana y lo sigue mientras cambie. */
export function useTheme(cfg: Config | null) {
  const pref = cfg?.appearance?.theme, ready = cfg !== null;
  // sin config todavía se queda el tema recordado (applyRemembered en main.tsx)
  useEffect(() => ready ? followTheme(pref) : undefined, [pref, ready]);
}

/** Eventos de hoy para la Bitácora. Se refresca cada `every` ms y con cada cambio de estado, con un mínimo de 5 s entre llamadas. */
export function useLogToday(snap: Snapshot, every = 15_000): LogItem[] | null {
  const [d, set] = useState<LogItem[] | null>(null);
  const last = useRef(0);
  const load = useCallback(() => { last.current = Date.now(); logToday().then(set).catch(() => {}); }, []);
  useEffect(() => { load(); const t = setInterval(load, every); return () => clearInterval(t); }, [load, every]);
  useEffect(() => { if (Date.now() - last.current > 5_000) load(); }, [snap]); // eslint-disable-line react-hooks/exhaustive-deps
  return d;
}

/* ---------- microinteracciones ---------- */

/** Cada vez que el modo foco entra o sale (no al abrir la ventana): `n` cambia para volver a disparar la animación. */
export function useFocusEdge(snap: Snapshot): { edge: 'enter' | 'exit'; n: number } | null {
  const prev = useRef<boolean | undefined>(undefined), [ev, setEv] = useState<{ edge: 'enter' | 'exit'; n: number } | null>(null);
  useEffect(() => {
    if (snap === EMPTY) return;   // todavía no llega el estado real
    const e = focusEdge(prev.current, snap.focus); prev.current = snap.focus;
    if (e) setEv(x => ({ edge: e, n: (x?.n ?? 0) + 1 }));
  }, [snap]);
  return ev;
}

/** Destello suave de una fila cuando su sesión termina o pide permiso (clase .flash de ui.css). */
export function useFlash<T extends HTMLElement>(state: SessionState) {
  const ref = useRef<T>(null), prev = useRef<SessionState | undefined>(undefined);
  useEffect(() => {
    const c = flashColor(prev.current, state), el = ref.current; prev.current = state;
    if (!c || !el) return;
    el.style.setProperty('--flash', c); el.classList.remove('flash'); void el.offsetWidth; el.classList.add('flash');
  }, [state]);
  return ref;
}

/** Resaltado que se desliza al elemento activo (`selector`) dentro de un contenedor `.hl` (ui.css). */
export function useHighlight(ref: RefObject<HTMLElement>, selector: string, deps: unknown[]) {
  useLayoutEffect(() => {
    const box = ref.current; if (!box) return;
    const place = () => {
      const el = box.querySelector<HTMLElement>(selector);
      if (!el) { box.removeAttribute('data-hl'); return; }
      box.style.setProperty('--hl-x', `${el.offsetLeft}px`); box.style.setProperty('--hl-y', `${el.offsetTop}px`);
      box.style.setProperty('--hl-w', `${el.offsetWidth}px`); box.style.setProperty('--hl-h', `${el.offsetHeight}px`);
      requestAnimationFrame(() => box.setAttribute('data-hl', ''));   // la primera vez aparece en su lugar, sin viajar
    };
    place();
    window.addEventListener('resize', place);
    return () => window.removeEventListener('resize', place);
  }, deps); // eslint-disable-line react-hooks/exhaustive-deps
}

/** Cifra que cuenta desde 0 la primera vez que aparece; después sigue al valor sin animar. */
export function useCountUp(to: number, ms = 700): number {
  const done = useRef(false), [v, set] = useState(() => reduceMotion() ? to : 0);
  useEffect(() => {
    if (done.current || reduceMotion()) { set(to); return; }
    if (!to) return;   // espera a que llegue el dato real
    done.current = true;
    const t0 = performance.now(); let raf = 0;
    const step = (t: number) => { const k = (t - t0) / ms; set(countAt(k, to)); if (k < 1) raf = requestAnimationFrame(step); };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }, [to, ms]);
  return v;
}

/** Halo lavanda al entrar o salir del foco (clases .f-enter / .f-exit de ui.css sobre `ref`). */
export function useFocusHalo(ref: RefObject<HTMLElement>, ev: { edge: 'enter' | 'exit'; n: number } | null) {
  useEffect(() => {
    const el = ref.current; if (!el || !ev) return;
    el.classList.remove('f-enter', 'f-exit'); void el.offsetWidth; el.classList.add(`f-${ev.edge}`);
  }, [ev?.n]); // eslint-disable-line react-hooks/exhaustive-deps
}

export type Resolved = { n: number; label: string; text: string; project: string };
/** El último pendiente resuelto (desde cualquier lado: isla, atajo, aquí), visible un momento para plegarse con ✓ o ✗. */
export function useResolved(): Resolved | null {
  const [r, set] = useState<Resolved | null>(null);
  useEffect(() => {
    let off: (() => void) | undefined, t = 0;
    onFx(fx => {
      if (fx.kind !== 'resolved') return;
      const [label, ...rest] = fx.text.split('|');
      set({ n: Date.now(), label, text: rest.join('|'), project: fx.project });
      clearTimeout(t); t = window.setTimeout(() => set(null), reduceMotion() ? 500 : 950);
    }).then(f => (off = f));
    return () => { off?.(); clearTimeout(t); };
  }, []);
  return r;
}
