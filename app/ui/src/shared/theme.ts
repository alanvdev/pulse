// Tema de las ventanas (Bolsillo, flyout, Mosaico): la versión oscura o clara de Grafito. La isla no lo usa.
export type ThemePref = 'dark' | 'light' | 'system';
/** "system" sigue el modo de Windows (WebView2 lo expone como prefers-color-scheme). */
export const resolveTheme = (pref: ThemePref | undefined, systemLight: boolean): 'dark' | 'light' =>
  pref === 'light' || (pref === 'system' && systemLight) ? 'light' : 'dark';
const mq = () => matchMedia('(prefers-color-scheme: light)');
const KEY = 'pulse.theme';
export function applyTheme(pref: ThemePref | undefined, root: HTMLElement = document.documentElement) {
  root.dataset.theme = resolveTheme(pref, mq().matches);
  try { if (pref) localStorage.setItem(KEY, pref); } catch { /* sin almacenamiento: no pasa nada */ }
}
/** Antes de que llegue la configuración: el último tema que vio esta ventana, para no abrir oscura y luego cambiar. */
export function applyRemembered() {
  let pref: string | null = null;
  try { pref = localStorage.getItem(KEY); } catch { /* sin almacenamiento */ }
  applyTheme(pref === 'light' || pref === 'system' || pref === 'dark' ? pref : undefined);
}
/** Aplica el tema y lo vuelve a aplicar si cambia el modo de Windows. Devuelve cómo dejar de seguirlo. */
export function followTheme(pref: ThemePref | undefined): () => void {
  applyTheme(pref);
  const m = mq(), f = () => applyTheme(pref);
  m.addEventListener('change', f);
  return () => m.removeEventListener('change', f);
}
