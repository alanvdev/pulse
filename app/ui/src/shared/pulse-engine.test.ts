import { describe, it, expect } from 'vitest';
import { edgeMask, keyframesFor, slotX } from './pulse-engine';
describe('pulse engine', () => {
  it('edge mask has one gradient per side', () => {
    expect(edgeMask(100).split('linear-gradient').length - 1).toBe(4);
    expect(edgeMask(100, 'left').split('linear-gradient').length - 1).toBe(1);
  });
  it('latido has two peaks', () => {
    const k = keyframesFor('latido', 1, { x: 50, y: 50, side: 'left' });
    expect(k.filter(f => (f.opacity as number) > .8).length).toBe(2);
  });
  it('slots spread across the screen', () => { expect([0, 1, 2].map(slotX)).toEqual([18, 50, 82]); });
});
