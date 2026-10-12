// Línea de tiempo del día (de medianoche a ahora): marcas de hora. Las barras vienen de daily_summary.
import { hhmm } from './format';
export const dayStart = (now: number) => { const d = new Date(now); d.setHours(0, 0, 0, 0); return d.getTime(); };
/** 00:00, 06:00, 12:00 y 18:00 si ya pasaron (y no chocan con la última), y la hora actual al final. Posición de 0 a 1. */
export function ticks(now: number): [number, string][] {
  const span = Math.max(1, now - dayStart(now)), out: [number, string][] = [];
  for (const h of [0, 6, 12, 18]) { const f = (h * 3_600_000) / span; if (h === 0 || f < 0.92) out.push([f, `${String(h).padStart(2, '0')}:00`]); }
  out.push([1, hhmm(now)]);
  return out;
}
