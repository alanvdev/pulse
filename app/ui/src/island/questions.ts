// Preguntas de Claude (AskUserQuestion) en la isla: una pregunta a la vez, opciones, selección múltiple, "Otra…" y respuesta.
import type { Pending, Question } from '../shared/bridge';
import { esc } from '../shared/format';
import { reduceMotion, SP_OPEN } from '../shared/motion';
import { digitIndex, qHint } from './model';
import { keys } from './shared';

/** Lo elegido en una pregunta: opciones marcadas, si "Otra…" está marcada y lo escrito. */
export type Pick = { labels: string[]; other: boolean; text: string };
type Action = 'option' | 'other' | 'next' | 'back';

const answered = (p: Pick | undefined) => !!p && (p.labels.length > 0 || (p.other && p.text.trim() !== ''));

/** Respuestas para Claude (pregunta → etiqueta; varias se unen con coma). `null` si falta alguna. */
export function buildAnswers(qs: Question[], picks: Pick[]): Record<string, string> | null {
  const out: Record<string, string> = {};
  for (let i = 0; i < qs.length; i++) {
    const p = picks[i]; if (!answered(p)) return null;
    out[qs[i].question] = [...p.labels, ...(p.other && p.text.trim() ? [p.text.trim()] : [])].join(', ');
  }
  return out;
}

/** Qué pasa tras una acción en el paso `step`: a qué paso ir y si ya se envía. */
export function flow(qs: Question[], step: number, picks: Pick[], action: Action): { step: number; send: boolean } {
  const last = qs.length - 1, single = !qs[step]?.multiSelect;
  if (action === 'back') return { step: Math.max(0, step - 1), send: false };
  if (action === 'next') {
    if (!answered(picks[step])) return { step, send: false };
    return step < last ? { step: step + 1, send: false } : { step, send: buildAnswers(qs, picks) !== null };
  }
  if (action === 'option' && single && answered(picks[step])) {
    if (qs.length === 1) return { step, send: true };          // una sola pregunta de opción única: con un clic basta
    if (step < last) return { step: step + 1, send: false };   // varias: avanza sola; la última espera a Enviar
  }
  return { step, send: false };
}

export function questionCard(p: Pending, countdown: boolean) {
  const qs = p.questions ?? [], n = qs.length, instant = n === 1 && !qs[0].multiSelect;
  const mark = (multi?: boolean) => `<span class="${multi ? 'chk' : 'rad'}"></span>`;
  return `<div class="card qcard" style="width:480px" data-qid="${p.id}" data-step="0">
    <div class="card-b">
      <div class="head"><span class="dot halo"></span><b>${esc(p.project)}</b>te pregunta</div>
      ${qs.map((q, i) => `<div class="q" data-q="${i}"${i ? ' hidden' : ''}>
        <div class="qh">${q.header ? `<span class="tag">${esc(q.header)}</span>` : ''}${q.multiSelect ? '<small>Puedes elegir varias</small>' : ''}${n > 1 ? `<small class="qstep mono">${i + 1} de ${n}</small>` : ''}</div>
        <b class="qq">${esc(q.question)}</b>
        <div class="qopts">${q.options.map(o => `<button type="button" class="qopt" data-q="${i}" data-label="${esc(o.label)}" aria-pressed="false">${mark(q.multiSelect)}<span>${esc(o.label)}${o.description ? `<small>${esc(o.description)}</small>` : ''}</span></button>`).join('')}<button type="button" class="qopt qother" data-q="${i}" data-other="1" aria-pressed="false">${mark(q.multiSelect)}<span>Otra…</span></button></div>
        <input class="qin" data-q="${i}" type="text" placeholder="Escribe tu respuesta" aria-label="Tu respuesta" hidden>
      </div>`).join('')}
      <div class="qtime"${countdown ? '' : ' hidden'}><span class="qleft"></span><i class="qbar"><b></b></i></div>
    </div>
    <div class="bar"><span class="qhint"></span>
      <button class="btn" type="button" data-act="skip" style="margin-left:auto"><span>En la terminal</span>${keys(['Esc'])}</button>
      <button class="btn" type="button" data-act="back" hidden><span>Atrás</span></button>
      <button class="btn pri" type="button" data-act="next" disabled${instant ? ' hidden' : ''}><span>Siguiente</span>${keys(['Ctrl', '↵'])}</button>
    </div></div>`;
}

type Ctx = { answer(id: number, a: Record<string, string>): void; skip(id: number): void; hold(id: number, on: boolean): void; islandFocus(on: boolean): void; hitbox(el: HTMLElement | null): void };

/** Conecta las tarjetas de preguntas dentro de `root` (delegación: sobrevive a los cambios de capa del morph). */
export function bindQuestions(root: HTMLElement, find: (id: number) => Pending | undefined, ctx: Ctx) {
  const cardOf = (el: Element) => el.closest<HTMLElement>('.qcard');
  const stepOf = (card: HTMLElement) => Number(card.dataset.step ?? 0);
  const picksOf = (card: HTMLElement, qs: Question[]): Pick[] => qs.map((_, i) => ({
    labels: [...card.querySelectorAll<HTMLElement>(`.qopt[data-q="${i}"][aria-pressed="true"]:not(.qother)`)].map(b => b.dataset.label!),
    other: !!card.querySelector(`.qother[data-q="${i}"][aria-pressed="true"]`),
    text: card.querySelector<HTMLInputElement>(`.qin[data-q="${i}"]`)?.value ?? '',
  }));
  // La cápsula tiene alto fijo (morph); si la tarjeta cambia de tamaño, se ajusta y se actualiza la zona de clics.
  const fit = (card: HTMLElement) => {
    const sh = card.closest<HTMLElement>('.shell'), layer = card.closest<HTMLElement>('.layer'); if (!sh || !layer) return;
    const from = sh.offsetHeight, to = layer.offsetHeight; if (from === to) return;
    sh.style.height = to + 'px';
    if (!reduceMotion()) sh.animate([{ height: from + 'px' }, { height: to + 'px' }], { duration: SP_OPEN.d, easing: SP_OPEN.e });
    ctx.hitbox(sh);
  };
  // Botones Atrás / Siguiente / Enviar según el paso y lo elegido.
  const refresh = (card: HTMLElement, p: Pending) => {
    const qs = p.questions ?? [], step = stepOf(card), picks = picksOf(card, qs), last = step === qs.length - 1;
    const next = card.querySelector<HTMLButtonElement>('[data-act="next"]')!;
    next.querySelector('span')!.textContent = last ? 'Enviar' : 'Siguiente';
    next.disabled = last ? buildAnswers(qs, picks) === null : !answered(picks[step]);
    card.querySelector<HTMLElement>('[data-act="back"]')!.hidden = step === 0;
    const q = qs[step], chosen = picks[step].labels.length + (picks[step].other ? 1 : 0);
    card.querySelector<HTMLElement>('.qhint')!.textContent = qHint(!!q?.multiSelect, (q?.options.length ?? 0) + 1, chosen);
  };
  const go = (card: HTMLElement, p: Pending, step: number) => {
    if (step === stepOf(card)) { refresh(card, p); return; }
    card.dataset.step = String(step);
    card.querySelectorAll<HTMLElement>('.q').forEach(q => (q.hidden = Number(q.dataset.q) !== step));
    refresh(card, p); fit(card);
    const input = card.querySelector<HTMLInputElement>(`.qin[data-q="${step}"]`);
    if (input && !input.hidden) requestAnimationFrame(() => input.focus());
  };
  const act = (card: HTMLElement, p: Pending, action: Action, delay = 0) => {
    const qs = p.questions ?? [], r = flow(qs, stepOf(card), picksOf(card, qs), action);
    const run = () => {
      if (r.send) { const a = buildAnswers(qs, picksOf(card, qs)); if (a) { ctx.islandFocus(false); ctx.answer(p.id, a); } }
      else go(card, p, r.step);
    };
    if (delay && !reduceMotion()) setTimeout(run, delay); else run();
  };
  root.addEventListener('click', e => {
    const t = e.target as HTMLElement, card = cardOf(t); if (!card) return;
    const p = find(Number(card.dataset.qid)); if (!p) return;
    const qs = p.questions ?? [];
    const opt = t.closest<HTMLElement>('.qopt');
    if (opt) {
      const i = Number(opt.dataset.q), multi = !!qs[i]?.multiSelect, on = opt.getAttribute('aria-pressed') !== 'true';
      if (!multi) card.querySelectorAll(`.qopt[data-q="${i}"]`).forEach(b => b.setAttribute('aria-pressed', 'false'));
      opt.setAttribute('aria-pressed', String(multi ? on : true));
      const input = card.querySelector<HTMLInputElement>(`.qin[data-q="${i}"]`)!;
      const other = !!card.querySelector(`.qother[data-q="${i}"][aria-pressed="true"]`);
      const wasHidden = input.hidden;
      input.hidden = !other;
      // en una sola pregunta de opción única no hay botón; con "Otra…" sí hace falta para mandar lo escrito
      if (qs.length === 1 && !multi) card.querySelector<HTMLElement>('[data-act="next"]')!.hidden = !other;
      if (wasHidden !== input.hidden) fit(card);
      if (opt.dataset.other && other) { ctx.islandFocus(true); requestAnimationFrame(() => input.focus()); }
      refresh(card, p);
      act(card, p, opt.dataset.other ? 'other' : 'option', 160);   // una pausa breve para ver la opción marcada
      return;
    }
    const a = t.closest<HTMLElement>('[data-act]')?.dataset.act;
    if (a === 'next') act(card, p, 'next');
    if (a === 'back') act(card, p, 'back');
    if (a === 'skip') { ctx.islandFocus(false); ctx.skip(p.id); }
  });
  root.addEventListener('input', e => { const card = cardOf(e.target as Element); const p = card && find(Number(card.dataset.qid)); if (card && p) refresh(card, p); });
  root.addEventListener('keydown', e => {
    const t = e.target as HTMLElement, card = cardOf(t), p = card && find(Number(card.dataset.qid)); if (!card || !p) return;
    if (t.matches('.qin')) {
      if (e.key === 'Enter') { e.preventDefault(); act(card, p, 'next'); }
      if (e.key === 'Escape') { e.preventDefault(); t.blur(); ctx.islandFocus(false); }
      return;
    }
    if (e.key === 'Escape') { e.preventDefault(); ctx.islandFocus(false); ctx.skip(p.id); return; }
    if (e.key === 'Enter' && e.ctrlKey) { e.preventDefault(); act(card, p, 'next'); return; }
    const opts = [...card.querySelectorAll<HTMLElement>(`.qopt[data-q="${stepOf(card)}"]`)], i = digitIndex(e.key, opts.length);
    if (i >= 0) { e.preventDefault(); opts[i].click(); }
  });
  // La cuenta regresiva se pausa mientras tienes el mouse sobre la tarjeta o escribes en ella.
  // Se sigue el mouse con mouseover/mouseout (el estado :hover llega tarde respecto a estos eventos).
  let held: number | null = null, hovered: HTMLElement | null = null;
  const sync = () => {
    const focused = cardOf(document.activeElement ?? document.body);
    const card = (hovered?.isConnected ? hovered : null) ?? (focused?.isConnected ? focused : null);
    const want = card ? Number(card.dataset.qid) : null;
    if (want === held) return;
    if (held !== null) ctx.hold(held, false);
    if (want !== null) ctx.hold(want, true);
    held = want;
  };
  root.addEventListener('mouseover', e => { hovered = cardOf(e.target as Element); sync(); });
  root.addEventListener('mouseout', e => { const to = e.relatedTarget as Element | null; if (!to || !hovered?.contains(to)) { hovered = to ? cardOf(to) : null; sync(); } });
  root.addEventListener('focusin', () => sync());
  root.addEventListener('focusout', () => setTimeout(sync, 0));
  // Texto y barra de la cuenta regresiva.
  setInterval(() => root.querySelectorAll<HTMLElement>('.qcard').forEach(card => {
    const p = find(Number(card.dataset.qid)); if (!p) return;
    if (!card.dataset.ready) { card.dataset.ready = '1'; refresh(card, p); }
    const left = Math.max(0, p.paused ? p.left_ms : p.deadline_ms - Date.now());
    card.querySelector('.qleft')!.textContent = p.paused ? 'En pausa mientras respondes' : `Pasa a la terminal en ${Math.ceil(left / 1000)} s`;
    card.querySelector<HTMLElement>('.qbar b')!.style.transform = `scaleX(${Math.min(1, left / p.timeout_ms)})`;
    card.classList.toggle('paused', p.paused);
  }), 250);
}
