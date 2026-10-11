// Mosaico: tablero de piezas reacomodables. Portado de docs/artboards/centro-de-mando.html (WIDGETS, wBody, renderMosaico con FLIP, arrastre, renderSheet, miniPulse) a React + Tailwind.
import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import { decide, setFocus, skip, type Config, type DaySummary, type Pending, type Snapshot } from '../shared/bridge';
import { ICON, ago, stateColor } from '../shared/format';
import { reduceMotion, spring } from '../shared/motion';
import { pulse } from '../shared/pulse-engine';
import { useConfig, useDaySummary, useHold, useNow, useSnapshot } from '../shared/react';

const WIDGETS: Record<string, { name: string; icon: string; size: string }> = {
  sess: { name: 'Sesiones', icon: 'monitoring', size: 's22' },
  pend: { name: 'Pendiente', icon: 'front_hand', size: 's21' },
  hoy: { name: 'Hoy', icon: 'schedule', size: '' },
  acc: { name: 'Acciones', icon: 'bolt', size: '' },
  foco: { name: 'Foco', icon: 'dark_mode', size: '' },
  pulso: { name: 'Pulso', icon: 'blur_on', size: 's12' },
  keys: { name: 'Teclas por sesión', icon: 'keyboard', size: 's21' },
  bitacora: { name: 'Bitácora', icon: 'timeline', size: 's21' },
};
const SP = spring(230, 22), SPS = spring(300, 30);
const Ms = ({ children, style }: { children: ReactNode; style?: CSSProperties }) => <span className="ms" style={style}>{children}</span>;
const cssVar = (v: Record<string, string | number>) => v as CSSProperties;
const hm = (ms: number) => { const m = Math.round(ms / 60000); return `${Math.floor(m / 60)}h ${m % 60}m`; };
const mins = (ms: number) => `${Math.round(ms / 60000)} min`;

function Pend({ p, more, day }: { p?: Pending; more: number; day: DaySummary | null }) {
  const [cfg] = useConfig();
  const danger = !!p?.danger && !!cfg?.island.hold;
  const hold = useHold(() => p && decide(p.id, true));
  if (!p) return <><h4><Ms>check_circle</Ms>Pendiente</h4><div className="big" style={{ fontSize: 20 }}>Nada pendiente</div><div className="sub">Te esperó {day ? mins(day.total_waited_ms) : '—'} hoy</div></>;
  if (p.kind === 'question') return <><h4><Ms style={{ color: '#ffb02e' }}>help</Ms>{p.project} te pregunta</h4><div className="text-[13px] text-[#e6e8eb] line-clamp-2">{p.command}</div><div className="flex gap-1.5 mt-auto items-center"><span className="sub flex-1">Respóndela en la isla</span><button className="wb" type="button" onClick={() => skip(p.id)}>En la terminal</button></div></>;
  return <><h4><Ms style={{ color: '#ffb02e' }}>front_hand</Ms>{p.project} pide permiso</h4><code>{p.command}</code>
    <div className="flex gap-1.5 mt-auto items-center">
      <button className="wb" type="button" onClick={() => decide(p.id, false)}>Denegar</button>
      <button className="wb y" type="button" style={cssVar({ '--hold': hold.p })} onClick={() => { if (!danger) decide(p.id, true); }} onPointerDown={e => { if (danger) { e.preventDefault(); hold.begin(); } }} onPointerUp={hold.end} onPointerLeave={hold.end}><span className="fill" /><span>{danger ? 'Mantener' : 'Permitir'}</span></button>
      {more > 0 && <span className="sub">+{more} más</span>}
    </div></>;
}

const PREVIEW: [string, string][] = [['start', 'Empieza'], ['permission', 'Permiso'], ['done', 'Terminó'], ['idle', 'Te espera'], ['error', 'Error'], ['subagent', 'Subagente']];
function Pulso({ cfg }: { cfg: Config }) {
  const mini = useRef<HTMLDivElement>(null);
  const fire = (kind: string) => { const pc = cfg.pulses.events[kind]; if (mini.current && pc) pulse(mini.current, { ...pc, thickness: Math.min(pc.thickness, 26) }, { x: 50, y: 50, side: 'left' }, { crisp: cfg.pulses.crisp, corners: cfg.pulses.corners, adapt: false, dither: cfg.pulses.dither, lightBackground: false, reduce: reduceMotion() }); };
  return <><h4><Ms>blur_on</Ms>Pulso</h4><div className="mini" ref={mini} /><div className="flex flex-wrap gap-1.5 mt-auto">{PREVIEW.map(([k, l]) => <button key={k} className="wb" type="button" title={cfg.pulses.events[k]?.shape} onClick={() => fire(k)}>{l}</button>)}</div></>;
}

function Bitacora({ day }: { day: DaySummary | null }) {
  const now = new Date(), label = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;
  return <><h4><Ms>timeline</Ms>Bitácora</h4>
    <div className="grid gap-1.5 overflow-auto min-h-0">
      {day?.sessions.length ? day.sessions.map((s, i) => <div key={s.project + i} className="grid grid-cols-[110px_minmax(0,1fr)_auto] gap-2 items-center text-[11.5px]">
        <span className="truncate text-[#c7ccd3]">{s.project}</span>
        <div className="lane">{s.lanes.map(([a, b, k], j) => <i key={j} className={k} style={{ left: `${a * 100}%`, width: `${Math.max(.3, (b - a) * 100)}%` }} />)}</div>
        <span className="font-mono text-[11px] text-[#8f96a0]">te esperó {mins(s.waited_ms)}</span>
      </div>) : <div className="sub">Todavía no hay actividad hoy.</div>}
    </div>
    <div className="flex justify-between font-mono text-[10.5px] text-[#5d646e] mt-auto pl-[118px] pr-[86px]"><span>00:00</span><span>{label}</span></div></>;
}

function Body({ id, snap, cfg, day, now }: { id: string; snap: Snapshot; cfg: Config; day: DaySummary | null; now: number }) {
  switch (id) {
    case 'sess': return <><h4><Ms>monitoring</Ms>Sesiones</h4><div className="grid gap-2 overflow-auto">{snap.sessions.length ? snap.sessions.map(s => <div key={s.id} className="s" style={cssVar({ '--c': stateColor(s.state) })}><span className={`ms ${s.state === 'Working' ? 'ic-spin' : ''}`}>{ICON[s.state]}</span><span className="min-w-0">{s.project}<small>{s.last}</small></span><span className="font-mono text-[11px] text-[#8f96a0]">{ago(now - s.since_ms)}</span></div>) : <div className="sub">No hay sesiones de Claude Code abiertas.</div>}</div></>;
    case 'pend': return <Pend p={snap.pending[0]} more={snap.pending.length - 1} day={day} />;
    case 'hoy': return <><h4><Ms>schedule</Ms>Hoy</h4><div className="big">{day ? hm(day.total_work_ms) : '—'}</div><div className="sub">de trabajo de Claude</div></>;
    case 'acc': { const h = new Date().getHours(), bars = (day?.actions_by_hour ?? []).slice(0, h + 1).slice(-16), max = Math.max(1, ...bars), total = (day?.sessions ?? []).reduce((a, s) => a + s.actions, 0);
      return <><h4><Ms>bolt</Ms>Acciones</h4><div className="big">{total}</div><div className="spark flex items-end gap-[3px] h-[38px] mt-auto">{bars.map((v, i) => <i key={i} style={{ height: `${Math.max(4, v / max * 100)}%` }} />)}</div></>; }
    case 'foco': return <><h4><Ms>dark_mode</Ms>Foco</h4><div className="big" style={{ fontSize: 20 }}>{snap.focus ? 'Activo' : 'Apagado'}</div><div className="flex gap-1.5 mt-auto"><button className={`wb ${snap.focus ? '' : 'y'}`} type="button" onClick={() => setFocus(!snap.focus)}>{snap.focus ? 'Salir' : 'Activar 1 h'}</button></div></>;
    case 'pulso': return <Pulso cfg={cfg} />;
    case 'keys': return <><h4><Ms>keyboard</Ms>Teclas por sesión</h4><div className="keys flex gap-1.5 mt-auto">{[0, 1, 2].map(slot => { const s = snap.sessions.find(x => x.slot === slot); const lit = !!s && (s.state === 'Waiting' || s.state === 'IdleWait'); return <span key={slot} className={lit ? 'lit' : ''} style={cssVar({ '--c': s ? stateColor(s.state) : '#69717c' })} title={s?.project ?? 'sin sesión'}>F{slot + 1}</span>; })}</div><div className="sub">Se encienden cuando esa sesión te espera</div></>;
    case 'bitacora': return <Bitacora day={day} />;
  }
  return null;
}

export function Mosaico() {
  const snap = useSnapshot(), [cfg, save] = useConfig(), now = useNow(), day = useDaySummary(snap);
  const [edit, setEdit] = useState(false), [sheet, setSheet] = useState(false);
  const grid = useRef<HTMLDivElement>(null);
  const prev = useRef<Record<string, DOMRect>>({});
  const drag = useRef<{ id: string; x: number; y: number; el: HTMLElement; r0?: DOMRect } | null>(null);
  const [order, setOrder] = useState<string[]>([]);
  useEffect(() => { if (cfg && !drag.current) setOrder(cfg.mosaico.order.filter(id => WIDGETS[id]).concat(Object.keys(WIDGETS).filter(id => !cfg.mosaico.order.includes(id)))); }, [cfg]);
  const hidden = new Set(cfg?.mosaico.hidden ?? []);

  const measure = () => { prev.current = {}; grid.current?.querySelectorAll<HTMLElement>('.w[data-w]').forEach(e => (prev.current[e.dataset.w!] = e.getBoundingClientRect())); };
  // FLIP: anima de la posición anterior a la nueva
  useLayoutEffect(() => {
    const d = drag.current;
    grid.current?.querySelectorAll<HTMLElement>('.w[data-w]').forEach(e => {
      const p = prev.current[e.dataset.w!]; if (!p) return; const n = e.getBoundingClientRect();
      if (d && e.dataset.w === d.id) { if (d.r0) { d.x += n.left - d.r0.left; d.y += n.top - d.r0.top; d.r0 = undefined; } return; }
      const dx = p.left - n.left, dy = p.top - n.top; if ((dx || dy) && !reduceMotion()) e.animate([{ transform: `translate(${dx}px,${dy}px)` }, { transform: 'none' }], { duration: SPS.d, easing: SPS.e });
    });
    prev.current = {};
  }, [order, cfg?.mosaico.hidden.join(), edit]);

  // arrastre en modo edición
  useEffect(() => {
    const down = (e: PointerEvent) => { if (!edit) return; const w = (e.target as HTMLElement).closest<HTMLElement>('.w[data-w]'); if (!w || (e.target as HTMLElement).closest('button')) return; drag.current = { id: w.dataset.w!, x: e.clientX, y: e.clientY, el: w }; w.classList.add('dragging'); w.setPointerCapture(e.pointerId); e.preventDefault(); };
    const move = (e: PointerEvent) => {
      const d = drag.current; if (!d) return;
      d.el.style.transform = `translate(${e.clientX - d.x}px,${e.clientY - d.y}px) scale(1.03)`;
      d.el.style.pointerEvents = 'none'; const over = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('.w[data-w]'); d.el.style.pointerEvents = '';
      if (over && over.dataset.w !== d.id) {
        d.el.style.transform = ''; measure(); d.r0 = prev.current[d.id];   // posición sin arrastre, para recolocar el punto de agarre tras reordenar
        setOrder(o => { const n = [...o], a = n.indexOf(d.id), b = n.indexOf(over.dataset.w!); n.splice(a, 1); n.splice(b, 0, d.id); return n; });
      }
    };
    const up = () => { const d = drag.current; if (!d) return; const el = d.el; el.classList.remove('dragging'); const t = el.style.transform; el.style.transform = ''; if (!reduceMotion() && t) el.animate([{ transform: t }, { transform: 'none' }], { duration: SPS.d, easing: SPS.e }); drag.current = null; setOrder(o => { if (cfg) save({ ...cfg, mosaico: { ...cfg.mosaico, order: o } }); return o; }); };
    document.addEventListener('pointerdown', down); document.addEventListener('pointermove', move); document.addEventListener('pointerup', up); document.addEventListener('pointercancel', up);
    return () => { document.removeEventListener('pointerdown', down); document.removeEventListener('pointermove', move); document.removeEventListener('pointerup', up); document.removeEventListener('pointercancel', up); };
  }, [edit, cfg, save]);

  // cerrar la hoja al hacer clic fuera
  useEffect(() => { if (!sheet) return; const f = (e: MouseEvent) => { if (!(e.target as HTMLElement).closest('.sheet, [data-act="msheet"]')) setSheet(false); }; document.addEventListener('click', f); return () => document.removeEventListener('click', f); }, [sheet]);

  if (!cfg) return null;
  const setHidden = (h: Set<string>) => { measure(); save({ ...cfg, mosaico: { order, hidden: [...h] } }); };
  const visible = order.filter(id => !hidden.has(id));
  return <div className={`h-full flex flex-col relative ${edit ? 'edit' : ''}`}>
    <div className="h-11 flex-none flex items-center gap-2.5 px-4 text-xs text-[#8f96a0] border-b border-[#1b1e23]">
      <span className="flex-1">{snap.sessions.length ? `${snap.sessions.length} sesión${snap.sessions.length > 1 ? 'es' : ''} · ${snap.pending.length ? `${snap.pending.length} pendiente${snap.pending.length > 1 ? 's' : ''}` : 'nada pendiente'}` : 'Sin sesiones'}</span>
      <button className="tbtn" type="button" aria-pressed={edit} onClick={() => { measure(); setEdit(!edit); setSheet(false); }}><Ms>edit</Ms><span>{edit ? 'Listo' : 'Editar'}</span></button>
    </div>
    <div ref={grid} className={`grid-m flex-1 overflow-auto p-3.5 grid gap-2.5 content-start ${edit ? 'edit' : ''}`}>
      {visible.map(id => <div key={id} data-w={id} className={`w ${WIDGETS[id].size} ${id === 'pend' && snap.pending.length ? 'pend' : ''} ${id === 'foco' ? 'moon' + (snap.focus ? ' on' : '') : ''}`}>
        <button className="x" type="button" aria-label={`Quitar ${WIDGETS[id].name}`} onClick={() => setHidden(new Set([...hidden, id]))}><Ms>remove</Ms></button>
        <Body id={id} snap={snap} cfg={cfg} day={day} now={now} />
      </div>)}
      <button className="w add" type="button" data-act="msheet" onClick={() => setSheet(!sheet)}><Ms>add</Ms>Agregar pieza</button>
    </div>
    {sheet && <div className="sheet"><h5 className="mx-1.5 mt-0.5 mb-1.5 text-[11.5px] font-medium text-[#8f96a0]">Agregar pieza</h5>
      {hidden.size ? [...hidden].filter(id => WIDGETS[id]).map(id => <button key={id} type="button" onClick={() => { const h = new Set(hidden); h.delete(id); setHidden(h); setSheet(false); requestAnimationFrame(() => { const n = grid.current?.querySelector<HTMLElement>(`.w[data-w="${id}"]`); if (n && !reduceMotion()) n.animate([{ opacity: 0, transform: 'scale(.9)' }, { opacity: 1, transform: 'none' }], { duration: SP.d, easing: SP.e }); }); }}><Ms>{WIDGETS[id].icon}</Ms>{WIDGETS[id].name}</button>)
        : <p className="m-1.5 text-[12.5px] text-[#8f96a0]">Todas las piezas están en el tablero. Quita alguna con − para verla aquí.</p>}
    </div>}
  </div>;
}
