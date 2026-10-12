# Artboards del rediseño (2026-10-11)

Fuente de los artboards decididos en el canvas de diseño: <https://claude.ai/artifact/S7YbH8JrXWzRKMk4GGTw6P> (fila "Final" y fila "Final: casos límite, avisos y la isla junto a los pulsos"). El spec que los explica está en `docs/specs/2026-10-11-rediseno.md`.

Cada `.dc.html` es el código fuente de un artboard del canvas (formato Design Component). No se abre directo en el navegador: depende del runtime del canvas (`support.js`) y de fuentes subidas al canvas (`/_blob/...`). Para verlos, abre el canvas. Para implementar, sirven como referencia exacta de estructura, medidas y estilos: los estilos están en línea y los tokens de color en `renderVals()` de cada archivo.

| Archivo | Qué muestra |
|---|---|
| `Comando.dc.html` | Isla en modo Comando: puntos de sesión, aviso "terminó", tarjeta de permiso, paleta |
| `Puntos.dc.html` | Comando: los puntos de sesión al pasar el mouse (reposo, transición, resumen) |
| `Orbe.dc.html` | Isla en modo Orbe: reposo, mouse encima, menú de acciones, permiso |
| `Casos.dc.html` | Casos límite de la isla: cola, comando largo, preguntas, salida del foco, Orbe en cada situación, paleta vacía |
| `Avisos.dc.html` | Catálogo de avisos breves de la isla, con icono y duración |
| `Pulso-Permiso.dc.html`, `Pulso-Termino.dc.html`, `Pulso-Error.dc.html` | Pantalla completa: isla, pulsos en los bordes y bandeja juntos |
| `Flyout.dc.html` | Clic derecho del ícono de la bandeja |
| `Bolsillo.dc.html` | Panel de la bandeja |
| `Mosaico.dc.html` | Ventana completa, pantalla Inicio |
| `Ajustes.dc.html` | Pantalla de Ajustes completa |
| `Botones.dc.html` | Comparación de estilos de botón; se eligió el 3 (solo texto) |

Cada artboard trae ajustes de vista (`tema`, `eventos`, `letra`). Los valores elegidos son: tema oscuro, eventos `apagados` y letra Hanken Grotesk.
