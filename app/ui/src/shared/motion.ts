// Portado de docs/artboards/isla.html: resortes con linear() y morph() de la cápsula.
export const reduceMotion = () => typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
export const T = (ms: number) => ms;
/** Una sola entrada por apertura. En Linux la ventana avisa dos veces (visibilitychange al mostrarse y focus ~100 ms
 *  después, cuando GNOME le da el teclado); sin esto la animación vuelve a empezar desde transparente. Ocultarse rearma. */
export function entryGate(gap = 500) {
  let last = -Infinity;
  return (visible: boolean, t: number) => { if (!visible) { last = -Infinity; return false; } if (t - last < gap) return false; last = t; return true; };
}

export function springPoints(k: number, d: number): number[] {
  let x = 0, v = 0; const dt = 1 / 120, pts = [0];
  for (let i = 0; i < 480; i++) { const a = -k * (x - 1) - d * v; v += a * dt; x += v * dt; if (i % 2) pts.push(+x.toFixed(4)); if (i > 30 && Math.abs(1 - x) < .0015 && Math.abs(v) < .01) break; }
  pts.push(1); return pts;
}
const hasLinear = () => typeof CSS !== 'undefined' && CSS.supports('transition-timing-function', 'linear(0, 1)');
export function spring(k: number, d: number): { e: string; d: number } {
  if (!hasLinear()) return { e: 'cubic-bezier(.32,.72,0,1)', d: 520 };
  const pts = springPoints(k, d);
  return { e: `linear(${pts.join(',')})`, d: Math.round(pts.length * 2 * (1 / 120) * 1000) };
}
export const SP_OPEN = spring(260, 23), SP_CLOSE = spring(340, 38), SP_MOVE = spring(200, 24);

export type Morph = { el: HTMLElement; w: number; h: number; sig: string; op?: number; anim?: Animation | null; ro?: ResizeObserver };
export interface MorphOpts { radius?: (w: number, h: number) => string; empty?: [number, number]; hidden?: boolean; dy?: number; life?: number }

export function morph(m: Morph, sig: string, html: string, o: MorphOpts = {}) {
  if (sig === m.sig) return { w: m.w, h: m.h, changed: false };
  m.sig = sig;
  const reduce = reduceMotion();
  const sh = m.el, old = [...sh.querySelectorAll<HTMLElement>(':scope > .layer')];
  const layer = document.createElement('div'); layer.className = 'layer'; layer.innerHTML = html; sh.appendChild(layer);
  let w = layer.offsetWidth, h = layer.offsetHeight;
  if (!html) [w, h] = o.empty || [0, 0];
  const grow = w * h >= m.w * m.h, sp = grow ? SP_OPEN : SP_CLOSE;
  const r1 = o.radius ? o.radius(w, h) : `${Math.min(h / 2, 16)}px`;
  const fw = sh.offsetWidth || w, fh = sh.offsetHeight || h, r0 = getComputedStyle(sh).borderRadius;
  if (m.anim) { try { m.anim.commitStyles(); } catch {} m.anim.cancel(); }
  sh.style.width = w + 'px'; sh.style.height = h + 'px'; sh.style.borderRadius = r1;
  m.anim = reduce ? null : sh.animate([{ width: fw + 'px', height: fh + 'px', borderRadius: r0 }, { width: w + 'px', height: h + 'px', borderRadius: r1 }], { duration: T(sp.d), easing: sp.e });
  m.w = w; m.h = h;
  // El contenido puede crecer después de medirlo (una tarjeta de preguntas arma su pie al montarse): la cápsula lo sigue
  // en vez de recortarlo, y avisa con "fit" para que la zona de clics se vuelva a medir.
  m.ro?.disconnect(); m.ro = undefined;
  if (html && typeof ResizeObserver === 'function') {
    const ro = new ResizeObserver(() => {
      if (layer !== sh.lastElementChild || m.ro !== ro) return;
      const w2 = layer.offsetWidth, h2 = layer.offsetHeight;
      if (w2 === m.w && h2 === m.h) return;
      m.w = w2; m.h = h2; sh.style.width = w2 + 'px'; sh.style.height = h2 + 'px';
      sh.dispatchEvent(new Event('fit', { bubbles: true }));
    });
    ro.observe(layer); m.ro = ro;
  }
  const op = o.hidden ? 0 : 1;
  if (m.op !== op) { sh.animate([{ opacity: m.op ?? 1 }, { opacity: op }], { duration: T(op ? 180 : 260), fill: 'forwards', easing: 'ease-out' }); m.op = op; }
  for (const x of old) { x.style.pointerEvents = 'none'; x.animate(reduce ? [{ opacity: 1 }, { opacity: 0 }] : [{ opacity: 1, filter: 'blur(0)', transform: 'scale(1)' }, { opacity: 0, filter: 'blur(4px)', transform: 'scale(.97)' }], { duration: T(reduce ? 120 : 150), easing: 'ease-in', fill: 'forwards' }).onfinish = () => x.remove(); }
  if (html) layer.animate(reduce ? [{ opacity: 0 }, { opacity: 1 }] : [{ opacity: 0, filter: 'blur(5px)', transform: `translateY(${o.dy ?? 2}px) scale(.985)` }, { opacity: 1, filter: 'blur(0)', transform: 'none' }], { duration: T(reduce ? 120 : 320), delay: T(old.length && !reduce ? 60 : 0), easing: 'cubic-bezier(.16,1,.3,1)', fill: 'backwards' });
  const tm = layer.querySelector<HTMLElement>('.tm');
  if (tm && o.life) tm.animate([{ transform: 'scaleX(1)' }, { transform: 'scaleX(0)' }], { duration: T(o.life), easing: 'linear', fill: 'forwards' });
  return { w, h, changed: true };
}
