// Bolsillo: panel de la bandeja (spec §7, artboard Bolsillo). Pestañas Ahora, Hoy y Ajustes rápidos.
import { useEffect, useRef, useState, type KeyboardEvent as RKeyboardEvent } from 'react';
import { decide, openDashboard, setFocus, skip, togglePanel, type Config, type DaySummary, type Pending, type Snapshot } from '../shared/bridge';
import { clock, headline, hm, sessionsLabel } from '../shared/format';
import { reduceMotion } from '../shared/motion';
import { useConfig, useDaySummary, useFocusEdge, useFocusHalo, useHold, useNow, useSnapshot, useTheme } from '../shared/react';
import { Button, Chip, Dot, Icon, Keys, Seg, Slider, StateIcon, Switch } from '../shared/ui';
import { Timeline, TimelineLegend } from '../shared/timeline';
import { focusLine, sessionRows, verb } from '../island/model';

type Tab = 'ahora' | 'hoy' | 'ajustes';
const TABS: [Tab, string][] = [['ahora', 'Ahora'], ['hoy', 'Hoy'], ['ajustes', 'Ajustes']];
const secsLeft = (p: Pending, now: number) => `${Math.max(0, Math.ceil((p.paused ? p.left_ms : p.deadline_ms - now) / 1000))}s`;

type Hold = ReturnType<typeof useHold>;
function PendingBlock({ p, cfg, now, hold }: { p: Pending; cfg: Config; now: number; hold: Hold }) {
  const danger = p.danger && cfg.island.hold;
  // Mantener también con el teclado: Enter o Espacio sostenidos sobre el botón
  const keyHold = (down: boolean) => (e: RKeyboardEvent) => { if (!danger || (e.key !== 'Enter' && e.key !== ' ')) return; e.preventDefault(); if (down && !e.repeat) hold.begin(); if (!down) hold.end(); };
  if (p.kind === 'question') return <div className="pb">
    <div className="pb-h"><Dot kind="halo" /><b>{p.project}</b>te pregunta</div>
    <div className="pb-q">{p.command}</div>
    <div className="pb-n">Respóndela en la isla, arriba de tu pantalla.</div>
    <div className="pb-a"><Button solo size="md" onClick={() => skip(p.id)}>Responder en la terminal</Button></div>
  </div>;
  return <div className="pb">
    <div className="pb-h"><Dot kind="halo" /><b>{p.project}</b>{verb(p.tool, true)}{cfg.island.countdown && <span className="pb-s mono">{secsLeft(p, now)}</span>}</div>
    <div className="pb-c mono">{p.command}</div>
    {p.description && <div className="pb-n">{p.description}</div>}
    <div className="pb-a">
      <Button solo size="md" keys={['N']} onClick={() => decide(p.id, false)}>Denegar</Button>
      <Button pri size="md" keys={['Y']} hold={hold.p} onClick={() => { if (!danger) decide(p.id, true); }}
        onPointerDown={e => { if (danger) { e.preventDefault(); hold.begin(); } }} onPointerUp={hold.end} onPointerLeave={hold.end} onPointerCancel={hold.end}
        onKeyDown={keyHold(true)} onKeyUp={keyHold(false)}>{danger ? 'Mantener' : 'Permitir'}</Button>
    </div>
  </div>;
}

function Ahora({ snap, cfg, save, now, hold }: { snap: Snapshot; cfg: Config; save: (c: Config) => void; now: number; hold: Hold }) {
  const top = snap.pending[0], rows = sessionRows(snap, now).filter(r => !top || r.pendingId !== top.id);
  const flip = <K extends 'sounds' | 'pulses' | 'keyboard'>(k: K) => save({ ...cfg, [k]: { ...cfg[k], enabled: !cfg[k].enabled } });
  return <>
    <header><div className="bp-n">{sessionsLabel(snap.sessions.length)}{snap.focus && <> · <span className="fz"><Icon name="dark_mode" />{focusLine(snap)}</span></>}</div><div className="bp-h">{headline(snap)}</div></header>
    {top && <PendingBlock key={top.id} p={top} cfg={cfg} now={now} hold={hold} />}
    {snap.sessions.length ? <div>{rows.map(r => <div key={r.id} className="s">
      <StateIcon s={r.pendingId ? 'Waiting' : r.state} />
      <span className="min-w-0"><span className="s-p">{r.project}</span><span className={`s-w ${r.mono ? 'mono' : ''}`}>{r.what}</span></span>
      <span className="s-t mono">{r.state === 'Done' ? '' : clock(now - r.since)}</span>
    </div>)}</div> : <p className="empty">No hay sesiones de Claude Code abiertas.</p>}
    <div className="chips">
      <Chip icon="dark_mode" tone="focus" on={snap.focus} onClick={() => setFocus(!snap.focus)}>Foco</Chip>
      <Chip icon="volume_up" on={cfg.sounds.enabled} onClick={() => flip('sounds')}>Sonidos</Chip>
      <Chip icon="blur_on" on={cfg.pulses.enabled} onClick={() => flip('pulses')}>Pulsos</Chip>
      <Chip icon="keyboard" on={cfg.keyboard.enabled} onClick={() => flip('keyboard')}>Teclado</Chip>
    </div>
  </>;
}

function Hoy({ day, now }: { day: DaySummary | null; now: number }) {
  return <>
    <header><div className="bp-n">Hoy</div><div className="bp-h">{day ? hm(day.total_work_ms) : '—'}</div><div className="bp-n">de trabajo de Claude · te esperó {day ? hm(day.total_waited_ms) : '—'}</div></header>
    <Timeline day={day} now={now} label={96} minGap={0.2} max={5} />
    <TimelineLegend />
  </>;
}

function Rapidos({ cfg, save }: { cfg: Config; save: (c: Config) => void }) {
  const set = <K extends keyof Config>(k: K, v: Partial<Config[K]>) => save({ ...cfg, [k]: { ...cfg[k], ...v } });
  return <>
    <header><div className="bp-n">Ajustes rápidos</div></header>
    <div>
      <div className="qs"><span>Modo de la isla</span><Seg label="Modo de la isla" value={cfg.island.mode} options={[['comando', 'Comando'], ['orbe', 'Orbe']]} onChange={mode => set('island', { mode })} /></div>
      <div className="qs"><span>Pulsos en los bordes</span><Switch label="Pulsos en los bordes" on={cfg.pulses.enabled} onChange={enabled => set('pulses', { enabled })} /></div>
      <div className="qs"><span>Sonidos</span><Switch label="Sonidos" on={cfg.sounds.enabled} onChange={enabled => set('sounds', { enabled })} /></div>
      <div className="qs"><span>Volumen</span><Slider label="Volumen" value={cfg.sounds.volume} min={0} max={100} step={5} onChange={volume => set('sounds', { volume })} /></div>
      <div className="qs"><span>Modo foco automático<small>En llamadas, pantalla completa y tu horario</small></span><Switch label="Modo foco automático" on={cfg.focus.auto} onChange={auto => set('focus', { auto })} /></div>
    </div>
    <Button solo size="md" onClick={() => openDashboard('ajustes')}>Todos los ajustes</Button>
  </>;
}

export function Bolsillo() {
  const snap = useSnapshot(), [cfg, save] = useConfig(), now = useNow(), day = useDaySummary(snap);
  useTheme(cfg);
  const [tab, setTab] = useState<Tab>('ahora');
  const top = snap.pending[0], holdId = useRef(0);
  // un solo "mantener" para el bloque y la tecla Y; aprueba el pendiente con el que empezó, y se cancela si ese cambia
  const hold = useHold(() => decide(holdId.current, true));
  useEffect(() => { hold.end(); }, [top?.id]); // eslint-disable-line react-hooks/exhaustive-deps
  const beginHold = () => { if (top) { holdId.current = top.id; hold.begin(); } };
  const bp = useRef<HTMLElement>(null), view = useRef<HTMLDivElement>(null);
  useFocusHalo(bp, useFocusEdge(snap));

  // aparece desde la bandeja al mostrarse
  useEffect(() => {
    const show = () => { if (document.visibilityState !== 'visible' || !bp.current || reduceMotion()) return; bp.current.animate([{ opacity: 0, transform: 'translateY(10px) scale(.98)' }, { opacity: 1, transform: 'none' }], { duration: 420, easing: 'cubic-bezier(.32,.72,0,1)' }); };
    window.addEventListener('focus', show); document.addEventListener('visibilitychange', show);
    return () => { window.removeEventListener('focus', show); document.removeEventListener('visibilitychange', show); };
  }, []);

  // Y / N resuelven el primer permiso (solo en Ahora, donde se ve); en uno delicado, Y se mantiene. Ctrl+O abre Pulse; Esc cierra.
  useEffect(() => {
    const danger = (p: Pending) => p.danger && !!cfg?.island.hold;
    const onKey = (e: KeyboardEvent) => {
      const k = e.key.toLowerCase();
      if (e.ctrlKey && k === 'o') { e.preventDefault(); openDashboard(); return; }
      if (e.key === 'Escape') { togglePanel(); return; }
      if ((e.target as HTMLElement).matches('input[type="text"], textarea') || e.repeat || tab !== 'ahora') return;
      const p = snap.pending[0]; if (!p || p.kind !== 'permission' || e.ctrlKey || e.altKey) return;
      if (k === 'n') decide(p.id, false);
      if (k === 'y') { if (danger(p)) beginHold(); else decide(p.id, true); }
    };
    const onUp = (e: KeyboardEvent) => { if (e.key.toLowerCase() === 'y') hold.end(); };
    window.addEventListener('keydown', onKey); window.addEventListener('keyup', onUp);
    return () => { window.removeEventListener('keydown', onKey); window.removeEventListener('keyup', onUp); };
  }, [snap, cfg, tab]); // eslint-disable-line react-hooks/exhaustive-deps

  const pick = (t: Tab) => { setTab(t); if (view.current && !reduceMotion()) view.current.animate([{ opacity: 0, transform: 'translateY(4px)' }, { opacity: 1, transform: 'none' }], { duration: 240, easing: 'cubic-bezier(.16,1,.3,1)' }); };
  if (!cfg) return null;
  return <section ref={bp} aria-label="Bolsillo" className={`bp surf f-host ${snap.focus ? 'in-focus' : ''}`}>
    <div ref={view} className="bp-view" role="tabpanel">
      {tab === 'ahora' && <Ahora snap={snap} cfg={cfg} save={save} now={now} hold={{ ...hold, begin: beginHold }} />}
      {tab === 'hoy' && <Hoy day={day} now={now} />}
      {tab === 'ajustes' && <Rapidos cfg={cfg} save={save} />}
    </div>
    <div className="bp-foot">
      <div role="tablist" aria-label="Vista" className="flex gap-1">{TABS.map(([k, l]) => <button key={k} type="button" role="tab" className="tab" aria-selected={tab === k} onClick={() => pick(k)}>{l}</button>)}</div>
      <button type="button" className="btn bp-open" onClick={() => openDashboard()}><span>Abrir Pulse</span><Keys k={['Ctrl', 'O']} /></button>
    </div>
  </section>;
}
