import { describe, it, expect } from 'vitest';
import { formatRoute, parseRoute } from './route';
describe('rutas de Mosaico', () => {
  it('pantalla y argumento', () => {
    expect(parseRoute('#sesiones/abc-123')).toEqual({ screen: 'sesiones', arg: 'abc-123' });
    expect(parseRoute('ajustes/pulsos')).toEqual({ screen: 'ajustes', arg: 'pulsos' });
    expect(parseRoute('hoy')).toEqual({ screen: 'hoy' });
  });
  it('vacía o desconocida va a Inicio', () => { expect(parseRoute('')).toEqual({ screen: 'inicio' }); expect(parseRoute(undefined)).toEqual({ screen: 'inicio' }); expect(parseRoute('#nada')).toEqual({ screen: 'inicio' }); });
  it('ida y vuelta', () => { expect(formatRoute({ screen: 'sesiones', arg: 'x' })).toBe('sesiones/x'); expect(formatRoute({ screen: 'hoy' })).toBe('hoy'); });
});
