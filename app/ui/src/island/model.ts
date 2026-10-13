// Lógica pura de la isla (sin DOM): avisos, cola de permisos, preguntas. Probada en model.test.ts.
import type { Config, DaySummary, Deferred, Fx, Pending, SessionState, Snapshot } from '../shared/bridge';
import { COLORS, dotKind, dur, hhmm, hm, sessionWhat, type DotKind } from '../shared/format';

/** Aviso breve (spec §3.4): `who` en negrita, `what` en tx2, `extra` en monoespaciada; `items` = "mientras estabas en foco". */
export interface Notice { id: number; icon: string; color: string; who?: string; what: string; extra?: string; life: number; items?: Deferred[] }
let seq = 0;
const N = (n: Omit<Notice, 'id'>): Notice => ({ id: ++seq, ...n });

export function toNotice(fx: Fx): Notice | null {
  const [head, ...rest] = fx.text.split('|'), tail = rest.join('|');
  switch (fx.kind) {
    case 'done': return fx.visual ? N({ icon: 'check', color: COLORS.done, who: fx.project, what: 'terminó', extra: fx.text ? dur(Number(fx.text)) : undefined, life: 2800 }) : null;
    case 'error': return fx.visual ? N({ icon: 'error', color: COLORS.error, who: fx.project, what: 'falló', extra: fx.text || undefined, life: 2800 }) : null;
    case 'notice': return head === 'plan'
      ? N({ icon: 'assignment', color: COLORS.idle, who: fx.project, what: 'terminó un plan, revísalo en la terminal', life: 4500 })
      : head === 'hold' ? N({ icon: 'touch_app', color: COLORS.permission, what: 'Es delicado: mantén presionado «Mantener» en la tarjeta', extra: tail || undefined, life: 3500 })
      : N({ icon: 'help', color: COLORS.idle, who: fx.project, what: `te pregunta en la terminal: ${tail}`, life: 4500 });
    case 'release': return N({ icon: 'dark_mode', color: COLORS.focus, what: 'Mientras estabas en foco', items: JSON.parse(fx.text) as Deferred[], life: 4000 });
    case 'resolved':
      switch (head) {
        case 'allow': return N({ icon: 'check', color: COLORS.done, what: 'Permitido', extra: tail, life: 1300 });
        case 'deny': return N({ icon: 'close', color: COLORS.error, what: 'Denegado', extra: tail, life: 1300 });
        case 'answer': return N({ icon: 'send', color: COLORS.idle, what: `Respuesta enviada: ${tail}`, life: 1800 });
        case 'skip': return N({ icon: 'terminal', color: COLORS.idle, what: 'Respóndela en la terminal', life: 1800 });
        case 'terminal': return N({ icon: 'terminal', color: COLORS.idle, what: 'Respondido en la terminal', life: 1600 });
        case 'none': return N({ icon: 'schedule', color: COLORS.permission, what: 'Sin respuesta: Claude te pregunta en la terminal', life: 2200 });
      }
      return null;
  }
  return null;
}

/** "Quiere ejecutar" para la terminal, "Quiere usar" para el resto de herramientas. */
export const verb = (tool: string, lower = false) => { const v = tool === 'Bash' || tool === 'PowerShell' ? 'Quiere ejecutar' : 'Quiere usar'; return lower ? v.toLowerCase() : v; };

/** Varios pendientes: posición ("1 de 3") y quién sigue (spec §3.3). */
export function queueInfo(pending: Pending[]): { pos: string; next: string; nextCmd: string } | null {
  if (pending.length < 2) return null;
  const n = pending[1];
  return { pos: `1 de ${pending.length}`, next: `Siguiente: ${n.project} ${n.kind === 'question' ? 'te pregunta' : verb(n.tool, true)}`, nextCmd: n.kind === 'question' ? '' : n.command };
}

const REL: Record<string, [string, string, string]> = {
  done: ['check', COLORS.done, 'terminó'], error: ['error', COLORS.error, 'falló una herramienta'], idle: ['hourglass', COLORS.idle, 'te espera'],
  permission: ['front_hand', COLORS.permission, 'pidió permiso'], start: ['play_arrow', COLORS.idle, 'empezó'], subagent: ['account_tree', COLORS.subagent, 'terminó un subagente'],
};
/** Una fila de "Mientras estabas en foco": icono, proyecto, qué pasó y a qué hora. */
export function releaseRow(d: Deferred) { const [icon, color, what] = REL[d.kind] ?? ['info', COLORS.idle, d.kind]; return { icon, color, project: d.project, text: `${what} · ${hhmm(d.at_ms)}` }; }

/** Texto a la izquierda de la barra de una pregunta. */
export const qHint = (multi: boolean, optionCount: number, chosen: number) => multi ? (chosen === 1 ? '1 elegida' : `${chosen} elegidas`) : `1 a ${Math.min(9, optionCount)} para elegir`;
/** Tecla 1–9 → índice de opción, o -1. */
export const digitIndex = (key: string, count: number) => { const n = Number(key); return /^[1-9]$/.test(key) && n <= Math.min(9, count) ? n - 1 : -1; };

export const PAL_EMPTY = 'Nada coincide. Prueba con "foco", "silenciar" o el nombre de un proyecto.';

/** Una fila del resumen de Comando (y de las tablas de sesiones). */
export interface Row { id: string; project: string; state: SessionState; dot: DotKind; what: string; mono: boolean; since: number; need: boolean; pendingId?: number }
const RANK: Record<SessionState, number> = { Waiting: 0, IdleWait: 1, Working: 2, Done: 3, Idle: 4 };
/** Sesiones en orden de urgencia (la que te necesita primero) y, a igual urgencia, por orden de llegada. */
export function sessionRows(s: Snapshot, now: number): Row[] {
  return [...s.sessions].sort((a, b) => RANK[a.state] - RANK[b.state] || a.slot - b.slot).map(x => {
    const p = s.pending.find(q => q.session_id === x.id), w = sessionWhat(x, p, now);
    return { id: x.id, project: x.project, state: x.state, dot: p ? 'halo' : dotKind(x.state), what: w.text, mono: w.mono, since: x.since_ms, need: !!p, pendingId: p?.id };
  });
}
/** Sesiones que se pueden quitar sin perder nada: no trabajan ni esperan una respuesta tuya en Pulse. */
export const inactiveIds = (s: Snapshot) => s.sessions.filter(x => x.state !== 'Working' && !s.pending.some(p => p.session_id === x.id)).map(x => x.id);
/** Puntos de sesión en reposo (spec §3.1): solo con el ajuste encendido y mientras alguna trabaja o espera. */
export const showDots = (s: Snapshot, c: Config) => c.island.dots && s.sessions.some(x => x.state === 'Working' || x.state === 'Waiting' || x.state === 'IdleWait');
/** Pie del resumen: horas de trabajo de hoy y cuánto te esperó Claude. */
export const dayLine = (d: DaySummary | null) => d ? `Hoy ${hm(d.total_work_ms)} · te esperó ${hm(d.total_waited_ms)}` : 'Hoy';

export type Side = 'left' | 'center' | 'right';
/** Al soltar el Orbe: la zona más cercana según la x (0 a 1) de su centro. */
export const nearestZone = (x: number): Side => x < 1 / 3 ? 'left' : x > 2 / 3 ? 'right' : 'center';
/** Forma del Orbe en reposo (spec §3.2): luna en foco, anillo casi transparente sin sesiones; si no, la sesión más urgente. */
export function orbDot(s: Snapshot): DotKind | 'moon' | 'none' {
  if (s.focus) return 'moon';
  if (!s.sessions.length) return 'none';
  const has = (st: SessionState) => s.sessions.some(x => x.state === st);
  return s.pending.length || has('Waiting') ? 'halo' : has('Working') ? 'fill' : has('Done') ? 'done' : 'calm';
}
export const focusLine = (s: Snapshot) => `En foco · ${s.deferred.length} guardado${s.deferred.length === 1 ? '' : 's'}`;

/** Zona de clics de la isla durante un resorte: el rectángulo de ahora (`r`) unido al final (`tw` × `th`), anclado como
 *  el CSS: arriba fijo; a la derecha o a la izquierda por ese borde, si no centrado. Sin tamaño final (NaN): `r` tal cual. */
export function growRect(r: { left: number; top: number; right: number; bottom: number }, tw: number, th: number, anchor: Side): { x: number; y: number; w: number; h: number } {
  let x = r.left, R = r.right, B = r.bottom;
  if (tw >= 0 && th >= 0) {
    const tx = anchor === 'right' ? r.right - tw : anchor === 'left' ? r.left : (r.left + r.right) / 2 - tw / 2;
    x = Math.min(x, tx); R = Math.max(R, tx + tw); B = Math.max(B, r.top + th);
  }
  return { x, y: r.top, w: R - x, h: B - r.top };
}
