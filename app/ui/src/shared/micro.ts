// Lógica pura de las microinteracciones (probada en micro.test.ts). Lo que toca el DOM vive en react.ts y en la isla.
import type { Pending, SessionState } from './bridge';

/** El modo foco acaba de encenderse o apagarse. Sin estado anterior (ventana recién abierta) no hay transición. */
export const focusEdge = (prev: boolean | undefined, next: boolean): 'enter' | 'exit' | null =>
  prev === undefined || prev === next ? null : next ? 'enter' : 'exit';

/** Quedan 5 s o menos para que la pregunta o el permiso se vaya a la terminal (y no está en pausa). */
export function isUrgent(p: Pending, now: number): boolean {
  if (p.paused) return false;
  const left = p.deadline_ms - now;
  return left > 0 && left <= 5_000;
}

/** Color del destello de una fila cuando su sesión cambia de estado; `null` si no merece destello. */
export function flashColor(prev: SessionState | undefined, next: SessionState): string | null {
  if (!prev || prev === next) return null;
  return next === 'Done' ? 'var(--done)' : next === 'Waiting' ? 'var(--perm)' : null;
}

/** Valor de una cifra que cuenta hacia `to` en el instante `t` (0 a 1), frenando al final. */
export const countAt = (t: number, to: number) => Math.round(to * (1 - Math.pow(1 - Math.min(1, Math.max(0, t)), 3)));
