import { describe, it, expect } from 'vitest';
import { resolveTheme } from './theme';
describe('tema', () => {
  it('oscuro y claro no dependen de Windows', () => { expect(resolveTheme('dark', true)).toBe('dark'); expect(resolveTheme('light', false)).toBe('light'); });
  it('"como Windows" sigue el modo del sistema', () => { expect(resolveTheme('system', true)).toBe('light'); expect(resolveTheme('system', false)).toBe('dark'); });
  it('sin ajuste (config vieja) queda oscuro', () => { expect(resolveTheme(undefined, true)).toBe('dark'); });
});
