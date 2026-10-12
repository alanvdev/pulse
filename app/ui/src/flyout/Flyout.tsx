// Flyout del clic derecho de la bandeja (spec §6). Se cierra al perder el foco (núcleo), con Esc o al elegir una acción.
import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { flyoutFit, flyoutHide, muteSession, onFlyoutOpen, openDashboard, openPalette, quit, setFocus, type Config } from '../shared/bridge';
import { statusLine, topDot } from '../shared/format';
import { useConfig, useFocusEdge, useFocusHalo, useSnapshot, useTheme } from '../shared/react';
import { Dot, Icon, Keys } from '../shared/ui';
import { step } from './menu';

type Item = { id: string; icon: string; label: string; keys?: string[]; on?: boolean; disabled?: boolean; run: () => void };

export function Flyout() {
  const snap = useSnapshot(), [cfg, save] = useConfig();
  useTheme(cfg);
  const [open, setOpen] = useState(0);   // cambia en cada apertura: reinicia la animación y la selección
  const box = useRef<HTMLDivElement>(null), icons = useRef<Record<string, string>>({});
  const focusEv = useFocusEdge(snap);
  useFocusHalo(box, focusEv);
  useEffect(() => { let off: (() => void) | undefined; onFlyoutOpen(() => setOpen(n => n + 1)).then(f => (off = f)); return () => off?.(); }, []);

  const now = Date.now();
  const perm = snap.pending.some(p => p.kind === 'permission');
  const allMuted = snap.sessions.length > 0 && snap.sessions.every(s => s.muted_until_ms > now);
  const close = () => { (document.activeElement as HTMLElement | null)?.blur(); flyoutHide(); };
  const toggle = <K extends 'sounds' | 'pulses' | 'keyboard'>(c: Config, k: K) => save({ ...c, [k]: { ...c[k], enabled: !c[k].enabled } });
  const groups: Item[][] = !cfg ? [] : [
    [
      ...(perm ? [{ id: 'perm', icon: 'front_hand', label: 'Responder permiso', keys: ['Ctrl', 'Alt', 'Espacio'], run: () => { openPalette(); close(); } }] : []),
      { id: 'open', icon: 'open_in_new', label: 'Abrir Pulse', run: () => { openDashboard(); close(); } },
      { id: 'mute', icon: allMuted ? 'notifications_active' : 'notifications_paused', label: allMuted ? 'Quitar silencio' : 'Silenciar 30 min', disabled: !snap.sessions.length,
        run: () => { snap.sessions.forEach(s => muteSession(s.id, allMuted ? 0 : 30)); close(); } },
    ],
    [
      { id: 'focus', icon: 'dark_mode', label: 'Modo foco', on: snap.focus, run: () => setFocus(!snap.focus) },
      { id: 'sounds', icon: cfg.sounds.enabled ? 'volume_up' : 'volume_off', label: 'Sonidos', on: cfg.sounds.enabled, run: () => toggle(cfg, 'sounds') },
      { id: 'pulses', icon: cfg.pulses.enabled ? 'blur_on' : 'blur_off', label: 'Pulsos en los bordes', on: cfg.pulses.enabled, run: () => toggle(cfg, 'pulses') },
      { id: 'keyboard', icon: 'keyboard', label: 'Teclado', on: cfg.keyboard.enabled, run: () => toggle(cfg, 'keyboard') },
    ],
    [
      { id: 'settings', icon: 'settings', label: 'Ajustes', run: () => { openDashboard('ajustes'); close(); } },
      { id: 'quit', icon: 'power_settings_new', label: 'Salir', run: () => quit() },
    ],
  ];
  const items = groups.flat();
  useEffect(() => { items.forEach(it => { icons.current[it.id] = it.icon; }); });   // el icono gira solo cuando cambia

  // el núcleo ajusta la ventana a lo que mide el menú (más el margen de 12 px para la sombra)
  useLayoutEffect(() => { if (box.current) flyoutFit(box.current.offsetHeight + 24); }, [items.length]);
  // al abrir, el foco va al menú: el lector de pantalla lo anuncia y las flechas funcionan de inmediato
  useEffect(() => { if (open) box.current?.focus(); }, [open]);

  // flechas entre filas (Enter y Espacio los resuelve el botón enfocado), Esc cierra
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') { e.preventDefault(); close(); return; }
      if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
      e.preventDefault();
      const rows = [...(box.current?.querySelectorAll<HTMLElement>('.mi') ?? [])];
      const next = step(items.map(i => !i.disabled), rows.indexOf(document.activeElement as HTMLElement), e.key === 'ArrowDown' ? 1 : -1);
      rows[next]?.focus();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [items]);

  if (!cfg) return null;
  let i = -1;
  return <div className="fly-wrap">
    <div key={open} ref={box} role="menu" aria-label="Pulse" aria-describedby="fly-status" tabIndex={-1} className={`fly surf f-host ${snap.focus ? 'in-focus' : ''}`}>
      <div className="fly-head" role="presentation"><b>Pulse</b><span id="fly-status"><Dot kind={topDot(snap)} />{statusLine(snap)}</span></div>
      {groups.map((g, gi) => <div key={gi} role="group">
        <div className="sep" role="separator" />
        {g.map(it => { i++; const toggleRow = it.on !== undefined;
          return <button key={it.id} type="button" className="mi" data-i={i} role={toggleRow ? 'menuitemcheckbox' : 'menuitem'} aria-checked={toggleRow ? it.on : undefined}
            data-focus={it.id === 'focus' && it.on ? 'on' : undefined} aria-disabled={it.disabled || undefined} onClick={() => { if (!it.disabled) it.run(); }}>
            <Icon key={it.icon} name={it.icon} className={icons.current[it.id] && icons.current[it.id] !== it.icon ? 'swap' : undefined} /><span>{it.label}</span>
            {toggleRow ? <span className={`sw sm ${it.on ? 'on' : ''}`} aria-hidden="true" /> : it.keys ? <Keys k={it.keys} /> : <span />}
          </button>; })}
      </div>)}
    </div>
  </div>;
}
