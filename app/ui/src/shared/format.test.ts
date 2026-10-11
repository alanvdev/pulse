import { it, expect } from 'vitest';
import { ago } from './format';
it('formats elapsed time in Spanish', () => {
  expect(ago(4_000)).toBe('4 s'); expect(ago(125_000)).toBe('2 min 05 s'); expect(ago(3_900_000)).toBe('1 h 05 min');
});
