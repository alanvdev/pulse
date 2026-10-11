// Preguntas de Claude (AskUserQuestion) en la isla: opciones, selección múltiple, "Otra…" y respuesta.
import type { Pending, Question } from '../shared/bridge';
import { COLORS, esc } from '../shared/format';

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
    ${countdown ? '<i class="cd"></i>' : ''}</div>`;
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
export function bindQuestions(root: HTMLElement, find: (id: number) => Pending | undefined, ctx: { answer(id: number, a: Record<string, string>): void; skip(id: number): void; islandFocus(on: boolean): void }) {
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
      input.hidden = !other;
      if (instant(qs)) card.querySelector<HTMLElement>('[data-act="send"]')!.hidden = !other;
      if (other && opt.dataset.other) { ctx.islandFocus(true); requestAnimationFrame(() => input.focus()); }
      if (!opt.dataset.other && instant(qs)) { submit(card, p); return; }
      refresh(card, p);
      return;
    }
    const act = t.closest<HTMLElement>('[data-act]')?.dataset.act;
    if (act === 'send') submit(card, p);
    if (act === 'skip') { ctx.islandFocus(false); ctx.skip(p.id); }
  });
  root.addEventListener('input', e => { const card = cardOf(e.target as Element); const p = card && find(Number(card.dataset.qid)); if (card && p) refresh(card, p); });
  root.addEventListener('keydown', e => {
    const t = e.target as HTMLElement; if (!t.matches('.qin')) return;
    const card = cardOf(t), p = card && find(Number(card.dataset.qid)); if (!card || !p) return;
    if (e.key === 'Enter') { e.preventDefault(); submit(card, p); }
    if (e.key === 'Escape') { t.blur(); ctx.islandFocus(false); }
  });
}
