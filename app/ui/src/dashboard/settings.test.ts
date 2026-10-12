import { describe, it, expect } from 'vitest';
import { finishOf, seconds, silentLine } from './settings';
describe('acabado (spec §13)', () => {
  it('completo y suave', () => {
    expect(finishOf({ crisp: true, corners: true, adapt: true, dither: true })).toBe('completo');
    expect(finishOf({ crisp: false, corners: true, adapt: true, dither: false })).toBe('suave');
  });
  it('una mezcla es "Personalizado" (ningún segmento)', () => expect(finishOf({ crisp: true, corners: false, adapt: true, dither: true })).toBeNull());
});
describe('textos', () => {
  it('segundos con coma', () => { expect(seconds(1100)).toBe('1,1 s'); expect(seconds(650)).toBe('0,7 s'); });
  it('qué eventos callan', () => {
    expect(silentLine({ start: true, permission: true, done: true, idle: true, error: true, subagent: false })).toBe('Subagente está en silencio');
    expect(silentLine({ start: true, permission: true, done: true, idle: true, error: true, subagent: true })).toBe('Suenan todos');
    expect(silentLine({ start: false, permission: true, done: true, idle: true, error: true, subagent: false })).toBe('Empieza, Subagente están en silencio');
    expect(silentLine({})).toBe('Ninguno suena');
  });
});
