// Ajustes: acabado de los pulsos, paleta pastel (spec §2.3), eventos, formas y textos.
import type { Config } from '../shared/bridge';
export type Finish = 'completo' | 'suave';
type F = Pick<Config['pulses'], 'crisp' | 'corners' | 'adapt' | 'dither'>;
const KEYS = ['crisp', 'corners', 'adapt', 'dither'] as const;
/** Suave: sin línea nítida ni grano; las esquinas y el ajuste en fondos claros siguen (spec §13). */
export const FINISH: Record<Finish, F> = { completo: { crisp: true, corners: true, adapt: true, dither: true }, suave: { crisp: false, corners: true, adapt: true, dither: false } };
/** Qué segmento marcar; `null` = las casillas no coinciden con ninguno ("Personalizado"). */
export const finishOf = (p: F): Finish | null => (Object.keys(FINISH) as Finish[]).find(k => KEYS.every(f => FINISH[k][f] === p[f])) ?? null;
export const PALETTE: [string, string][] = [['Ámbar', '#e3b05c'], ['Verde', '#8fc3a0'], ['Coral', '#e08a7e'], ['Azul', '#8fb0d6'], ['Lavanda', '#b4a6d9'], ['Arena', '#d6c49a'], ['Gris', '#b3bab5']];
export const EVENTS: [string, string][] = [['start', 'Empieza'], ['permission', 'Permiso'], ['done', 'Terminó'], ['idle', 'Te espera'], ['error', 'Error'], ['subagent', 'Subagente']];
export const SHAPES: [string, string][] = [['respira', 'Respira'], ['latido', 'Latido'], ['ola', 'Ola'], ['lateral', 'Lateral'], ['barrido', 'Barrido'], ['origen', 'Origen']];
export const seconds = (ms: number) => `${(ms / 1000).toFixed(1).replace('.', ',')} s`;
export function silentLine(per: Record<string, boolean>): string {
  const off = EVENTS.filter(([k]) => !per[k]).map(([, l]) => l);
  if (!off.length) return 'Suenan todos';
  if (off.length === EVENTS.length) return 'Ninguno suena';
  return `${off.join(', ')} ${off.length === 1 ? 'está' : 'están'} en silencio`;
}
