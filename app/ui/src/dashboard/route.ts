// Rutas de Mosaico en el hash: "inicio", "sesiones/<id>", "hoy", "bitacora", "ajustes/<sección>".
export type Screen = 'inicio' | 'sesiones' | 'hoy' | 'bitacora' | 'ajustes';
/** `n` cambia en cada navegación: pedir la misma ruta otra vez vuelve a abrir y mostrar lo pedido. */
export interface Route { screen: Screen; arg?: string; n?: number }
const SCREENS: Screen[] = ['inicio', 'sesiones', 'hoy', 'bitacora', 'ajustes'];
export function parseRoute(h: string | null | undefined): Route {
  const [s, ...rest] = (h ?? '').replace(/^#/, '').split('/');
  const screen = (SCREENS as string[]).includes(s) ? s as Screen : 'inicio', arg = rest.join('/');
  return screen !== 'inicio' && arg ? { screen, arg } : { screen };
}
export const formatRoute = (r: Route) => r.arg ? `${r.screen}/${r.arg}` : r.screen;
