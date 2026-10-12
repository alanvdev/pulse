import type { DaySummary } from '../../shared/bridge';
import { hhmm, hm } from '../../shared/format';
import { Timeline, TimelineLegend } from '../../shared/timeline';
import { dayStats } from '../stats';
import { useCountUp } from '../../shared/react';

/** Cifra de Hoy que cuenta desde cero la primera vez. */
function Stat({ label, value, fmt }: { label: string; value: number; fmt: (v: number) => string }) {
  const v = useCountUp(value);
  return <div className="mz-stat"><span>{label}</span><b className="mono">{fmt(v)}</b></div>;
}

function HourBars({ counts, now }: { counts: number[]; now: number }) {
  const h = new Date(now).getHours(), bars = counts.slice(0, h + 1), max = Math.max(1, ...bars), top = bars.indexOf(Math.max(...bars, 0));
  const total = bars.reduce((a, b) => a + b, 0), p2 = (n: number) => String(n).padStart(2, '0');
  return <div role="img" aria-label={total ? `${total} acciones hoy; la hora con más fue las ${p2(top)}:00 (${bars[top]})` : 'Sin acciones hoy'}>
    <div className="hb-bars">{bars.map((v, i) => <i key={i} className={i === h ? 'now' : ''} style={{ height: `${Math.max(3, v / max * 100)}%` }} title={`${p2(i)}:00 · ${v} acciones`} />)}</div>
    <div className="hb-ax mono"><span>00:00</span><span>{p2(h)}:00</span></div>
  </div>;
}

export function Hoy({ day, now }: { day: DaySummary | null; now: number }) {
  const st = dayStats(day);
  const cells: [string, number, (v: number) => string][] = [['Trabajo de Claude', st.work, hm], ['Te esperó', st.waited, hm], ['Espera más larga', st.maxWait, hm], ['Permisos', st.perms, String], ['Errores', st.errors, String]];
  return <>
    <header className="mz-h"><h1>Hoy</h1><span>00:00 a {hhmm(now)}</span></header>
    <div className="mz-stats">{cells.map(([l, v, f]) => <Stat key={l} label={l} value={v} fmt={f} />)}</div>
    <section className="mz-s"><div className="mz-s-h"><h2>Por proyecto</h2><TimelineLegend /></div><div className="mz-box"><Timeline day={day} now={now} /></div></section>
    <section className="mz-s"><h2>Acciones por hora</h2><div className="mz-box"><HourBars counts={day?.actions_by_hour ?? []} now={now} /></div></section>
  </>;
}
