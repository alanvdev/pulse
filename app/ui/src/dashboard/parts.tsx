// Piezas de Mosaico: bloque "Necesita tu respuesta" y tabla de sesiones (filas de 48 px).
import { useEffect, type CSSProperties, type KeyboardEvent as RKeyboardEvent, type ReactNode } from 'react';
import { decide, skip, type Config, type Pending, type Session, type Snapshot } from '../shared/bridge';
import { clock } from '../shared/format';
import { useFlash, useHold } from '../shared/react';
import type { Row } from '../island/model';
import { isUrgent } from '../shared/micro';
import { Button, Icon, StateIcon } from '../shared/ui';
import { sessionRows, verb } from '../island/model';

const secsLeft = (p: Pending, now: number) => `${Math.max(0, Math.ceil((p.paused ? p.left_ms : p.deadline_ms - now) / 1000))}s`;

export function PendingBlock({ p, cfg, now, more }: { p: Pending; cfg: Config; now: number; more: number }) {
  const danger = p.danger && cfg.island.hold, hold = useHold(() => decide(p.id, true));
  // N deniega, Y permite; en uno delicado, Y (o Enter/Espacio sobre el botón) se mantiene
  useEffect(() => {
    if (p.kind !== 'permission') return;
    const down = (e: KeyboardEvent) => {
      if (e.repeat || e.ctrlKey || e.altKey || e.metaKey || (e.target as HTMLElement).matches('input, textarea')) return;
      const k = e.key.toLowerCase();
      if (k === 'n') decide(p.id, false);
      if (k === 'y') { if (danger) hold.begin(); else decide(p.id, true); }
    };
    const up = (e: KeyboardEvent) => { if (e.key.toLowerCase() === 'y') hold.end(); };
    window.addEventListener('keydown', down); window.addEventListener('keyup', up);
    return () => { window.removeEventListener('keydown', down); window.removeEventListener('keyup', up); };
  }, [p.id, p.kind, danger]); // eslint-disable-line react-hooks/exhaustive-deps
  const keyHold = (on: boolean) => (e: RKeyboardEvent) => { if (!danger || (e.key !== 'Enter' && e.key !== ' ')) return; e.preventDefault(); if (on && !e.repeat) hold.begin(); if (!on) hold.end(); };
  return <div className="mz-pend">
    <Icon name={p.kind === 'question' ? 'help' : 'front_hand'} />
    <div className="mz-pend-b">
      <div className="mz-pend-h"><b>{p.project}</b> {p.kind === 'question' ? 'te pregunta' : verb(p.tool, true)}{cfg.island.countdown && <> · <span className={`mono ${isUrgent(p, now) ? 'urgent' : ''}`}>{secsLeft(p, now)}</span></>}{more > 0 && <span className="tx3"> · {more} más en espera</span>}</div>
      <div className={`mz-pend-c ${p.kind === 'question' ? '' : 'mono'}`}>{p.command}</div>
      {p.kind === 'question' ? <div className="mz-pend-d">Respóndela en la isla, arriba de tu pantalla.</div> : p.description && <div className="mz-pend-d">{p.description}</div>}
    </div>
    <div className="mz-pend-a">{p.kind === 'question'
      ? <Button solo size="lg" onClick={() => skip(p.id)}>Responder en la terminal</Button>
      : <><Button solo size="lg" keys={['N']} onClick={() => decide(p.id, false)}>Denegar</Button>
        <Button pri size="lg" keys={['Y']} hold={hold.p} onClick={() => { if (!danger) decide(p.id, true); }}
          onPointerDown={e => { if (danger) { e.preventDefault(); hold.begin(); } }} onPointerUp={hold.end} onPointerLeave={hold.end} onPointerCancel={hold.end}
          onKeyDown={keyHold(true)} onKeyUp={keyHold(false)}>{danger ? 'Mantener' : 'Permitir'}</Button></>}
    </div>
  </div>;
}

/** Tabla de sesiones. Con `renderDetail`, cada fila se expande al hacer clic (pantalla Sesiones). */
export function SessionTable({ snap, cfg, now, onOpen, expanded, renderDetail }: { snap: Snapshot; cfg: Config; now: number; onOpen: (id: string) => void; expanded?: string; renderDetail?: (s: Session) => ReactNode }) {
  const rows = sessionRows(snap, now);
  if (!rows.length) return <p className="empty">No hay sesiones de Claude Code abiertas.</p>;
  const keys = cfg.keyboard.enabled && cfg.keyboard.session_keys;
  return <div className="mz-table">{rows.map((r, i) => <TableRow key={r.id} r={r} i={i} s={snap.sessions.find(x => x.id === r.id)!} now={now} keys={keys}
    open={expanded === r.id} expandable={!!renderDetail} onOpen={onOpen} detail={renderDetail} />)}</div>;
}

/** Fila de 48 px: entra escalonada y destella cuando la sesión termina o pide permiso. */
function TableRow({ r, i, s, now, keys, open, expandable, onOpen, detail }: { r: Row; i: number; s: Session; now: number; keys: boolean; open: boolean; expandable: boolean; onOpen: (id: string) => void; detail?: (s: Session) => ReactNode }) {
  const st = r.pendingId ? 'Waiting' : r.state, ref = useFlash<HTMLDivElement>(st);
  return <div ref={ref} className="mz-rw row-in" data-sid={r.id} style={{ '--i': i } as CSSProperties}>
    <button type="button" className="mz-row" aria-expanded={expandable ? open : undefined} onClick={() => onOpen(r.id)}>
      <StateIcon s={st} /><span className="mz-p">{r.project}</span>
      <span className={`mz-w ${r.mono ? 'mono' : ''}`}>{r.what}</span>
      <span className="mono mz-t">{r.state === 'Done' ? '' : clock(now - r.since)}</span>
      {keys && s.slot < 3 ? <kbd className="kbd">F{s.slot + 1}</kbd> : <span />}
    </button>
    {detail && open && detail(s)}
  </div>;
}
