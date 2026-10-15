// Registra o quita los hooks de Pulse en un settings.json de Claude Code y los atajos globales en GNOME.
// Lo usan scripts/install.sh y scripts/uninstall.sh:
//   node scripts/linux-setup.mjs hooks-add <settings.json> <ruta de pulse>
//   node scripts/linux-setup.mjs hooks-remove <settings.json>
//   node scripts/linux-setup.mjs keys-add <ruta de pulse>
//   node scripts/linux-setup.mjs keys-remove
import { existsSync, readFileSync, writeFileSync, copyFileSync, mkdirSync, renameSync, realpathSync, constants } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { homedir } from 'node:os';
import { execFileSync } from 'node:child_process';

const [cmd, ...args] = process.argv.slice(2);
// Entradas que llaman a Pulse (las de Linux y las de Windows): son las únicas que se reemplazan o se quitan.
const OURS = /[\\/]pulse(\.exe)?"? (emit|ask)\b|sounds\/play\.ps1/;
// Se quitan hook por hook, no el grupo entero: el menú /hooks de Claude Code mete en el mismo grupo los hooks
// con el mismo matcher, y ahí puede haber uno tuyo junto al de Pulse. Un grupo solo se va si queda vacío.
const stripOurs = list => (Array.isArray(list) ? list : []).map(e => ({ ...e, hooks: (e.hooks ?? []).filter(h => !OURS.test(h.command ?? '')) })).filter(e => e.hooks.length);
const countOurs = hooks => Object.values(hooks ?? {}).flat().flatMap(e => e?.hooks ?? []).filter(h => OURS.test(h.command ?? '')).length;

// Hora local, como en install.ps1: yyyyMMdd-HHmmss.
const stamp = () => { const d = new Date(), p = n => String(n).padStart(2, '0'); return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(d.getHours())}${p(d.getMinutes())}${p(d.getSeconds())}`; };

function readSettings(path) {
  if (!existsSync(path)) { mkdirSync(dirname(path), { recursive: true }); return {}; }
  const raw = readFileSync(path, 'utf8');
  return raw.trim() ? JSON.parse(raw) : {};
}
function backup(path) {
  if (!existsSync(path)) return null;
  // Nunca se pisa un respaldo existente: dos corridas en el mismo segundo dejan dos archivos.
  const base = `${path}.bak-pulse-${stamp()}`;
  for (let n = 0; ; n++) {
    const to = n ? `${base}-${n}` : base;
    try { copyFileSync(path, to, constants.COPYFILE_EXCL); return to; } catch (e) { if (e.code !== 'EEXIST') throw e; }
  }
}
// Aparte y renombrado encima: Claude Code vigila settings.json y podía leerlo a medio escribir. Se sigue el enlace
// simbólico, para reemplazar el archivo real y no el enlace.
const write = (path, s) => {
  const real = existsSync(path) ? realpathSync(path) : path, tmp = `${real}.pulse-tmp`;
  writeFileSync(tmp, JSON.stringify(s, null, 2) + '\n');
  renameSync(tmp, real);
};

function entry(pulse, sub, matcher, async, timeout) {
  const hook = { type: 'command', command: `"${pulse}" ${sub}` };
  if (async) hook.async = true; else hook.timeout = timeout;
  return matcher ? { matcher, hooks: [hook] } : { hooks: [hook] };
}

// Los mismos eventos que registra install.ps1 en Windows. PermissionRequest espera tu respuesta en la isla
// ("Tiempo para responder", hasta 120 s): su hook tiene 130 s para que Claude Code no lo corte antes.
const EVENTS = {
  SessionStart: ['emit', null, true], SessionEnd: ['emit', null, true], UserPromptSubmit: ['emit', null, true],
  PreToolUse: ['emit', '*', true], PermissionRequest: ['ask', null, false], Notification: ['emit', 'idle_prompt', true],
  Stop: ['emit', null, true], PostToolUseFailure: ['emit', null, true], SubagentStart: ['emit', null, true], SubagentStop: ['emit', null, true],
};

function hooksAdd(path, pulse) {
  // Una carpeta mal escrita (`~/.claud-work`) no se crea con hooks dentro: solo la de siempre puede no existir aún.
  const dir = dirname(resolve(path));
  if (!existsSync(dir) && dir !== join(homedir(), '.claude')) { console.log(`  ${dir} no existe: ahí no se registraron hooks (¿la ruta está bien escrita?)`); return; }
  const s = readSettings(path);
  const saved = backup(path);
  s.hooks ??= {};
  for (const [ev, [sub, matcher, async]] of Object.entries(EVENTS)) {
    s.hooks[ev] = [...stripOurs(s.hooks[ev]), entry(pulse, sub, matcher, async, ev === 'PermissionRequest' ? 130 : 35)];
  }
  // Preguntas de Claude (AskUserQuestion): la isla puede contestarlas antes de que salgan en la terminal.
  // 60 s visibles que se pausan mientras contestas; tope de 300 s en pulse, y el hook un poco más.
  s.hooks.PreToolUse.push(entry(pulse, 'ask', 'AskUserQuestion', false, 310));
  write(path, s);
  console.log(`  hooks en ${path}${saved ? ` (respaldo: ${saved})` : ''}`);
}

function hooksRemove(path) {
  if (!existsSync(path)) return;
  const s = readSettings(path);
  // Sin entradas de Pulse no se respalda ni se reescribe: uninstall.sh revisa todas las cuentas, no solo las que lo tienen.
  if (!countOurs(s.hooks)) return;
  const saved = backup(path);
  for (const ev of Object.keys(s.hooks)) {
    if (!Array.isArray(s.hooks[ev])) continue;
    s.hooks[ev] = stripOurs(s.hooks[ev]);
    if (!s.hooks[ev].length) delete s.hooks[ev];
  }
  if (!Object.keys(s.hooks).length) delete s.hooks;
  write(path, s);
  console.log(`  hooks de Pulse quitados de ${path} (respaldo: ${saved})`);
}

// Atajos globales: en Wayland una app no puede registrarlos, así que se dan de alta como atajos de GNOME que llaman a `pulse key`.
const SCHEMA = 'org.gnome.settings-daemon.plugins.media-keys';
const BASE = '/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/';
const KEYS = [
  ['pulse-allow', 'Pulse: permitir', '<Primary><Alt>y', 'allow'],
  ['pulse-deny', 'Pulse: denegar', '<Primary><Alt>n', 'deny'],
  ['pulse-palette', 'Pulse: paleta', '<Primary><Alt>space', 'palette'],
  ['pulse-palette-enter', 'Pulse: paleta (Enter)', '<Primary><Alt>Return', 'palette'],
];
const gs = (...a) => execFileSync('gsettings', a, { encoding: 'utf8' }).trim();
// Texto como valor GVariant: entre comillas simples y escapado (si no, `"ruta" key allow` no se puede leer).
const gstr = v => `'${v.replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'`;
// sin GNOME, gsettings se queja por stderr ("No such schema"): eso ya lo dice el mensaje de abajo
const hasGnome = () => { try { execFileSync('gsettings', ['get', SCHEMA, 'custom-keybindings'], { stdio: 'ignore' }); return true; } catch { return false; } };
// gsettings imprime "['/a/', '/b/']" o "@as []".
const readList = () => [...gs('get', SCHEMA, 'custom-keybindings').matchAll(/'([^']*)'/g)].map(m => m[1]);
const writeList = l => gs('set', SCHEMA, 'custom-keybindings', `[${l.map(p => `'${p}'`).join(', ')}]`);

function keysAdd(pulse) {
  if (!hasGnome()) { console.log('  sin GNOME: los atajos globales no se registraron (la isla se contesta con clic)'); return; }
  const list = readList();
  for (const [id, name, binding, action] of KEYS) {
    const path = `${BASE}${id}/`, schema = `${SCHEMA}.custom-keybinding:${path}`;
    gs('set', schema, 'name', gstr(name));
    gs('set', schema, 'command', gstr(`"${pulse}" key ${action}`));
    gs('set', schema, 'binding', gstr(binding));
    if (!list.includes(path)) list.push(path);
  }
  writeList(list);
  console.log('  atajos de GNOME: Ctrl+Alt+Y permite, Ctrl+Alt+N deniega, Ctrl+Alt+Espacio abre la paleta');
}

function keysRemove() {
  if (!hasGnome()) return;
  const ours = KEYS.map(([id]) => `${BASE}${id}/`);
  for (const path of ours) { try { gs('reset-recursively', `${SCHEMA}.custom-keybinding:${path}`); } catch { /* ya no estaba */ } }
  writeList(readList().filter(p => !ours.includes(p)));
  console.log('  atajos de GNOME quitados');
}

switch (cmd) {
  case 'hooks-add': hooksAdd(args[0], args[1]); break;
  case 'hooks-remove': hooksRemove(args[0]); break;
  case 'keys-add': keysAdd(args[0]); break;
  case 'keys-remove': keysRemove(); break;
  default: console.error('uso: linux-setup.mjs hooks-add <settings.json> <pulse> | hooks-remove <settings.json> | keys-add <pulse> | keys-remove'); process.exit(1);
}
