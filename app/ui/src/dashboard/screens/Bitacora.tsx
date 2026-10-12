import { useState } from 'react';
import type { Snapshot } from '../../shared/bridge';
import { hhmm } from '../../shared/format';
import { useLogToday } from '../../shared/react';
import { Chip, Icon, Seg } from '../../shared/ui';
import { filterLog, LOG_ICON, LOG_KINDS, projectsOf, type LogKind } from '../log';

/** Eventos de hoy leídos de log.jsonl, con filtro por tipo y por proyecto (spec §13). */
export function Bitacora({ snap }: { snap: Snapshot }) {
  const items = useLogToday(snap), [project, setProject] = useState<string | null>(null), [kind, setKind] = useState<LogKind>('todo');
  const list = items ? filterLog(items, project, kind) : [], projects = items ? projectsOf(items) : [];
  return <>
    <header className="mz-h"><h1>Bitácora</h1><span>Hoy · {items ? `${items.length} eventos` : '…'}</span></header>
    <div className="mz-filters">
      <Seg label="Tipo de evento" value={kind} options={LOG_KINDS} onChange={setKind} />
      {projects.length > 1 && <span className="mz-chips" role="group" aria-label="Proyecto">
        <Chip on={!project} onClick={() => setProject(null)}>Todos</Chip>
        {projects.map(p => <Chip key={p} on={project === p} onClick={() => setProject(project === p ? null : p)}>{p}</Chip>)}
      </span>}
    </div>
    {items && !list.length
      ? <p className="empty">{items.length ? 'Nada con estos filtros.' : 'Todavía no hay eventos hoy.'}</p>
      : <ol className="mz-log">{list.map((i, n) => { const [ic, c] = LOG_ICON[i.kind] ?? ['info', 'var(--tx3)'];
          return <li key={n}><span className="mono tx3">{hhmm(i.at_ms)}</span><Icon name={ic} style={{ color: c }} /><span className="mz-log-p">{i.project}</span><span className={`mz-log-t ${i.kind === 'tool' || i.kind === 'permission' ? 'mono' : ''}`}>{i.text}</span></li>; })}</ol>}
  </>;
}
