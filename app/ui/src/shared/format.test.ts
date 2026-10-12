import { describe, it, expect } from 'vitest';
import { ago, clock, dotKind, dur, hhmm, hm, since } from './format';
describe('formatos', () => {
  it('tiempo transcurrido en español', () => { expect(ago(4_000)).toBe('4 s'); expect(ago(125_000)).toBe('2 min 05 s'); expect(ago(3_900_000)).toBe('1 h 05 min'); });
  it('reloj corto para filas', () => { expect(clock(400_000)).toBe('6:40'); expect(clock(5_000)).toBe('0:05'); expect(clock(3_725_000)).toBe('1:02:05'); });
  it('horas y minutos de trabajo', () => { expect(hm(14 * 60_000)).toBe('14 min'); expect(hm(192 * 60_000)).toBe('3 h 12 min'); expect(hm(0)).toBe('0 min'); });
  it('duración de un turno', () => { expect(dur(372_000)).toBe('6m 12s'); expect(dur(9_000)).toBe('9s'); });
  it('hace cuánto', () => { expect(since(40_000)).toBe('40 s'); expect(since(190_000)).toBe('3 min'); expect(since(7_300_000)).toBe('2 h'); });
  it('hora local', () => { expect(hhmm(new Date(2026, 9, 11, 9, 5).getTime())).toBe('09:05'); });
  it('forma del punto por estado', () => {
    expect(dotKind('Working')).toBe('fill'); expect(dotKind('Waiting')).toBe('halo'); expect(dotKind('Done')).toBe('done');
    expect(dotKind('IdleWait')).toBe('calm'); expect(dotKind('Idle')).toBe('calm');
  });
});
