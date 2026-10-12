import { describe, it, expect } from 'vitest';
import { step } from './menu';
describe('flechas del menú', () => {
  it('salta las filas deshabilitadas', () => expect(step([true, false, true], 0, 1)).toBe(2));
  it('da la vuelta en los extremos', () => { expect(step([true, true, true], 2, 1)).toBe(0); expect(step([true, true, true], 0, -1)).toBe(2); });
  it('sin fila elegida: abajo va a la primera, arriba a la última', () => { expect(step([true, true], -1, 1)).toBe(0); expect(step([true, true], -1, -1)).toBe(1); });
  it('sin filas habilitadas no hay selección', () => expect(step([false, false], 0, 1)).toBe(-1));
});
