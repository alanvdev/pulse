// Lógica pura de la isla (sin DOM): avisos, cola de permisos, preguntas. Probada en model.test.ts.
import type { Deferred, Fx, Pending } from '../shared/bridge';
import { COLORS, dur, hhmm } from '../shared/format';

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
      : N({ icon: 'help', color: COLORS.idle, who: fx.project, what: `te pregunta en la terminal: ${tail}`, life: 4500 });
    case 'release': return N({ icon: 'dark_mode', color: COLORS.idle, what: 'Mientras estabas en foco', items: JSON.parse(fx.text) as Deferred[], life: 4000 });
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
