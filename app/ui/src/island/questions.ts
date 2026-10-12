// Preguntas de Claude (AskUserQuestion) en la isla: opciones, selección múltiple, "Otra…" y respuesta.
import type { Pending, Question } from '../shared/bridge';
import { COLORS, esc } from '../shared/format';
import { reduceMotion, SP_OPEN } from '../shared/motion';

/** Una sola pregunta de opción única: con un clic basta. */
const instant = (qs: Question[]) => qs.length === 1 && !qs[0].multiSelect;

export function questionCard(p: Pending, countdown: boolean) {
  const qs = p.questions ?? [];
  return `<div class="card qcard" data-qid="${p.id}">
    <div class="ch"><span class="nm">${esc(p.project)}</span><span class="st" style="--c:${COLORS.permission}">te pregunta</span></div>
    ${qs.map((q, i) => `<div class="q" data-q="${i}">
      <div class="qh">${q.header ? `<span class="qtag">${esc(q.header)}</span>` : ''}<b>${esc(q.question)}</b>${q.multiSelect ? '<small>Puedes elegir varias</small>' : ''}</div>
      <div class="qopts">${q.options.map(o => `<button type="button" class="qopt" data-q="${i}" data-label="${esc(o.label)}" aria-pressed="false"><span>${esc(o.label)}</span>${o.description ? `<small>${esc(o.description)}</small>` : ''}</button>`).join('')}<button type="button" class="qopt qother" data-q="${i}" data-other="1" aria-pressed="false"><span>Otra…</span></button></div>
      <input class="qin" data-q="${i}" type="text" placeholder="Escribe tu respuesta" hidden>
    </div>`).join('')}
    <div class="acts"><button class="b-no" type="button" data-act="skip" data-id="${p.id}"><span>Responder en la terminal</span></button><button class="b-yes" type="button" data-act="send" data-id="${p.id}" disabled${instant(qs) ? ' hidden' : ''}><span>Enviar</span></button></div>
    <div class="qtime"${countdown ? '' : ' hidden'}><span class="qleft"></span><i class="qbar"><b></b></i></div></div>`;
}

/** Respuestas elegidas en la tarjeta (pregunta → etiqueta; varias se unen con coma). `null` si falta alguna. */
export function collect(card: HTMLElement, qs: Question[]): Record<string, string> | null {
  const out: Record<string, string> = {};
  for (let i = 0; i < qs.length; i++) {
    const picked = [...card.querySelectorAll<HTMLElement>(`.qopt[data-q="${i}"][aria-pressed="true"]`)].map(b => b.dataset.other ? card.querySelector<HTMLInputElement>(`.qin[data-q="${i}"]`)!.value.trim() : b.dataset.label!).filter(Boolean);
    if (!picked.length) return null;
    out[qs[i].question] = picked.join(', ');
  }
  return out;
}

/** Conecta las tarjetas de preguntas dentro de `root` (delegación: sobrevive a los cambios de capa del morph). */
export function bindQuestions(root: HTMLElement, find: (id: number) => Pending | undefined, ctx: { answer(id: number, a: Record<string, string>): void; skip(id: number): void; hold(id: number, on: boolean): void; islandFocus(on: boolean): void; hitbox(el: HTMLElement | null): void }) {
  // La cápsula tiene alto fijo (morph); si la tarjeta cambia de tamaño (aparece "Otra…"), se ajusta y se actualiza la zona de clics.
  const fit = (card: HTMLElement) => {
    const sh = card.closest<HTMLElement>('.shell'), layer = card.closest<HTMLElement>('.layer'); if (!sh || !layer) return;
    const from = sh.offsetHeight, to = layer.offsetHeight; if (from === to) return;
    sh.style.height = to + 'px';
    if (!reduceMotion()) sh.animate([{ height: from + 'px' }, { height: to + 'px' }], { duration: SP_OPEN.d, easing: SP_OPEN.e });
    ctx.hitbox(sh);
  };
  const cardOf = (el: Element) => el.closest<HTMLElement>('.qcard');
  const refresh = (card: HTMLElement, p: Pending) => { const send = card.querySelector<HTMLButtonElement>('[data-act="send"]'); if (send) send.disabled = !collect(card, p.questions ?? []); };
  const submit = (card: HTMLElement, p: Pending) => { const a = collect(card, p.questions ?? []); if (a) { ctx.islandFocus(false); ctx.answer(p.id, a); } };
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
      if (instant(qs)) card.querySelector<HTMLElement>('[data-act="send"]')!.hidden = !other;
      if (wasHidden !== input.hidden) fit(card);
      if (other && opt.dataset.other) { ctx.islandFocus(true); requestAnimationFrame(() => input.focus()); }
      if (!opt.dataset.other && instant(qs)) { submit(card, p); return; }
      refresh(card, p);
      return;
    }
    const act = t.closest<HTMLElement>('[data-act]')?.dataset.act;
    if (act === 'send') submit(card, p);
    if (act === 'skip') { ctx.islandFocus(false); ctx.skip(p.id); }
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
    const left = Math.max(0, p.paused ? p.left_ms : p.deadline_ms - Date.now());
    card.querySelector('.qleft')!.textContent = p.paused ? 'En pausa mientras respondes' : `Pasa a la terminal en ${Math.ceil(left / 1000)} s`;
    card.querySelector<HTMLElement>('.qbar b')!.style.transform = `scaleX(${Math.min(1, left / p.timeout_ms)})`;
    card.classList.toggle('paused', p.paused);
  }), 250);
  root.addEventListener('input', e => { const card = cardOf(e.target as Element); const p = card && find(Number(card.dataset.qid)); if (card && p) refresh(card, p); });
  root.addEventListener('keydown', e => {
    const t = e.target as HTMLElement; if (!t.matches('.qin')) return;
    const card = cardOf(t), p = card && find(Number(card.dataset.qid)); if (!card || !p) return;
    if (e.key === 'Enter') { e.preventDefault(); submit(card, p); }
    if (e.key === 'Escape') { t.blur(); ctx.islandFocus(false); }
  });
}
