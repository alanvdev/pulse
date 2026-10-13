import type { Pending, Session, SessionState, Snapshot } from './bridge';
// Colores de los eventos en la versión oscura (spec §2.2). La isla siempre es oscura; los pulsos toman los suyos de config.
export const COLORS = { start: '#eaedea', permission: '#e3b05c', done: '#8fc3a0', idle: '#9ea39e', error: '#e08a7e', subagent: '#8fb0d6', focus: '#b4a6d9', calm: '#7d827d' } as const;
export const LBL: Record<SessionState, string> = { Idle: 'en calma', Done: 'terminó', Working: 'trabajando', IdleWait: 'te espera', Waiting: 'pide permiso' };
export const ICON: Record<SessionState, string> = { Idle: 'hourglass', Done: 'check', Working: 'progress_activity', IdleWait: 'hourglass', Waiting: 'front_hand' };
export const stateColor = (s: SessionState) => ({ Idle: '#9ea39e', Done: COLORS.done, Working: '#eaedea', IdleWait: '#9ea39e', Waiting: COLORS.permission })[s];
/** Color del icono de estado como variable CSS, para ventanas con tema claro u oscuro. */
export const STATE_VAR: Record<SessionState, string> = { Idle: 'var(--tx2)', Done: 'var(--done)', Working: 'var(--tx2)', IdleWait: 'var(--tx2)', Waiting: 'var(--perm)' };
/** Forma del punto de sesión (spec §3.1): lleno trabaja, ámbar con halo te necesita, anillo verde terminó, anillo gris en calma. */
export type DotKind = 'fill' | 'halo' | 'done' | 'calm';
export const dotKind = (s: SessionState): DotKind => s === 'Waiting' ? 'halo' : s === 'Working' ? 'fill' : s === 'Done' ? 'done' : 'calm';

const p2 = (n: number) => String(n).padStart(2, '0');
export function ago(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  if (s < 60) return `${s} s`;
  if (s < 3600) return `${Math.floor(s / 60)} min ${p2(s % 60)} s`;
  return `${Math.floor(s / 3600)} h ${p2(Math.floor(s / 60) % 60)} min`;
}
/** "6:40", "1:02:05": tiempo en una fila. */
export function clock(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000)), h = Math.floor(s / 3600), m = Math.floor(s / 60) % 60;
  return h ? `${h}:${p2(m)}:${p2(s % 60)}` : `${m}:${p2(s % 60)}`;
}
/** "3 h 12 min", "14 min": horas de trabajo y esperas. */
export function hm(ms: number): string { const m = Math.round(Math.max(0, ms) / 60000); return m >= 60 ? `${Math.floor(m / 60)} h ${m % 60} min` : `${m} min`; }
/** "6m 12s": cuánto duró un turno (aviso "terminó"). */
export function dur(ms: number): string { const s = Math.max(0, Math.round(ms / 1000)); return s >= 60 ? `${Math.floor(s / 60)}m ${s % 60}s` : `${s}s`; }
/** "40 s", "3 min", "2 h": "hace cuánto". */
export function since(ms: number): string { const s = Math.max(0, Math.floor(ms / 1000)); return s < 60 ? `${s} s` : s < 3600 ? `${Math.floor(s / 60)} min` : `${Math.floor(s / 3600)} h`; }
/** Hora local "09:05". */
export function hhmm(atMs: number): string { const d = new Date(atMs); return `${p2(d.getHours())}:${p2(d.getMinutes())}`; }
export const esc = (s: unknown) => String(s ?? '').replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]!));
/** Cuántas sesiones hay, cuántas te necesitan (permiso, pregunta o te esperan) y cuántas trabajan. */
export function counts(s: Snapshot) {
  return { total: s.sessions.length, need: s.sessions.filter(x => x.state === 'Waiting' || x.state === 'IdleWait').length, working: s.sessions.filter(x => x.state === 'Working').length };
}
/** "1 te necesita · 2 trabajan", "Todo en calma", "Sin sesiones" (flyout y Orbe). */
export function statusLine(s: Snapshot): string {
  const c = counts(s);
  if (!c.total) return 'Sin sesiones';
  const parts = [c.need ? `${c.need} te necesita${c.need > 1 ? 'n' : ''}` : '', c.working ? `${c.working} trabaja${c.working > 1 ? 'n' : ''}` : ''].filter(Boolean);
  return parts.length ? parts.join(' · ') : 'Todo en calma';
}
/** Punto que resume a todas, con las formas de la bandeja: ámbar solo si hay un permiso o pregunta esperando; "te espera" es anillo gris (spec §2.2). */
export function topDot(s: Snapshot): DotKind { const c = counts(s); return s.sessions.some(x => x.state === 'Waiting') ? 'halo' : c.working ? 'fill' : 'calm'; }
/** Qué hace la sesión, para filas de la isla, Bolsillo y Mosaico. `mono`: es un comando o un archivo. */
export function sessionWhat(s: Session, p: Pending | undefined, now: number): { text: string; mono: boolean } {
  if (p) return { text: `${p.kind === 'question' ? 'Pregunta' : 'Permiso'} · ${p.command}`, mono: true };
  if (s.state === 'Done') return { text: `Terminó hace ${since(now - s.since_ms)}`, mono: false };
  if (!s.last) return { text: LBL[s.state], mono: false };
  return { text: s.last + (s.subagents ? ` · ${s.subagents} subagente${s.subagents > 1 ? 's' : ''}` : ''), mono: true };
}
/** Frase de 20 px de Bolsillo (spec §7). */
export function headline(s: Snapshot): string {
  const c = counts(s);
  if (!c.total) return 'Sin sesiones';
  if (c.need) return c.need === 1 ? 'Una te necesita' : `${c.need} te necesitan`;
  if (c.working) return c.working === 1 ? 'Una trabajando' : `${c.working} trabajando`;
  return 'Todo en calma';
}
/** Dónde contestar una pregunta: la isla está arriba al centro hasta que la mueves con Ctrl+arrastrar (island.pos). */
export const answerHint = (pos: [number, number] | null) => pos ? 'Respóndela en la isla.' : 'Respóndela en la isla, arriba de tu pantalla.';
export const sessionsLabel = (n: number) => n === 1 ? '1 sesión' : `${n} sesiones`;
