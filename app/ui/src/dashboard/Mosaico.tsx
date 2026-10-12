// Mosaico (spec §8, artboard Mosaico): barra lateral de 236 px y una superficie con Inicio, Sesiones, Hoy, Bitácora y Ajustes.
import { useEffect, useState } from 'react';
import { onRoute, openPalette } from '../shared/bridge';
import { useConfig, useDaySummary, useNow, useSnapshot, useTheme } from '../shared/react';
import { Icon } from '../shared/ui';
import { formatRoute, parseRoute, type Route, type Screen } from './route';
import { Hoy } from './screens/Hoy';
import { Inicio } from './screens/Inicio';
import { Sesiones } from './screens/Sesiones';

declare global { interface Window { __pulseRoute?: string } }

/** Ruta actual: la que dejó el núcleo al crear la ventana, la del hash o la que llega por `pulse://route`. */
function useRoute(): [Route, (r: Route) => void] {
  const [route, set] = useState<Route>(() => parseRoute(window.__pulseRoute || location.hash));
  const go = (r: Route) => { set(r); history.replaceState(null, '', '#' + formatRoute(r)); };
  useEffect(() => { let off: (() => void) | undefined; onRoute(v => go(parseRoute(v))).then(f => (off = f)); return () => off?.(); }, []);
  return [route, go];
}

const NAV: [Screen, string, string][] = [['inicio', 'home', 'Inicio'], ['sesiones', 'stacks', 'Sesiones'], ['hoy', 'schedule', 'Hoy']];

export function Mosaico() {
  const snap = useSnapshot(), [cfg] = useConfig(), now = useNow(), day = useDaySummary(snap);
  useTheme(cfg);
  const [route, go] = useRoute();
  useEffect(() => {
    const k = (e: KeyboardEvent) => { if (e.ctrlKey && e.key.toLowerCase() === 'k') { e.preventDefault(); openPalette(); } };
    window.addEventListener('keydown', k); return () => window.removeEventListener('keydown', k);
  }, []);
  if (!cfg) return null;
  const screen = route.screen === 'ajustes' || route.screen === 'bitacora' ? 'inicio' : route.screen;   // llegan en las Tasks 5.2 y 5.3
  const link = (s: Screen, ic: string, l: string, count?: number) => <a key={s} href={`#${s}`} aria-current={screen === s ? 'page' : undefined} onClick={e => { e.preventDefault(); go({ screen: s }); }}><Icon name={ic} />{l}<small>{count || ''}</small></a>;
  return <div className="mz">
    <nav className="mz-nav" aria-label="Secciones">
      <div className="mz-brand"><span className="mz-dots" aria-hidden="true"><i className="dot halo" /><i className="dot fill" /><i className="dot calm" /></span>Pulse<button type="button" className="kbd mz-k" aria-label="Abrir la paleta (Ctrl K)" onClick={() => openPalette()}>Ctrl K</button></div>
      {NAV.map(([s, ic, l]) => link(s, ic, l, s === 'sesiones' ? snap.sessions.length : undefined))}
      <div className="mz-fill" />
      {link('ajustes', 'settings', 'Ajustes')}
    </nav>
    <main className="mz-main surf">
      {screen === 'inicio' && <Inicio snap={snap} cfg={cfg} day={day} now={now} go={go} />}
      {screen === 'sesiones' && <Sesiones snap={snap} cfg={cfg} now={now} arg={route.arg} />}
      {screen === 'hoy' && <Hoy day={day} now={now} />}
    </main>
  </div>;
}
