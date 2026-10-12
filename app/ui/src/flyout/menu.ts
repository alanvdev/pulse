/** Siguiente fila con flechas: salta las deshabilitadas y da la vuelta. `from` = -1 si no hay ninguna elegida. */
export function step(enabled: boolean[], from: number, dir: 1 | -1): number {
  const n = enabled.length;
  if (!enabled.some(Boolean)) return -1;
  let i = from < 0 ? (dir > 0 ? -1 : n) : from;
  for (let k = 0; k < n; k++) { i = (i + dir + n) % n; if (enabled[i]) return i; }
  return from;
}
