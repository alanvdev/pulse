// Piezas de Mosaico: bloque "Necesita tu respuesta" y tabla de sesiones (filas de 48 px).
import type { ReactNode } from 'react';
import { decide, skip, type Config, type Pending, type Session, type Snapshot } from '../shared/bridge';
import { clock } from '../shared/format';
import { useHold } from '../shared/react';
import { Button, Icon, StateIcon } from '../shared/ui';
import { sessionRows, verb } from '../island/model';

const secsLeft = (p: Pending, now: number) => `${Math.max(0, Math.ceil((p.paused ? p.left_ms : p.deadline_ms - now) / 1000))}s`;

export function PendingBlock({ p, cfg, now, more }: { p: Pending; cfg: Config; now: number; more: number }) {
  const danger = p.danger && cfg.island.hold, hold = useHold(() => decide(p.id, true));
  return <div className="mz-pend">
    <Icon name={p.kind === 'question' ? 'help' : 'front_hand'} />
    <div className="mz-pend-b">
      <div className="mz-pend-h"><b>{p.project}</b> {p.kind === 'question' ? 'te pregunta' : verb(p.tool, true)}{cfg.island.countdown && <> · <span className="mono">{secsLeft(p, now)}</span></>}{more > 0 && <span className="tx3"> · {more} más en espera</span>}</div>
      <div className={`mz-pend-c ${p.kind === 'question' ? '' : 'mono'}`}>{p.command}</div>
      {p.kind === 'question' ? <div className="mz-pend-d">Respóndela en la isla, arriba de tu pantalla.</div> : p.description && <div className="mz-pend-d">{p.description}</div>}
    </div>
    <div className="mz-pend-a">{p.kind === 'question'
      ? <Button solo size="lg" onClick={() => skip(p.id)}>Responder en la terminal</Button>
      : <><Button solo size="lg" keys={['N']} onClick={() => decide(p.id, false)}>Denegar</Button>
        <Button pri size="lg" keys={['Y']} hold={hold.p} onClick={() => { if (!danger) decide(p.id, true); }}
          onPointerDown={e => { if (danger) { e.preventDefault(); hold.begin(); } }} onPointerUp={hold.end} onPointerLeave={hold.end}>{danger ? 'Mantener' : 'Permitir'}</Button></>}
    </div>
  </div>;
}

/** Tabla de sesiones. Con `renderDetail`, cada fila se expande al hacer clic (pantalla Sesiones). */
export function SessionTable({ snap, cfg, now, onOpen, expanded, renderDetail }: { snap: Snapshot; cfg: Config; now: number; onOpen: (id: string) => void; expanded?: string; renderDetail?: (s: Session) => ReactNode }) {
  const rows = sessionRows(snap, now);
  if (!rows.length) return <p className="empty">No hay sesiones de Claude Code abiertas.</p>;
  const keys = cfg.keyboard.enabled && cfg.keyboard.session_keys;
  return <div className="mz-table">{rows.map(r => { const s = snap.sessions.find(x => x.id === r.id)!, open = expanded === r.id;
    return <div key={r.id} className="mz-rw" data-sid={r.id}>
      <button type="button" className="mz-row" aria-expanded={renderDetail ? open : undefined} onClick={() => onOpen(r.id)}>
        <StateIcon s={r.pendingId ? 'Waiting' : r.state} /><span className="mz-p">{r.project}</span>
        <span className={`mz-w ${r.mono ? 'mono' : ''}`}>{r.what}</span>
        <span className="mono mz-t">{r.state === 'Done' ? '' : clock(now - r.since)}</span>
        {keys && s.slot < 3 ? <kbd className="kbd">F{s.slot + 1}</kbd> : <span />}
      </button>
      {renderDetail && open && renderDetail(s)}
    </div>; })}</div>;
}
