// En qué sistema corre Pulse, para esconder lo que ahí no existe. El WebKitGTK de Linux dice "Linux" en su user agent;
// el WebView2 de Windows, no.
export const isLinux = typeof navigator !== 'undefined' && /Linux/.test(navigator.userAgent);   // en las pruebas (Node) no hay navigator
/** Razer Chroma: su SDK solo existe en Windows. */
export const hasChroma = !isLinux;
