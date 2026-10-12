import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
export type SessionState = 'Idle' | 'Done' | 'Working' | 'IdleWait' | 'Waiting';
export interface Session { id: string; project: string; slot: number; state: SessionState; last: string; subagents: number; since_ms: number; last_event_ms: number; actions: number; muted_until_ms: number }
export interface Question { question: string; header?: string; options: { label: string; description?: string }[]; multiSelect?: boolean }
export interface Pending { id: number; kind: 'permission' | 'question'; questions?: Question[] | null; session_id: string; project: string; tool: string; command: string; description: string; danger: boolean; at_ms: number; timeout_ms: number; deadline_ms: number; paused: boolean; left_ms: number }
export interface FeedItem { session_id: string; project: string; text: string; at_ms: number }
export interface Deferred { session_id: string; project: string; kind: string; at_ms: number }
export interface Snapshot { sessions: Session[]; pending: Pending[]; feed: FeedItem[]; focus: boolean; focus_auto: boolean; deferred: Deferred[] }
export type FxKind = 'start' | 'permission' | 'done' | 'idle' | 'error' | 'subagent' | 'release' | 'resolved' | 'notice';
export interface Fx { kind: FxKind; session_id: string; project: string; slot: number; text: string; visual: boolean; sound: boolean; monitor: number }
export interface PulseCfg { color: string; shape: string; duration_ms: number; intensity: number; thickness: number; persistent: boolean }
export interface Config {
  island: { mode: 'comando' | 'orbe'; orb_side: 'left' | 'center' | 'right'; dots: boolean; keys: boolean; hold: boolean; countdown: boolean; beam: boolean; ask_timeout_s: number };
  pulses: { enabled: boolean; placement: 'active' | 'all'; crisp: boolean; corners: boolean; adapt: boolean; dither: boolean; events: Record<string, PulseCfg> };
  sounds: { enabled: boolean; volume: number; per_event: Record<string, boolean> };
  focus: { auto: boolean; detect_call: boolean; detect_fullscreen: boolean; schedule: boolean; from: string; to: string; rules: Record<string, 'normal' | 'mudo' | 'nada' | 'despues'> };
  keyboard: { enabled: boolean; session_keys: boolean; base: number };
  mosaico: { order: string[]; hidden: string[] };
  appearance: { theme: 'dark' | 'light' | 'system' };
}
export interface DaySummary { sessions: { project: string; work_ms: number; waited_ms: number; max_wait_ms: number; perms: number; errors: number; subagents: number; actions: number; lanes: [number, number, 'w' | 'p' | 'e'][] }[]; total_work_ms: number; total_waited_ms: number; actions_by_hour: number[] }
export const onState = (f: (s: Snapshot) => void) => listen<Snapshot>('pulse://state', e => f(e.payload));
export const onFx = (f: (x: Fx) => void) => listen<Fx>('pulse://fx', e => f(e.payload));
export const onConfig = (f: (c: Config) => void) => listen<Config>('pulse://config', e => f(e.payload));
export const getSnapshot = () => invoke<Snapshot>('get_snapshot');
export const getConfig = () => invoke<Config>('get_config');
export const setConfig = (config: Config) => invoke('set_config', { config });
export const decide = (id: number, allow: boolean) => invoke('decide', { id, allow });
export const answer = (id: number, answers: Record<string, string>) => invoke('answer', { id, answers });
export const skip = (id: number) => invoke('skip', { id });
export const holdQuestion = (id: number, on: boolean) => invoke('hold_question', { id, on });
export const allowAll = () => invoke('allow_all');
export const setFocus = (on: boolean) => invoke('set_focus', { on });
export const muteSession = (sessionId: string, minutes: number) => invoke('mute_session', { sessionId, minutes });
export const dailySummary = () => invoke<DaySummary>('daily_summary');
export const setIslandHitbox = (rect: { x: number; y: number; w: number; h: number } | null) => invoke('set_island_hitbox', { rect });
export const islandFocus = (on: boolean) => invoke('island_focus', { on });
export const openDashboard = (view?: string) => invoke('open_dashboard', { view: view ?? null });
export const flyoutFit = (height: number) => invoke('flyout_fit', { height });
export const flyoutHide = () => invoke('flyout_hide');
export const openPalette = () => invoke('open_palette');
export const quit = () => invoke('quit');
export const onFlyoutOpen = (f: () => void) => listen('pulse://flyout-open', () => f());
export const onRoute = (f: (view: string) => void) => listen<string>('pulse://route', e => f(e.payload));
export const togglePanel = () => invoke('toggle_panel');
export interface LogItem { at_ms: number; session_id: string; project: string; kind: string; text: string }
export const logToday = () => invoke<LogItem[]>('log_today');
