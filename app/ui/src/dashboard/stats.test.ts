import { it, expect } from 'vitest';
import { dayStats } from './stats';
it('cifras del día', () => {
  const s = (max_wait_ms: number, perms: number, errors: number) => ({ project: 'p', work_ms: 0, waited_ms: 0, max_wait_ms, perms, errors, subagents: 0, actions: 0, lanes: [] });
  expect(dayStats({ sessions: [s(60_000, 2, 1), s(300_000, 3, 0)], total_work_ms: 9, total_waited_ms: 7, actions_by_hour: [] })).toEqual({ work: 9, waited: 7, maxWait: 300_000, perms: 5, errors: 1 });
  expect(dayStats(null)).toEqual({ work: 0, waited: 0, maxWait: 0, perms: 0, errors: 0 });
});
