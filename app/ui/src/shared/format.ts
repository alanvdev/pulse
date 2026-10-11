import type { SessionState } from './bridge';
export const COLORS = { start: '#e6eeff', permission: '#ffb02e', done: '#4fe0a0', idle: '#b6c0d4', error: '#ff5a52', subagent: '#5cc8ff', focus: '#b9a2ff', calm: '#69717c' } as const;
export const LBL: Record<SessionState, string> = { Idle: 'en calma', Done: 'terminó', Working: 'trabajando', IdleWait: 'te espera', Waiting: 'pide permiso' };
export const ICON: Record<SessionState, string> = { Idle: 'bedtime', Done: 'check', Working: 'progress_activity', IdleWait: 'hourglass', Waiting: 'front_hand' };
export const stateColor = (s: SessionState) => ({ Idle: COLORS.calm, Done: COLORS.done, Working: '#e8eefc', IdleWait: COLORS.idle, Waiting: COLORS.permission })[s];
export function ago(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  if (s < 60) return `${s} s`;
  if (s < 3600) return `${Math.floor(s / 60)} min ${String(s % 60).padStart(2, '0')} s`;
  return `${Math.floor(s / 3600)} h ${String(Math.floor(s / 60) % 60).padStart(2, '0')} min`;
}
export const esc = (s: unknown) => String(s ?? '').replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]!));
