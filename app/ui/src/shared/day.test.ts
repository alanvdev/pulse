import { describe, it, expect } from 'vitest';
import { ticks } from './day';
const at = (h: number, m = 0) => new Date(2026, 9, 11, h, m).getTime();
describe('marcas de la línea de tiempo', () => {
  it('de noche se ven las cinco', () => {
    expect(ticks(at(23, 42)).map(t => t[1])).toEqual(['00:00', '06:00', '12:00', '18:00', '23:42']);
    expect(ticks(at(23, 42))[1][0]).toBeCloseTo(6 / 23.7, 3);
  });
  it('temprano solo las que ya pasaron, y la de ahora al final', () => {
    expect(ticks(at(7)).map(t => t[1])).toEqual(['00:00', '06:00', '07:00']);
    expect(ticks(at(6, 10)).map(t => t[1])).toEqual(['00:00', '06:10']);   // 06:00 chocaría con la de ahora
    expect(ticks(at(7), 0.2).map(t => t[1])).toEqual(['00:00', '07:00']);   // en Bolsillo, más espacio
  });
});
