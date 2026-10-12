import type { Snapshot } from './bridge';

/** Cuánto dura el brillo de "te espera" antes de apagarse solo (la sesión sigue marcada en la isla, Bolsillo y Mosaico). */
export const IDLE_GLOW_MS = 60_000;

/**
 * Brillos que deben quedar encendidos en los bordes. Clave `sesión:tipo[:desde]`:
 * - permiso pendiente: hasta que se resuelva;
 * - "te espera": un minuto desde que empezó (incluir `since_ms` hace que un nuevo "te espera" vuelva a brillar).
 */
export function wantedResiduals(s: Snapshot, now: number): Set<string> {
  const want = new Set<string>();
  for (const p of s.pending) want.add(`${p.session_id}:permission`);
  for (const x of s.sessions) if (x.state === 'IdleWait' && now - x.since_ms < IDLE_GLOW_MS) want.add(`${x.id}:idle:${x.since_ms}`);
  return want;
}
