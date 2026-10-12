// Bitácora: grupos de tipos de evento, filtros e iconos.
import type { LogItem } from '../shared/bridge';
export type LogKind = 'todo' | 'permisos' | 'termino' | 'errores' | 'herramientas' | 'prompts';
export const LOG_KINDS: [LogKind, string][] = [['todo', 'Todo'], ['permisos', 'Permisos'], ['termino', 'Terminó'], ['errores', 'Errores'], ['herramientas', 'Herramientas'], ['prompts', 'Prompts']];
const GROUP: Record<string, LogKind> = {
  permission: 'permisos', question: 'permisos', resolve: 'permisos', done: 'termino', idle: 'termino', plan: 'termino', error: 'errores',
  tool: 'herramientas', 'subagent-start': 'herramientas', 'subagent-stop': 'herramientas', start: 'prompts', 'session-start': 'prompts', 'session-end': 'prompts',
};
export const kindGroup = (k: string): LogKind => GROUP[k] ?? 'herramientas';
export const filterLog = (items: LogItem[], project: string | null, kind: LogKind) => items.filter(i => (!project || i.project === project) && (kind === 'todo' || kindGroup(i.kind) === kind));
export const projectsOf = (items: LogItem[]) => [...new Set(items.map(i => i.project))].sort((a, b) => a.localeCompare(b, 'es'));
/** Icono y color de cada tipo (el icono lleva el significado; el color lo confirma). */
export const LOG_ICON: Record<string, [string, string]> = {
  start: ['chat', 'var(--tx2)'], tool: ['build', 'var(--tx3)'], permission: ['front_hand', 'var(--perm)'], question: ['help', 'var(--perm)'], resolve: ['gavel', 'var(--tx2)'],
  done: ['check', 'var(--done)'], idle: ['hourglass', 'var(--tx2)'], plan: ['assignment', 'var(--tx2)'], error: ['error', 'var(--err)'],
  'subagent-start': ['account_tree', 'var(--sub)'], 'subagent-stop': ['account_tree', 'var(--sub)'], 'session-start': ['login', 'var(--tx3)'], 'session-end': ['logout', 'var(--tx3)'],
};
