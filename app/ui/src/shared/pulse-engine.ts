import type { PulseCfg } from './bridge';
export type Geo = { x: number; y: number; side: 'left' | 'right' };
type Side = 'left' | 'right' | 'top' | 'bottom';
const SIDES: Side[] = ['left', 'right', 'top', 'bottom'];
const DIR: Record<Side, string> = { left: 'to right', right: 'to left', top: 'to bottom', bottom: 'to top' };
const rgb = (h: string) => { const n = parseInt(h.slice(1), 16); return [n >> 16 & 255, n >> 8 & 255, n & 255]; };
const rgba = (h: string, a: number) => `rgba(${rgb(h).join(',')},${a})`;
const darken = (h: string, k: number) => '#' + rgb(h).map(v => Math.round(v * (1 - k)).toString(16).padStart(2, '0')).join('');
export const slotX = (slot: number) => [18, 50, 82][slot % 3];

export const edgeMask = (t: number, side?: Side) => (side ? [side] : SIDES).map(s => `linear-gradient(${DIR[s]}, #000 0, rgba(0,0,0,.5) ${t * .3}px, rgba(0,0,0,.14) ${t * .65}px, transparent ${t}px)`).join(',');
export const lineMask = (side?: Side) => (side ? [side] : SIDES).map(s => `linear-gradient(${DIR[s]}, #000 0, #000 1.5px, rgba(0,0,0,.35) 3px, transparent 6px)`).join(',');
export const cornerMask = (t: number) => ['0 0', '100% 0', '0 100%', '100% 100%'].map(p => `radial-gradient(${t * 1.5}px ${t * 1.5}px at ${p}, #000, rgba(0,0,0,.3) 45%, transparent 70%)`).join(',');
const setMask = (e: HTMLElement, m: string) => { e.style.maskImage = m; (e.style as any).webkitMaskImage = m; e.style.maskComposite = 'add'; (e.style as any).webkitMaskComposite = 'source-over'; };

export function paintFor(shape: string, c: string, g: Geo): string {
  if (shape === 'ola') return `conic-gradient(from var(--a) at 50% 50%, transparent 0deg, transparent 230deg, ${rgba(c, .35)} 300deg, ${c} 354deg, transparent 360deg)`;
  if (shape === 'barrido') return `linear-gradient(to top, transparent calc(var(--r) - 38%), ${c} var(--r), transparent calc(var(--r) + 38%))`;
  if (shape === 'origen') return `radial-gradient(var(--r) var(--r) at ${g.x}% ${g.y}%, transparent 0%, ${rgba(c, .25)} 50%, ${c} 84%, transparent 100%)`;
  return c;
}
export function keyframesFor(shape: string, I: number, g: Geo): Keyframe[] {
  const E = 'cubic-bezier(.37,0,.63,1)';
  switch (shape) {
    case 'latido': return [{ opacity: 0, easing: 'cubic-bezier(.2,.8,.2,1)' }, { opacity: I, offset: .1, easing: 'cubic-bezier(.5,0,.5,1)' }, { opacity: .12 * I, offset: .3, easing: 'cubic-bezier(.2,.8,.2,1)' }, { opacity: .88 * I, offset: .42, easing: 'cubic-bezier(.4,0,.2,1)' }, { opacity: 0 }];
    case 'ola': { const a0 = Math.atan2(g.x - 50, -(g.y - 50) * .625) * 180 / Math.PI; return [{ '--a': `${a0}deg`, opacity: 0 }, { '--a': `${a0 + 54}deg`, opacity: I, offset: .15 }, { '--a': `${a0 + 270}deg`, opacity: I, offset: .75 }, { '--a': `${a0 + 380}deg`, opacity: 0 }]; }
    case 'barrido': return [{ '--r': '-40%', opacity: I, easing: 'cubic-bezier(.45,0,.2,1)' }, { '--r': '140%', opacity: I }];
    case 'lateral': return [{ opacity: 0 }, { opacity: I, offset: .1 }, { opacity: .2 * I, offset: .25 }, { opacity: I, offset: .4 }, { opacity: 0, easing: E }];
    case 'origen': return [{ '--r': '0%', opacity: I, easing: 'cubic-bezier(.22,1,.36,1)' }, { '--r': '120%', opacity: I, offset: .55 }, { '--r': '230%', opacity: 0 }];
    default: return [{ opacity: 0, easing: E }, { opacity: I, offset: .38, easing: E }, { opacity: 0 }];
  }
}
/** Ruido en el canal alfa: sirve de máscara para que el grano tome el color de la luz. */
let grain = '';
function grainURL() {
  if (grain) return grain;
  const c = document.createElement('canvas'); c.width = c.height = 96; const x = c.getContext('2d')!; const d = x.createImageData(96, 96);
  for (let i = 0; i < d.data.length; i += 4) { d.data[i] = d.data[i + 1] = d.data[i + 2] = 255; d.data[i + 3] = Math.random() * 255; }
  x.putImageData(d, 0, 0); return (grain = c.toDataURL());
}
export interface EngineOpts { crisp: boolean; corners: boolean; adapt: boolean; dither: boolean; lightBackground: boolean; reduce: boolean }
export function pulse(layer: HTMLElement, cfg: PulseCfg, g: Geo, o: EngineOpts) {
  const t = Math.max(4, cfg.thickness), I = cfg.intensity / 100, D = cfg.duration_ms;
  const shape = o.reduce ? 'respira' : cfg.shape, side = shape === 'lateral' ? g.side : undefined;
  let c = cfg.color;
  const wrap = document.createElement('div'); wrap.style.cssText = 'position:absolute;inset:0;pointer-events:none';
  if (o.adapt && o.lightBackground) { c = darken(c, .18); wrap.style.mixBlendMode = 'multiply'; }
  const paint = paintFor(shape, c, g);
  const add = (mask: string, op: number) => { const e = document.createElement('div'); e.style.cssText = `position:absolute;inset:0;opacity:${op}`; e.style.background = paint; setMask(e, mask); wrap.appendChild(e); };
  add(edgeMask(side ? t * 1.3 : t, side), .82);
  if (o.crisp) add(lineMask(side), 1);
  if (o.corners && !side) add(cornerMask(t), .45);
  // Grano tenue del mismo color de la luz, solo donde hay luz (en una ventana transparente un "overlay" gris no tiene con qué mezclarse).
  if (o.dither) { const n = document.createElement('div'); n.style.cssText = 'position:absolute;inset:0;opacity:.28'; setMask(n, edgeMask(t, side)); const g = document.createElement('div'); g.style.cssText = 'position:absolute;inset:0'; g.style.background = paint; const gm = `url(${grainURL()})`; g.style.maskImage = gm; (g.style as any).webkitMaskImage = gm; n.appendChild(g); wrap.appendChild(n); }
  layer.appendChild(wrap);
  const a = wrap.animate(keyframesFor(shape, I, g), { duration: D, easing: 'linear' });
  const kill = () => wrap.remove(); a.onfinish = kill; setTimeout(kill, D + 400);
}
const residuals = new Map<string, HTMLElement>();
export function residualOn(key: string, layer: HTMLElement, cfg: PulseCfg, reduce: boolean) {
  residualOff(key, true);
  const e = document.createElement('div'); e.style.cssText = 'position:absolute;inset:0;opacity:0;pointer-events:none'; e.style.background = cfg.color; setMask(e, edgeMask(cfg.thickness * .8));
  layer.appendChild(e);
  const lo = cfg.intensity / 100 * .14, hi = cfg.intensity / 100 * .36;
  if (reduce) e.style.opacity = String(hi); else e.animate([{ opacity: lo }, { opacity: hi }], { duration: 2400, iterations: Infinity, direction: 'alternate', easing: 'ease-in-out', delay: cfg.duration_ms * .6, fill: 'backwards' });
  residuals.set(key, e);
}
export function residualOff(key: string, instant = false) {
  const e = residuals.get(key); if (!e) return; residuals.delete(key);
  if (instant) return e.remove();
  e.getAnimations().forEach(a => a.pause());
  e.animate([{ opacity: getComputedStyle(e).opacity }, { opacity: 0 }], { duration: 450 }).onfinish = () => e.remove();
}
export const residualKeys = () => [...residuals.keys()];
