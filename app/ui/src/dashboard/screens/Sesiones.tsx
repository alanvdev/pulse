import { useEffect, useState } from 'react';
import { muteSession, type Config, type Session, type Snapshot } from '../../shared/bridge';
import { hhmm, statusLine } from '../../shared/format';
import { Button } from '../../shared/ui';
import { SessionTable } from '../parts';

function Detail({ s, snap, now }: { s: Session; snap: Snapshot; now: number }) {
  const feed = snap.feed.filter(f => f.session_id === s.id).slice(0, 8), muted = s.muted_until_ms > now;
  return <div className="mz-det">
    <div><h3>Últimas acciones</h3>{feed.length
      ? <ol>{feed.map((f, i) => <li key={i}><span className="mono tx3">{hhmm(f.at_ms)}</span><span className="mono">{f.text}</span></li>)}</ol>
      : <p className="empty">Sin acciones desde que abriste Pulse.</p>}</div>
    <dl>
      <div><dt>Subagentes</dt><dd>{s.subagents}</dd></div>
      <div><dt>Acciones</dt><dd>{s.actions}</dd></div>
      <div><dt>Tecla</dt><dd>{s.slot < 3 ? `F${s.slot + 1}` : '—'}</dd></div>
      <Button solo onClick={() => muteSession(s.id, muted ? 0 : 30)}>{muted ? 'Quitar silencio' : 'Silenciar 30 min'}</Button>
    </dl>
  </div>;
}

/** Todas las sesiones; cada fila se expande. `arg` (desde la isla) abre y muestra esa sesión. */
export function Sesiones({ snap, cfg, now, arg }: { snap: Snapshot; cfg: Config; now: number; arg?: string }) {
  const [open, setOpen] = useState<string | undefined>(arg);
  useEffect(() => { if (!arg) return; setOpen(arg); requestAnimationFrame(() => document.querySelector(`[data-sid="${CSS.escape(arg)}"]`)?.scrollIntoView({ block: 'nearest' })); }, [arg]);
  return <>
    <header className="mz-h"><h1>Sesiones</h1><span>{statusLine(snap)}</span></header>
    <SessionTable snap={snap} cfg={cfg} now={now} expanded={open} onOpen={id => setOpen(o => o === id ? undefined : id)} renderDetail={s => <Detail s={s} snap={snap} now={now} />} />
  </>;
}
