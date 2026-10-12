// Envoltorios React de los controles de shared/ui.css.
import type { ButtonHTMLAttributes, CSSProperties, MouseEvent, ReactNode } from 'react';
import type { SessionState } from './bridge';
import { ICON, STATE_VAR, type DotKind } from './format';

const cx = (...c: (string | false | undefined | null)[]) => c.filter(Boolean).join(' ');
const vars = (v: Record<string, string | number>) => v as CSSProperties;

export const Icon = ({ name, className, style }: { name: string; className?: string; style?: CSSProperties }) =>
  <span className={cx('ms', className)} style={style} aria-hidden="true">{name}</span>;
export const Keys = ({ k }: { k: string[] }) => <span className="keys">{k.map(x => <kbd key={x} className="kbd">{x}</kbd>)}</span>;

type BtnProps = ButtonHTMLAttributes<HTMLButtonElement> & { pri?: boolean; solo?: boolean; size?: 'md' | 'lg'; keys?: string[]; hold?: number; children: ReactNode };
export function Button({ pri, solo, size, keys, hold, className, style, children, ...rest }: BtnProps) {
  return <button type="button" {...rest} className={cx('btn', pri && 'pri', solo && 'solo', size, className)} style={{ ...style, ...vars({ '--hold': hold ?? 0 }) }}>
    <span>{children}</span>{keys && <Keys k={keys} />}
  </button>;
}
export const Switch = ({ on, label, onChange, small }: { on: boolean; label: string; onChange: (v: boolean) => void; small?: boolean }) =>
  <input type="checkbox" role="switch" className={cx('sw', small && 'sm')} checked={on} aria-label={label} onChange={e => onChange(e.target.checked)} />;
export function Seg<T extends string>({ value, options, onChange, label }: { value: T | null; options: [T, string][]; onChange: (v: T) => void; label: string }) {
  return <span className="seg" role="group" aria-label={label}>{options.map(([v, l]) => <button key={v} type="button" aria-pressed={value === v} onClick={() => onChange(v)}>{l}</button>)}</span>;
}
/** Segmentado de varias opciones a la vez (qué eventos suenan). */
export function MultiSeg({ values, options, onChange, label }: { values: Record<string, boolean>; options: [string, string][]; onChange: (k: string, on: boolean) => void; label: string }) {
  return <span className="seg" role="group" aria-label={label}>{options.map(([k, l]) => <button key={k} type="button" aria-pressed={!!values[k]} onClick={() => onChange(k, !values[k])}>{l}</button>)}</span>;
}
export function Slider({ value, min, max, step = 1, label, format = String, onChange }: { value: number; min: number; max: number; step?: number; label: string; format?: (v: number) => string; onChange: (v: number) => void }) {
  const v = max > min ? ((value - min) / (max - min)) * 100 : 0;
  return <span className="sl"><input type="range" min={min} max={max} step={step} value={value} aria-label={label} style={vars({ '--v': `${v}%` })} onChange={e => onChange(Number(e.target.value))} /><output>{format(value)}</output></span>;
}
export const Swatches = ({ value, options, onChange, label }: { value: string; options: [string, string][]; onChange: (c: string) => void; label: string }) =>
  <span role="radiogroup" aria-label={label} style={{ display: 'flex', gap: 9 }}>
    {options.map(([name, c]) => <button key={c} type="button" role="radio" className="swc" aria-checked={value.toLowerCase() === c.toLowerCase()} aria-label={name} style={{ background: c }} onClick={() => onChange(c)} />)}
  </span>;
/** Relleno que sale desde el punto del clic (desde el centro si fue con teclado). */
function ripple(e: MouseEvent<HTMLElement>) {
  const el = e.currentTarget, r = el.getBoundingClientRect(), kb = e.detail === 0;
  el.style.setProperty('--rx', kb ? '50%' : `${e.clientX - r.left}px`); el.style.setProperty('--ry', kb ? '50%' : `${e.clientY - r.top}px`);
  el.classList.remove('rip'); void el.offsetWidth; el.classList.add('rip');
}
/** `iconOff`: icono cuando está apagado (volumen ↔ silencio); cambia con un giro corto. */
export const Chip = ({ on, icon, iconOff, children, onClick, tone }: { on: boolean; icon?: string; iconOff?: string; children: ReactNode; onClick: () => void; tone?: 'focus' }) => {
  const ic = on || !iconOff ? icon : iconOff;
  return <button type="button" className={cx('chip', tone)} aria-pressed={on} onClick={e => { ripple(e); onClick(); }}>{ic && <Icon key={ic} name={ic} />}{children}</button>;
};
export const Dot = ({ kind }: { kind: DotKind }) => <span className={`dot ${kind}`} aria-hidden="true" />;
export const StateIcon = ({ s, size = 16 }: { s: SessionState; size?: number }) =>
  <Icon name={ICON[s]} className={s === 'Working' ? 'spin' : undefined} style={{ color: STATE_VAR[s], fontSize: size }} />;
