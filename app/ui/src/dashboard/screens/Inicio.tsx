import type { CSSProperties } from 'react';
import type { Config, DaySummary, Snapshot } from '../../shared/bridge';
import { hm } from '../../shared/format';
import { Timeline, TimelineLegend } from '../../shared/timeline';
import { PendingBlock, SessionTable } from '../parts';
import { useResolved } from '../../shared/react';
import { ResolvedNote } from '../../shared/ui';
import type { Route } from '../route';

export function Inicio({ snap, cfg, day, now, go }: { snap: Snapshot; cfg: Config; day: DaySummary | null; now: number; go: (r: Route) => void }) {
  const top = snap.pending[0], resolved = useResolved();
  return <>
    <header className="mz-h"><h1>Inicio</h1>{day && <span>{hm(day.total_work_ms)} de trabajo hoy · te esperó {hm(day.total_waited_ms)}</span>}</header>
    {top && <section className="mz-s"><h2>Necesita tu respuesta</h2>
      {resolved && <ResolvedNote key={resolved.n} {...resolved} />}
      <PendingBlock key={top.id} p={top} cfg={cfg} now={now} more={snap.pending.length - 1} /></section>}
    {/* era el último: se pliega la sección entera para que Sesiones suba sin saltar */}
    {!top && resolved && <div key={resolved.n} className="fold" style={{ '--fold-gap': '-26px' } as CSSProperties}><div><section className="mz-s"><h2>Necesita tu respuesta</h2><ResolvedNote {...resolved} fold={false} /></section></div></div>}
    <section className="mz-s"><h2>Sesiones</h2><SessionTable snap={snap} cfg={cfg} now={now} onOpen={id => go({ screen: 'sesiones', arg: id })} /></section>
    <section className="mz-s"><div className="mz-s-h"><h2>Hoy</h2><TimelineLegend /></div><div className="mz-box"><Timeline day={day} now={now} max={5} /></div></section>
  </>;
}
