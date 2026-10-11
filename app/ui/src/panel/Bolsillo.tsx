// Bolsillo: panel de la bandeja. Portado de docs/artboards/centro-de-mando.html (ringSVG, bCard, bView, moveInd, ripple) a React + Tailwind.
import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type MouseEvent as RMouseEvent, type ReactNode } from 'react';
import { decide, openDashboard, setFocus, type Config, type DaySummary, type Pending, type Session, type Snapshot } from '../shared/bridge';
import { ICON, LBL, ago, stateColor } from '../shared/format';
import { reduceMotion } from '../shared/motion';
import { useConfig, useDaySummary, useNow, useSnapshot } from '../shared/react';

type Tab = 'ahora' | 'ajustes' | 'dia';
const TABS: [Tab, string, string][] = [['ahora', 'home', 'Inicio'], ['ajustes', 'tune', 'Ajustes'], ['dia', 'bar_chart', 'Hoy']];
const Ms = ({ children, className = '', style }: { children: ReactNode; className?: string; style?: CSSProperties }) => <span className={`ms ${className}`} style={style}>{children}</span>;
const cssVar = (v: Record<string, string | number>) => v as CSSProperties;

function Ring({ sessions }: { sessions: Session[] }) {
  const L = 2 * Math.PI * 25, n = Math.max(3, sessions.length), seg = L / n - 5;
  return <svg viewBox="0 0 58 58" className="w-[58px] h-[58px] -rotate-90"><circle className="bg" cx="29" cy="29" r="25" />
    {sessions.map((s, i) => <circle key={s.id} cx="29" cy="29" r="25" stroke={s.state === 'Idle' ? 'transparent' : stateColor(s.state)} strokeDasharray={`${s.state === 'Idle' ? 0 : seg} ${L}`} strokeDashoffset={-i * L / n - 2.5} />)}</svg>;
}

/** Mantener 900 ms para aprobar comandos delicados (misma lógica holdBegin/holdEnd de isla.html). */
function useHold(onDone: () => void) {
  const raf = useRef(0), t0 = useRef(0), [p, setP] = useState(0);
  const begin = () => { if (raf.current) return; t0.current = performance.now(); const step = (t: number) => { const x = Math.min(1, (t - t0.current) / 900); setP(x); if (x >= 1) { raf.current = 0; setP(0); onDone(); return; } raf.current = requestAnimationFrame(step); }; raf.current = requestAnimationFrame(step); };
  const end = () => { if (!raf.current) return; cancelAnimationFrame(raf.current); raf.current = 0; setP(0); };
  return { p, begin, end };
}

function flash(el: HTMLElement | null, ok: boolean, then: () => void) {
  const card = el?.closest<HTMLElement>('.card');
  if (card && !reduceMotion()) { card.animate([{ boxShadow: `inset 0 0 0 1px ${ok ? 'rgba(79,224,160,.7)' : 'rgba(255,90,82,.6)'}` }, { boxShadow: 'inset 0 0 0 1px transparent' }], { duration: 500 }); setTimeout(then, 180); } else then();
}

function PermActs({ p, cfg }: { p: Pending; cfg: Config }) {
  const danger = p.danger && cfg.island.hold;
  const yes = useRef<HTMLButtonElement>(null);
  const hold = useHold(() => decide(p.id, true));
  return <div className="acts flex gap-1.5">
    <button type="button" className="flex-1 inline-flex items-center justify-center gap-1.5 rounded-[9px] p-[7px] text-[12.5px] font-medium" onClick={e => flash(e.currentTarget, false, () => decide(p.id, false))}>Denegar <kbd>N</kbd></button>
    <button ref={yes} type="button" className="y flex-1 inline-flex items-center justify-center gap-1.5 rounded-[9px] p-[7px] text-[12.5px] font-medium" style={cssVar({ '--hold': hold.p })}
      onClick={e => { if (!danger) flash(e.currentTarget, true, () => decide(p.id, true)); }}
      onPointerDown={e => { if (danger) { e.preventDefault(); hold.begin(); } }} onPointerUp={hold.end} onPointerLeave={hold.end}>
      <span className="fill" /><span>{danger ? 'Mantener' : 'Permitir'}</span> <kbd>Y</kbd></button>
  </div>;
}

function Card({ s, p, cfg, fresh, now }: { s: Session; p?: Pending; cfg: Config; fresh: boolean; now: number }) {
  const c = stateColor(s.state);
  return <div className={`card rounded-xl px-3 py-[11px] mb-2 grid gap-[9px] ${p ? 'need' : ''} ${fresh ? 'enter' : ''}`}>
    <div className="grid grid-cols-[auto_minmax(0,1fr)_auto] gap-2.5 items-center">
      <span className="ic w-[30px] h-[30px] rounded-[9px] grid place-items-center" style={cssVar({ '--c': c })}><Ms className={s.state === 'Working' && !reduceMotion() ? 'spin' : ''} style={{ fontSize: 16 }}>{ICON[s.state]}</Ms></span>
      <div className="min-w-0"><div className="text-[13px] font-medium">{s.project}</div><div className="text-xs text-muted truncate">{p ? 'necesita tu permiso' : s.last || LBL[s.state]}</div></div>
      <span className="font-mono text-[11px] text-faint">{ago(now - s.since_ms)}</span>
    </div>
    {p && <div className="ex grid gap-[9px]"><div className="font-mono text-xs text-[#ffd590] bg-black/25 rounded-lg px-[9px] py-[7px] truncate">{p.command}</div><PermActs p={p} cfg={cfg} /></div>}
  </div>;
}

function ripple(btn: HTMLElement, e: RMouseEvent) {
  if (reduceMotion()) return;
  const r = btn.getBoundingClientRect(), d = Math.max(r.width, r.height) * 2.2, s = document.createElement('i'); s.className = 'rip';
  s.style.cssText = `width:${d}px;height:${d}px;left:${(e.clientX || r.left + r.width / 2) - r.left - d / 2}px;top:${(e.clientY || r.top + r.height / 2) - r.top - d / 2}px`;
  btn.appendChild(s); s.animate([{ transform: 'scale(0)', opacity: .6 }, { transform: 'scale(1)', opacity: 0 }], { duration: 550, easing: 'cubic-bezier(.16,1,.3,1)' }).onfinish = () => s.remove();
}

function Inicio({ snap, cfg, save, now }: { snap: Snapshot; cfg: Config; save: (c: Config) => void; now: number }) {
  const seen = useRef(new Set<string>());
  const working = snap.sessions.filter(s => s.state === 'Working').length;
  const need = snap.pending.length + snap.sessions.filter(s => s.state === 'IdleWait').length;
  const tiles: [string, string, boolean, () => void][] = [
    ['dark_mode', 'Foco', snap.focus, () => setFocus(!snap.focus)],
    ['notifications_off', 'Silencio', !cfg.sounds.enabled, () => save({ ...cfg, sounds: { ...cfg.sounds, enabled: !cfg.sounds.enabled } })],
    ['blur_on', 'Pulsos', cfg.pulses.enabled, () => save({ ...cfg, pulses: { ...cfg.pulses, enabled: !cfg.pulses.enabled } })],
    ['keyboard', 'Teclado', cfg.keyboard.enabled, () => save({ ...cfg, keyboard: { ...cfg.keyboard, enabled: !cfg.keyboard.enabled } })],
  ];
  const rank = (s: Session) => (snap.pending.some(p => p.session_id === s.id) || s.state === 'Waiting' ? 2 : s.state === 'IdleWait' ? 1 : 0);
  const sorted = [...snap.sessions].sort((a, b) => rank(b) - rank(a));
  const cards = sorted.map(s => { const key = s.id + s.state, fresh = !seen.current.has(key); return { s, fresh, key }; });
  useEffect(() => { cards.forEach(c => seen.current.add(c.key)); });
  return <>
    <div className="grid grid-cols-[auto_minmax(0,1fr)_auto] gap-3.5 items-center mb-4">
      <div className="bp-ring relative w-[58px] h-[58px]"><Ring sessions={snap.sessions} /><b className="absolute inset-0 grid place-items-center font-mono text-base font-medium">{working}</b></div>
      <div><h3 className="m-0 text-lg font-medium tracking-[-.01em]">{working} trabajando</h3><p className="mt-0.5 text-[12.5px] text-muted">{need ? `${need} te necesita${need > 1 ? 'n' : ''}` : 'Nada pendiente'}{snap.focus ? ' · en foco' : ''}</p></div>
      <button type="button" className="w-[30px] h-[30px] rounded-lg grid place-items-center bg-white/[.08] hover:bg-white/[.14] text-[#d9dde2] transition-colors" aria-label="Abrir tablero completo" title="Abrir tablero completo" onClick={() => openDashboard()}><Ms style={{ fontSize: 17 }}>open_in_new</Ms></button>
    </div>
    <div className="grid grid-cols-4 gap-2 mb-4">
      {tiles.map(([ic, l, on, run]) => <button key={l} type="button" aria-pressed={on} className="tile rounded-xl bg-white/[.07] pt-[11px] px-1 pb-[9px] grid justify-items-center gap-1.5 text-[11px] text-[#d8dce1]" onClick={e => { ripple(e.currentTarget, e); run(); }}><Ms>{ic}</Ms>{l}</button>)}
    </div>
    <div className="flex justify-between items-center text-xs text-faint mx-0.5 mt-1 mb-2"><span>Sesiones</span><span className="font-mono">{snap.sessions.length}</span></div>
    {cards.length ? cards.map(({ s, fresh }) => <Card key={s.id} s={s} p={snap.pending.find(p => p.session_id === s.id)} cfg={cfg} fresh={fresh} now={now} />)
      : <p className="text-[12.5px] text-faint mx-0.5">No hay sesiones de Claude Code abiertas.</p>}
  </>;
}

function Row({ icon, title, sub, children }: { icon: string; title: string; sub?: string; children: ReactNode }) {
  return <div className="srow flex items-center gap-3 py-[11px] px-1 text-[13px]"><Ms className="text-muted">{icon}</Ms><span className="flex-1 min-w-0">{title}{sub && <small className="block text-[11.5px] text-faint">{sub}</small>}</span>{children}</div>;
}
function Seg<T extends string>({ value, options, onChange }: { value: T; options: [T, string][]; onChange: (v: T) => void }) {
  return <span className="iseg flex bg-black/25 rounded-lg p-0.5 gap-0.5">{options.map(([v, l]) => <button key={v} type="button" aria-pressed={value === v} className="rounded-md px-[9px] py-1 text-xs text-muted" onClick={() => onChange(v)}>{l}</button>)}</span>;
}
const Sw = ({ on, label, onChange }: { on: boolean; label: string; onChange: (v: boolean) => void }) => <input type="checkbox" className="sw" checked={on} aria-label={label} onChange={e => onChange(e.target.checked)} />;

function Ajustes({ cfg, save }: { cfg: Config; save: (c: Config) => void }) {
  const set = <K extends keyof Config>(k: K, v: Partial<Config[K]>) => save({ ...cfg, [k]: { ...cfg[k], ...v } });
  return <>
    <div className="flex justify-between items-center text-xs text-faint mx-0.5 mt-1 mb-2"><span>Ajustes</span></div>
    <Row icon="toast" title="Modo de la isla" sub={cfg.island.mode === 'orbe' ? 'Un punto de luz que se expande' : 'Avisos breves y paleta con Ctrl+Alt+Espacio'}><Seg value={cfg.island.mode} options={[['comando', 'Comando'], ['orbe', 'Orbe']]} onChange={mode => set('island', { mode })} /></Row>
    <Row icon="blur_on" title="Pulsos en los bordes" sub="Latido ámbar al pedir permiso"><Sw on={cfg.pulses.enabled} label="Pulsos en los bordes" onChange={enabled => set('pulses', { enabled })} /></Row>
    <Row icon="desktop_windows" title="Dónde pulsar"><Seg value={cfg.pulses.placement} options={[['active', 'Monitor activo'], ['all', 'Todos']]} onChange={placement => set('pulses', { placement })} /></Row>
    <Row icon="graphic_eq" title="Sonidos" sub={`Tus 4 sonidos · volumen ${cfg.sounds.volume}`}><Sw on={cfg.sounds.enabled} label="Sonidos" onChange={enabled => set('sounds', { enabled })} /></Row>
    <div className="px-1 pb-2 -mt-1"><input type="range" min={0} max={100} step={5} value={cfg.sounds.volume} aria-label="Volumen" className="w-full" onChange={e => set('sounds', { volume: Number(e.target.value) })} /></div>
    <Row icon="keyboard" title="Teclado Razer" sub="F1 a F3 por sesión"><Sw on={cfg.keyboard.enabled} label="Teclado Razer" onChange={enabled => set('keyboard', { enabled })} /></Row>
    <Row icon="keyboard_command_key" title="Atajos de teclado" sub="Ctrl+Alt+Y para permitir, Ctrl+Alt+N para denegar"><Sw on={cfg.island.keys} label="Atajos de teclado" onChange={keys => set('island', { keys })} /></Row>
    <Row icon="back_hand" title="Mantener en comandos delicados" sub="git push --force, rm -rf, migrate…"><Sw on={cfg.island.hold} label="Mantener en comandos delicados" onChange={hold => set('island', { hold })} /></Row>
    <Row icon="dark_mode" title="Modo foco automático" sub="Llamadas y pantalla completa"><Sw on={cfg.focus.detect_call && cfg.focus.detect_fullscreen} label="Modo foco automático" onChange={on => set('focus', { detect_call: on, detect_fullscreen: on })} /></Row>
  </>;
}

const hm = (ms: number) => { const m = Math.round(ms / 60000); return m >= 60 ? `${Math.floor(m / 60)} h ${m % 60} min` : `${m} min`; };
function Hoy({ day }: { day: DaySummary | null }) {
  const frac = (lanes: [number, number, string][], k: string) => lanes.filter(l => l[2] === k).reduce((a, l) => a + (l[1] - l[0]), 0);
  return <>
    <div className="flex justify-between items-center text-xs text-faint mx-0.5 mt-1 mb-2"><span>Hoy</span><span className="font-mono">00:00 a ahora</span></div>
    <div className="card rounded-xl px-3 py-[11px] mb-2"><div className="font-mono text-xl">{day ? hm(day.total_work_ms) : '—'}</div><div className="text-xs text-muted">de trabajo de Claude · te esperó {day ? hm(day.total_waited_ms) : '—'}</div></div>
    {day?.sessions.map((s, i) => { const w = frac(s.lanes, 'w'), p = frac(s.lanes, 'p'), scale = Math.max(w + p, .0001) > 1 ? 1 / (w + p) : 1; return <div key={s.project + i} className="card rounded-xl px-3 py-[11px] mb-2 grid gap-[9px]">
      <div className="flex justify-between"><span className="text-[13px] font-medium">{s.project}</span><span className="font-mono text-[11px] text-faint">{s.actions} acciones</span></div>
      <div className="dbar h-[7px] rounded-[9px] bg-white/[.07] overflow-hidden flex"><i style={{ width: `${w * scale * 100}%`, background: '#e8eefc', animationDelay: `${i * 80}ms` }} /><i style={{ width: `${p * scale * 100}%`, background: '#ffb02e', animationDelay: `${i * 80 + 200}ms` }} /></div>
    </div>; })}
    {day && !day.sessions.length && <p className="text-[12.5px] text-faint mx-0.5">Todavía no hay actividad hoy.</p>}
    <p className="text-xs text-muted mx-0.5 my-1">Blanco: trabajando. Ámbar: esperando tu permiso.</p>
  </>;
}

export function Bolsillo() {
  const snap = useSnapshot(), [cfg, save] = useConfig(), now = useNow();
  const [tab, setTab] = useState<Tab>('ahora');
  const day = useDaySummary(snap);
  const bp = useRef<HTMLDivElement>(null), view = useRef<HTMLDivElement>(null), tabs = useRef<HTMLDivElement>(null), ind = useRef<HTMLElement>(null);

  // indicador de pestaña (moveInd)
  useLayoutEffect(() => {
    const cur = tabs.current?.querySelector<HTMLElement>(`[data-v="${tab}"]`); if (!cur || !ind.current) return;
    ind.current.style.width = cur.offsetWidth + 'px'; ind.current.style.transform = `translateX(${cur.offsetLeft - 6}px)`;
  }, [tab, cfg]);

  // aparece como en toggleBolsillo al mostrarse la ventana
  useEffect(() => {
    const show = () => { if (document.visibilityState !== 'visible' || !bp.current || reduceMotion()) return; bp.current.animate([{ opacity: 0, transform: 'translateY(14px) scale(.96)', filter: 'blur(4px)' }, { opacity: 1, transform: 'none', filter: 'blur(0)' }], { duration: 560, easing: 'cubic-bezier(.32,.72,0,1)' }); };
    window.addEventListener('focus', show); document.addEventListener('visibilitychange', show);
    return () => { window.removeEventListener('focus', show); document.removeEventListener('visibilitychange', show); };
  }, []);

  // Y / N resuelven el primer pendiente
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => { if ((e.target as HTMLElement).matches('input') || !snap.pending.length || e.repeat) return; const p = snap.pending[0], k = e.key.toLowerCase(); if (k === 'n') decide(p.id, false); if (k === 'y' && !(p.danger && cfg?.island.hold)) decide(p.id, true); };
    window.addEventListener('keydown', onKey); return () => window.removeEventListener('keydown', onKey);
  }, [snap, cfg]);

  const pick = (t: Tab) => { setTab(t); if (view.current && !reduceMotion()) view.current.animate([{ opacity: 0, transform: 'translateY(6px)' }, { opacity: 1, transform: 'none' }], { duration: 300, easing: 'cubic-bezier(.16,1,.3,1)' }); };
  if (!cfg) return null;
  return <div ref={bp} className="bp flex flex-col">
    <div ref={view} className="bp-view flex-1 overflow-auto px-4 pt-4 pb-2.5">
      {tab === 'ahora' && <Inicio snap={snap} cfg={cfg} save={save} now={now} />}
      {tab === 'ajustes' && <Ajustes cfg={cfg} save={save} />}
      {tab === 'dia' && <Hoy day={day} />}
    </div>
    <div ref={tabs} role="tablist" className="bp-tabs relative flex border-t border-white/[.07] p-1.5 gap-1">
      <i ref={ind} className="bp-ind" />
      {TABS.map(([k, ic, l]) => <button key={k} type="button" role="tab" data-v={k} aria-current={tab === k} className="relative flex-1 rounded-[9px] p-1.5 text-[11.5px] text-muted grid justify-items-center gap-0.5 transition-colors" onClick={() => pick(k)}><Ms style={{ fontSize: 19 }}>{ic}</Ms>{l}</button>)}
    </div>
  </div>;
}
