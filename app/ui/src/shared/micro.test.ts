import { describe, it, expect } from 'vitest';
import type { Pending } from './bridge';
import { countAt, flashColor, focusEdge, isUrgent } from './micro';

const p = (over: Partial<Pending>): Pending => ({ id: 1, kind: 'permission', session_id: 's', project: 'p', tool: 'Bash', command: 'ls', description: '', danger: false, at_ms: 0, timeout_ms: 30_000, deadline_ms: 30_000, paused: false, left_ms: 30_000, ...over });

describe('microinteracciones', () => {
  it('entrar y salir del foco', () => {
    expect(focusEdge(false, true)).toBe('enter');
    expect(focusEdge(true, false)).toBe('exit');
    expect(focusEdge(true, true)).toBeNull();
    expect(focusEdge(undefined, true)).toBeNull();   // al abrir la ventana ya en foco no se anima
  });
  it('los últimos 5 s de la cuenta regresiva', () => {
    expect(isUrgent(p({}), 24_000)).toBe(false);
    expect(isUrgent(p({}), 25_500)).toBe(true);
    expect(isUrgent(p({}), 30_000)).toBe(false);                       // ya se fue a la terminal
    expect(isUrgent(p({ paused: true, left_ms: 3_000 }), 0)).toBe(false);   // en pausa no apura
  });
  it('destello al cambiar de estado', () => {
    expect(flashColor('Working', 'Done')).toBe('var(--done)');
    expect(flashColor('Working', 'Waiting')).toBe('var(--perm)');
    expect(flashColor('Done', 'Working')).toBeNull();
    expect(flashColor(undefined, 'Done')).toBeNull();   // fila nueva: entra, no destella
  });
  it('cifras que cuentan con freno al final', () => {
    expect(countAt(0, 100)).toBe(0);
    expect(countAt(1, 100)).toBe(100);
    expect(countAt(.5, 100)).toBeGreaterThan(50);
  });
});
