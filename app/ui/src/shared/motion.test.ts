import { describe, it, expect } from 'vitest';
import { entryGate, springPoints } from './motion';
describe('spring', () => {
  it('starts at 0, ends at 1 and settles', () => {
    const p = springPoints(260, 23);
    expect(p[0]).toBe(0); expect(p[p.length - 1]).toBe(1); expect(p.length).toBeLessThan(260);
  });
  it('an underdamped spring overshoots, a damped one does not', () => {
    expect(Math.max(...springPoints(260, 23))).toBeGreaterThan(1);
    expect(Math.max(...springPoints(340, 38))).toBeLessThanOrEqual(1.001);
  });
});
describe('entryGate', () => {
  it('lets one entry through per opening, even if the window signals twice', () => {
    const g = entryGate();
    expect(g(true, 1000)).toBe(true); expect(g(true, 1100)).toBe(false);   // visibilitychange y luego focus
    expect(g(true, 2000)).toBe(true);
  });
  it('hiding re-arms it, so a quick reopen still animates', () => {
    const g = entryGate();
    expect(g(true, 1000)).toBe(true); expect(g(false, 1200)).toBe(false); expect(g(true, 1300)).toBe(true);
  });
});
