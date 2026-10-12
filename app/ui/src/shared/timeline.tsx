// Línea de tiempo por proyecto (spec §8): trabajando en --work, esperando permiso en ámbar con rayas, errores como marca.
import { Fragment } from 'react';
import type { DaySummary } from './bridge';
import { ticks } from './day';
import { hm } from './format';
import './day.css';

/** `max`: cuántos proyectos mostrar (los más recientes); el resto se resume en una línea. */
export function Timeline({ day, now, label = 170, minGap, max }: { day: DaySummary | null; now: number; label?: number; minGap?: number; max?: number }) {
  if (!day) return null;
  if (!day.sessions.length) return <p className="empty">Todavía no hay actividad hoy.</p>;
  const shown = max ? day.sessions.slice(0, max) : day.sessions, rest = day.sessions.length - shown.length;
  return <div className="tl" style={{ gridTemplateColumns: `${label}px minmax(0,1fr)` }}>
    {shown.map((s, i) => <Fragment key={s.project + i}>
      <span className="tl-name">{s.project}</span>
      <div className="tl-lane" role="img" aria-label={`${s.project}: ${hm(s.work_ms)} trabajando, ${hm(s.waited_ms)} esperando tu permiso, ${s.errors} errores`}>
        {s.lanes.map(([a, b, k], j) => <i key={j} className={`k-${k}`} style={{ left: `${a * 100}%`, width: k === 'e' ? undefined : `${Math.max(.4, (b - a) * 100)}%` }} />)}
      </div>
    </Fragment>)}
    {rest > 0 && <><span className="tl-more">y {rest} más</span><span /></>}
    <span /><div className="tl-ticks mono" aria-hidden="true">{ticks(now, minGap).map(([f, l]) => <span key={l} style={{ left: `${f * 100}%` }}>{l}</span>)}</div>
  </div>;
}
export const TimelineLegend = () => <span className="tl-leg"><i className="k-w" />trabajando<i className="k-p" />esperando tu permiso<i className="k-e" />error</span>;
