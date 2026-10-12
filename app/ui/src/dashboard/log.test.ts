import { describe, it, expect } from 'vitest';
import type { LogItem } from '../shared/bridge';
import { filterLog, kindGroup, projectsOf } from './log';
const it_ = (project: string, kind: string): LogItem => ({ at_ms: 0, session_id: 's', project, kind, text: '' });
const items = [it_('pulse', 'permission'), it_('api', 'done'), it_('pulse', 'error'), it_('api', 'tool'), it_('pulse', 'start')];
describe('bitácora', () => {
  it('agrupa los tipos', () => { expect(kindGroup('resolve')).toBe('permisos'); expect(kindGroup('subagent-stop')).toBe('herramientas'); expect(kindGroup('rara')).toBe('herramientas'); });
  it('filtra por proyecto y por tipo', () => {
    expect(filterLog(items, null, 'todo')).toHaveLength(5);
    expect(filterLog(items, 'pulse', 'todo').map(i => i.kind)).toEqual(['permission', 'error', 'start']);
    expect(filterLog(items, null, 'errores').map(i => i.project)).toEqual(['pulse']);
    expect(filterLog(items, 'api', 'termino')).toHaveLength(1);
  });
  it('proyectos sin repetir, en orden', () => expect(projectsOf(items)).toEqual(['api', 'pulse']));
});
