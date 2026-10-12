import { describe, it, expect } from 'vitest';
import { buildAnswers, flow, type Pick } from './questions';
import type { Question } from '../shared/bridge';

const q = (question: string, multiSelect = false, labels = ['A', 'B']): Question => ({ question, multiSelect, options: labels.map(label => ({ label })) });
const pick = (labels: string[] = [], other = false, text = ''): Pick => ({ labels, other, text });

describe('respuestas', () => {
  it('una opción por pregunta', () => {
    expect(buildAnswers([q('¿Uno?'), q('¿Dos?')], [pick(['A']), pick(['B'])])).toEqual({ '¿Uno?': 'A', '¿Dos?': 'B' });
  });
  it('varias opciones se unen con coma, y "Otra" agrega lo escrito', () => {
    expect(buildAnswers([q('¿Partes?', true, ['Isla', 'Panel', 'Tablero'])], [pick(['Isla', 'Tablero'], true, 'Ajustes')])).toEqual({ '¿Partes?': 'Isla, Tablero, Ajustes' });
  });
  it('"Otra" sin texto no cuenta; si falta alguna pregunta no hay respuesta', () => {
    expect(buildAnswers([q('¿Uno?')], [pick([], true, '   ')])).toBeNull();
    expect(buildAnswers([q('¿Uno?'), q('¿Dos?')], [pick(['A']), pick()])).toBeNull();
  });
});

describe('pasos', () => {
  it('una sola pregunta de opción única se envía con un clic', () => {
    expect(flow([q('¿Uno?')], 0, [pick(['A'])], 'option')).toEqual({ step: 0, send: true });
  });
  it('varias preguntas: opción única avanza sola, la última no se envía sin confirmar', () => {
    const qs = [q('¿Uno?'), q('¿Dos?')];
    expect(flow(qs, 0, [pick(['A']), pick()], 'option')).toEqual({ step: 1, send: false });
    expect(flow(qs, 1, [pick(['A']), pick(['B'])], 'option')).toEqual({ step: 1, send: false });
    expect(flow(qs, 1, [pick(['A']), pick(['B'])], 'next')).toEqual({ step: 1, send: true });
  });
  it('selección múltiple y "Otra" no avanzan solas; Siguiente sin respuesta no avanza', () => {
    const qs = [q('¿Partes?', true), q('¿Dos?')];
    expect(flow(qs, 0, [pick(['A']), pick()], 'option')).toEqual({ step: 0, send: false });
    expect(flow([q('¿Uno?'), q('¿Dos?')], 0, [pick([], true, ''), pick()], 'other')).toEqual({ step: 0, send: false });
    expect(flow(qs, 0, [pick(), pick()], 'next')).toEqual({ step: 0, send: false });
    expect(flow(qs, 0, [pick(['A']), pick()], 'next')).toEqual({ step: 1, send: false });
  });
  it('Atrás regresa sin bajar de la primera', () => {
    expect(flow([q('a'), q('b')], 1, [pick(['A']), pick()], 'back')).toEqual({ step: 0, send: false });
    expect(flow([q('a'), q('b')], 0, [pick(), pick()], 'back')).toEqual({ step: 0, send: false });
  });
});
