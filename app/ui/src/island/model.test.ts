import { describe, it, expect } from 'vitest';
import type { Config, Fx, Pending, Session, SessionState, Snapshot } from '../shared/bridge';
import { COLORS } from '../shared/format';
import { dayLine, digitIndex, focusLine, inactiveIds, nearestZone, orbDot, qHint, queueInfo, releaseRow, sessionRows, showDots, toNotice, verb } from './model';

const fx = (kind: Fx['kind'], text = '', visual = true): Fx => ({ kind, session_id: 's', project: 'pulse', slot: 0, text, visual, sound: false, monitor: 0 });
const pend = (id: number, project: string, command: string, tool = 'Bash', kind: Pending['kind'] = 'permission'): Pending =>
  ({ id, kind, session_id: 's' + id, project, tool, command, description: '', danger: false, at_ms: 0, timeout_ms: 30_000, deadline_ms: 30_000, paused: false, left_ms: 30_000 });

describe('avisos (spec §3.4)', () => {
  const n = (f: Fx) => { const x = toNotice(f)!; return [x.icon, x.color, x.who ?? '', x.what, x.extra ?? '', x.life]; };
  it('terminó y falló', () => {
    expect(n(fx('done', '372000'))).toEqual(['check', COLORS.done, 'pulse', 'terminó', '6m 12s', 2800]);
    expect(n(fx('done'))).toEqual(['check', COLORS.done, 'pulse', 'terminó', '', 2800]);
    expect(n(fx('error', 'npm test (exit 1)'))).toEqual(['error', COLORS.error, 'pulse', 'falló', 'npm test (exit 1)', 2800]);
  });
  it('terminó y falló no se avisan si el pulso está apagado o en foco', () => { expect(toNotice(fx('done', '1', false))).toBeNull(); expect(toNotice(fx('error', 'x', false))).toBeNull(); });
  it('plan y pregunta en la terminal', () => {
    expect(n(fx('notice', 'plan|'))).toEqual(['assignment', COLORS.idle, 'pulse', 'terminó un plan, revísalo en la terminal', '', 4500]);
    expect(n(fx('notice', 'question|¿Qué base uso?'))).toEqual(['help', COLORS.idle, 'pulse', 'te pregunta en la terminal: ¿Qué base uso?', '', 4500]);
  });
  it('respuestas', () => {
    expect(n(fx('resolved', 'allow|git push'))).toEqual(['check', COLORS.done, '', 'Permitido', 'git push', 1300]);
    expect(n(fx('resolved', 'deny|rm -rf x'))).toEqual(['close', COLORS.error, '', 'Denegado', 'rm -rf x', 1300]);
    expect(n(fx('resolved', 'answer|Postgres'))).toEqual(['send', COLORS.idle, '', 'Respuesta enviada: Postgres', '', 1800]);
    expect(n(fx('resolved', 'skip|x'))).toEqual(['terminal', COLORS.idle, '', 'Respóndela en la terminal', '', 1800]);
    expect(n(fx('resolved', 'terminal|x'))).toEqual(['terminal', COLORS.idle, '', 'Respondido en la terminal', '', 1600]);
    expect(n(fx('resolved', 'none|x'))).toEqual(['schedule', COLORS.permission, '', 'Sin respuesta: Claude te pregunta en la terminal', '', 2200]);
  });
  it('salida del foco: lista de lo guardado, 4 s', () => {
    const x = toNotice(fx('release', JSON.stringify([{ session_id: 's', project: 'p', kind: 'done', at_ms: 0 }])))!;
    expect([x.items?.length, x.life]).toEqual([1, 4000]);
  });
  it('los pulsos sueltos no son avisos', () => { expect(toNotice(fx('start'))).toBeNull(); expect(toNotice(fx('permission'))).toBeNull(); });
});

describe('tarjetas', () => {
  it('verbo según la herramienta', () => { expect(verb('Bash', true)).toBe('quiere ejecutar'); expect(verb('Edit')).toBe('Quiere usar'); });
  it('cola de permisos', () => {
    expect(queueInfo([pend(1, 'a', 'ls')])).toBeNull();
    expect(queueInfo([pend(1, 'a', 'ls'), pend(2, 'pulse', 'git push --force'), pend(3, 'c', 'x')])).toEqual({ pos: '1 de 3', next: 'Siguiente: pulse quiere ejecutar', nextCmd: 'git push --force' });
    expect(queueInfo([pend(1, 'a', 'ls'), pend(2, 'b', '¿Uno?', 'AskUserQuestion', 'question')])?.next).toBe('Siguiente: b te pregunta');
  });
  it('fila de "mientras estabas en foco"', () => {
    const at = new Date(2026, 9, 11, 10, 42).getTime();
    expect(releaseRow({ session_id: 's', project: 'landing', kind: 'done', at_ms: at })).toEqual({ icon: 'check', color: COLORS.done, project: 'landing', text: 'terminó · 10:42' });
    expect(releaseRow({ session_id: 's', project: 'api', kind: 'error', at_ms: at }).text).toBe('falló una herramienta · 10:42');
    expect(releaseRow({ session_id: 's', project: 'p', kind: 'idle', at_ms: at }).icon).toBe('hourglass');
  });
});

describe('preguntas', () => {
  it('pista de la barra', () => {
    expect(qHint(false, 3, 0)).toBe('1 a 3 para elegir');
    expect(qHint(false, 12, 0)).toBe('1 a 9 para elegir');
    expect(qHint(true, 3, 1)).toBe('1 elegida');
    expect(qHint(true, 3, 2)).toBe('2 elegidas');
  });
  it('teclas 1 a 9', () => { expect(digitIndex('2', 3)).toBe(1); expect(digitIndex('4', 3)).toBe(-1); expect(digitIndex('0', 3)).toBe(-1); expect(digitIndex('a', 3)).toBe(-1); });
});

const ses = (id: string, state: SessionState, slot = 0, over: Partial<Session> = {}): Session => ({ id, project: id, slot, state, last: '', subagents: 0, since_ms: 0, last_event_ms: 0, actions: 0, muted_until_ms: 0, ...over });
const snap = (sessions: Session[], pending: Pending[] = []): Snapshot => ({ sessions, pending, feed: [], focus: false, focus_auto: false, deferred: [] });
const cfgDots = (dots: boolean) => ({ island: { dots } }) as Config;

describe('puntos y resumen de Comando', () => {
  it('orden: te necesita, te espera, trabaja, terminó, en calma; empate por llegada', () => {
    const s = snap([ses('d', 'Done', 0), ses('w', 'Working', 1), ses('i', 'Idle', 2), ses('q', 'IdleWait', 3), ses('p', 'Waiting', 4)]);
    expect(sessionRows(s, 0).map(r => r.id)).toEqual(['p', 'q', 'w', 'd', 'i']);
  });
  it('la que tiene un pendiente se marca, con punto ámbar y su pendiente', () => {
    const p = { ...pend(7, 'api', 'npm run migrate'), session_id: 'api' };
    const [r] = sessionRows(snap([ses('api', 'Waiting')], [p]), 0);
    expect([r.need, r.dot, r.pendingId, r.what]).toEqual([true, 'halo', 7, 'Permiso · npm run migrate']);
  });
  it('qué hace cada una', () => {
    const rows = sessionRows(snap([ses('w', 'Working', 0, { last: 'Edit orbe.ts', subagents: 2, since_ms: 0 })]), 400_000);
    expect([rows[0].what, rows[0].mono, rows[0].dot, rows[0].state]).toEqual(['Edit orbe.ts · 2 subagentes', true, 'fill', 'Working']);
  });
  it('los puntos se ven si alguna trabaja o espera, y si el ajuste está encendido', () => {
    expect(showDots(snap([ses('a', 'Working')]), cfgDots(true))).toBe(true);
    expect(showDots(snap([ses('a', 'Working')]), cfgDots(false))).toBe(false);
    expect(showDots(snap([ses('a', 'Done'), ses('b', 'Idle')]), cfgDots(true))).toBe(false);
    expect(showDots(snap([ses('a', 'IdleWait')]), cfgDots(true))).toBe(true);
  });
  it('pie del resumen', () => {
    expect(dayLine(null)).toBe('Hoy');
    expect(dayLine({ sessions: [], total_work_ms: 192 * 60_000, total_waited_ms: 14 * 60_000, actions_by_hour: [] })).toBe('Hoy 3 h 12 min · te esperó 14 min');
  });
});

describe('Orbe', () => {
  it('zona más cercana al soltarlo', () => { expect(nearestZone(.1)).toBe('left'); expect(nearestZone(.5)).toBe('center'); expect(nearestZone(.9)).toBe('right'); });
  it('el punto resume a todas las sesiones', () => {
    expect(orbDot(snap([]))).toBe('none');
    expect(orbDot(snap([ses('a', 'Done'), ses('b', 'Working')]))).toBe('fill');
    expect(orbDot(snap([ses('a', 'Working'), ses('b', 'Waiting')]))).toBe('halo');
    expect(orbDot(snap([ses('a', 'Done'), ses('b', 'Idle')]))).toBe('done');
    expect(orbDot(snap([ses('a', 'IdleWait')]))).toBe('calm');
    expect(orbDot({ ...snap([ses('a', 'Working')]), focus: true })).toBe('moon');
  });
  it('línea del modo foco', () => {
    const d = (n: number) => Array.from({ length: n }, () => ({ session_id: 's', project: 'p', kind: 'done', at_ms: 0 }));
    expect(focusLine({ ...snap([]), focus: true, deferred: d(3) })).toBe('En foco · 3 guardados');
    expect(focusLine({ ...snap([]), focus: true, deferred: d(1) })).toBe('En foco · 1 guardado');
  });
});

describe('quitar sesiones', () => {
  it('inactivas: ni trabajando ni con un pendiente', () => {
    const s = snap([ses('a', 'Working'), ses('b', 'Done'), ses('c', 'Idle'), ses('d', 'IdleWait'), ses('s1', 'Waiting')], [pend(1, 's1', 'ls')]);
    expect(inactiveIds(s)).toEqual(['b', 'c', 'd']);
    expect(inactiveIds(snap([]))).toEqual([]);
  });
});
