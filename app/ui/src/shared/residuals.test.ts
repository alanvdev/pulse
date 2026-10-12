import { it, expect } from 'vitest';
import { wantedResiduals, IDLE_GLOW_MS } from './residuals';
import type { Snapshot } from './bridge';

const snap = (over: Partial<Snapshot>): Snapshot => ({ sessions: [], pending: [], feed: [], focus: false, focus_auto: false, deferred: [], ...over });
const sess = (id: string, state: any, since_ms: number) => ({ id, project: id, slot: 0, state, last: '', subagents: 0, since_ms, last_event_ms: since_ms, actions: 0, muted_until_ms: 0 });
const perm = (session_id: string) => ({ id: 1, kind: 'permission', session_id, project: '', tool: 'Bash', command: 'ls', description: '', danger: false, at_ms: 0, timeout_ms: 30_000, deadline_ms: 30_000, paused: false, left_ms: 0 }) as any;

it('"te espera" brilla solo un minuto; el permiso se queda hasta resolverse', () => {
  const s = snap({ sessions: [sess('a', 'IdleWait', 1_000)], pending: [perm('b')] });
  expect([...wantedResiduals(s, 1_000 + IDLE_GLOW_MS - 1)].sort()).toEqual(['a:idle:1000', 'b:permission']);
  expect([...wantedResiduals(s, 1_000 + IDLE_GLOW_MS)]).toEqual(['b:permission']);
  expect(IDLE_GLOW_MS).toBeLessThanOrEqual(60_000);
});
it('un nuevo "te espera" de la misma sesión vuelve a brillar (otra clave)', () => {
  expect([...wantedResiduals(snap({ sessions: [sess('a', 'IdleWait', 500_000)] }), 500_000)]).toEqual(['a:idle:500000']);
});
