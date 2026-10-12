import type { DaySummary } from '../shared/bridge';
/** Cifras de la pantalla Hoy: trabajo, te esperó, espera más larga, permisos y errores. */
export function dayStats(d: DaySummary | null) {
  const s = d?.sessions ?? [];
  return { work: d?.total_work_ms ?? 0, waited: d?.total_waited_ms ?? 0, maxWait: Math.max(0, ...s.map(x => x.max_wait_ms)), perms: s.reduce((a, x) => a + x.perms, 0), errors: s.reduce((a, x) => a + x.errors, 0) };
}
