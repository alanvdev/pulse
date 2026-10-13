// Ajustes (spec §9, artboard Ajustes): pantalla de Mosaico con su barra lateral y una columna de 720 px con todo config.rs.
import { useEffect, useRef, useState, type ReactNode } from 'react';
import type { Config, PulseCfg } from '../../shared/bridge';
import { reduceMotion } from '../../shared/motion';
import { Icon, MultiSeg, Seg, Slider, Swatches, Switch } from '../../shared/ui';
import { useHighlight } from '../../shared/react';
import { EVENTS, FINISH, finishOf, PALETTE, seconds, SHAPES, silentLine } from '../settings';

const SECTIONS: [string, string, string][] = [['isla', 'wb_iridescent', 'Isla'], ['pulsos', 'blur_on', 'Pulsos'], ['sonidos', 'volume_up', 'Sonidos'], ['foco', 'dark_mode', 'Modo foco'], ['teclado', 'keyboard', 'Teclado'], ['apariencia', 'contrast', 'Apariencia']];
const RULES: [Config['focus']['rules'][string], string][] = [['normal', 'Normal'], ['mudo', 'Sin sonido'], ['nada', 'Nada'], ['despues', 'Después']];
const FINISH_KEYS = [['crisp', 'Línea nítida'], ['corners', 'Esquinas'], ['adapt', 'Ajuste en fondos claros'], ['dither', 'Grano']] as const;

function Row({ title, sub, children }: { title: string; sub?: string; children?: ReactNode }) {
  return <div className="aj-r"><span><b>{title}</b>{sub && <small>{sub}</small>}</span>{children}</div>;
}
function Section({ id, title, children }: { id: string; title: string; children: ReactNode }) {
  return <section id={`aj-${id}`}><h2>{title}</h2><div className="aj-box">{children}</div></section>;
}

export function Ajustes({ cfg, save, section, back }: { cfg: Config; save: (c: Config) => void; section?: string; back: () => void }) {
  const [ev, setEv] = useState('permission'), [detail, setDetail] = useState(false), [cur, setCur] = useState(section ?? 'isla');
  const main = useRef<HTMLElement>(null), nav = useRef<HTMLElement>(null);
  useHighlight(nav, 'a[aria-current]', [cur]);
  const set = <K extends keyof Config>(k: K, v: Partial<Config[K]>) => save({ ...cfg, [k]: { ...cfg[k], ...v } });
  const pc = cfg.pulses.events[ev], evName = EVENTS.find(([k]) => k === ev)?.[1] ?? ev;
  const setPc = (v: Partial<PulseCfg>) => set('pulses', { events: { ...cfg.pulses.events, [ev]: { ...pc, ...v } } });
  const fin = finishOf(cfg.pulses);
  const go = (id: string) => { setCur(id); document.getElementById(`aj-${id}`)?.scrollIntoView({ behavior: reduceMotion() ? 'auto' : 'smooth', block: 'start' }); };

  useEffect(() => { if (section) document.getElementById(`aj-${section}`)?.scrollIntoView({ block: 'start' }); }, [section]);
  // la barra marca la sección a la vista: la última cuyo inicio pasó el 40 % de arriba, o la última si ya no hay más que desplazar
  useEffect(() => {
    const root = main.current; if (!root) return;
    const onScroll = () => {
      const secs = [...root.querySelectorAll<HTMLElement>('section[id^="aj-"]')], top = root.getBoundingClientRect().top + root.clientHeight * .4;
      const atEnd = root.scrollTop + root.clientHeight >= root.scrollHeight - 2;
      const v = atEnd ? secs[secs.length - 1] : secs.filter(s => s.getBoundingClientRect().top <= top).pop() ?? secs[0];
      if (v) setCur(v.id.slice(3));
    };
    root.addEventListener('scroll', onScroll, { passive: true });
    return () => root.removeEventListener('scroll', onScroll);
  }, []);

  return <div className="mz">
    <nav ref={nav} className="mz-nav hl" aria-label="Ajustes">
      <a href="#inicio" className="aj-back" onClick={e => { e.preventDefault(); back(); }}><Icon name="arrow_back" />Volver<small /></a>
      {SECTIONS.map(([id, ic, l]) => <a key={id} href={`#ajustes/${id}`} aria-current={cur === id ? 'location' : undefined} onClick={e => { e.preventDefault(); go(id); }}><Icon name={ic} />{l}<small /></a>)}
    </nav>
    <main ref={main} className="mz-main surf aj screen-in">
      <div className="aj-col">
        <h1>Ajustes</h1>

        <Section id="isla" title="Isla">
          <Row title="Modo" sub="Comando: avisos breves y paleta. Orbe: un punto que se expande."><Seg label="Modo de la isla" value={cfg.island.mode} options={[['comando', 'Comando'], ['orbe', 'Orbe']]} onChange={mode => set('island', { mode })} /></Row>
          <Row title="Lugar del Orbe" sub="Solo en modo Orbe. También puedes arrastrarlo."><Seg label="Lugar del Orbe" value={cfg.island.orb_side} options={[['left', 'Izquierda'], ['center', 'Centro'], ['right', 'Derecha']]} onChange={orb_side => set('island', { orb_side })} /></Row>
          <Row title="Puntos mientras Claude trabaja" sub="En Comando: uno por sesión, arriba al centro"><Switch label="Puntos mientras Claude trabaja" on={cfg.island.dots} onChange={dots => set('island', { dots })} /></Row>
          <Row title="Tiempo para responder" sub="Después, Claude pregunta en la terminal. Pulse nunca decide solo."><Slider label="Segundos para responder" value={cfg.island.ask_timeout_s} min={10} max={120} step={5} format={v => `${v} s`} onChange={ask_timeout_s => set('island', { ask_timeout_s })} /></Row>
          <Row title="Cuenta regresiva"><Switch label="Cuenta regresiva" on={cfg.island.countdown} onChange={countdown => set('island', { countdown })} /></Row>
          <Row title="Haz de luz al llegar un aviso"><Switch label="Haz de luz al llegar un aviso" on={cfg.island.beam} onChange={beam => set('island', { beam })} /></Row>
          <Row title="Atajos globales" sub="Ctrl+Alt+Y permite, Ctrl+Alt+N deniega, Ctrl+Alt+Espacio abre la paleta"><Switch label="Atajos globales" on={cfg.island.keys} onChange={keys => set('island', { keys })} /></Row>
          <Row title="Mantener presionado en comandos delicados" sub="git push --force, rm -rf, migrate"><Switch label="Mantener presionado en comandos delicados" on={cfg.island.hold} onChange={hold => set('island', { hold })} /></Row>
        </Section>

        <Section id="pulsos" title="Pulsos en los bordes">
          <Row title="Pulsos"><Switch label="Pulsos" on={cfg.pulses.enabled} onChange={enabled => set('pulses', { enabled })} /></Row>
          <Row title="Dónde"><Seg label="Dónde" value={cfg.pulses.placement} options={[['active', 'Monitor activo'], ['all', 'Todos']]} onChange={placement => set('pulses', { placement })} /></Row>
          <div className="aj-evs" role="group" aria-label="Evento a editar">
            {EVENTS.map(([k, l]) => <button key={k} type="button" className="chip" aria-pressed={ev === k} onClick={() => setEv(k)}><span className="dot" style={{ background: cfg.pulses.events[k]?.color }} />{l}</button>)}
          </div>
          {pc && <>
            <Row title={`Forma · ${evName}`} sub="La forma también comunica, no solo el color"><Seg label={`Forma de ${evName}`} value={pc.shape} options={SHAPES} onChange={shape => setPc({ shape })} /></Row>
            <Row title={`Color · ${evName}`} sub="Cualquier color sirve para cualquier evento"><Swatches label={`Color de ${evName}`} value={pc.color} options={PALETTE} onChange={color => setPc({ color })} /></Row>
            <Row title="Intensidad"><Slider label="Intensidad" value={pc.intensity} min={0} max={100} onChange={intensity => setPc({ intensity })} /></Row>
            <Row title="Grosor"><Slider label="Grosor" value={pc.thickness} min={40} max={220} step={5} onChange={thickness => setPc({ thickness })} /></Row>
            <Row title="Duración"><Slider label="Duración" value={pc.duration_ms} min={300} max={3000} step={50} format={seconds} onChange={duration_ms => setPc({ duration_ms })} /></Row>
            <Row title="Deja el brillo encendido mientras espera"><Switch label="Brillo persistente" on={pc.persistent} onChange={persistent => setPc({ persistent })} /></Row>
          </>}
          <Row title="Acabado" sub={fin ? 'Línea nítida, esquinas, ajuste en fondos claros y grano' : 'Personalizado'}>
            <span className="aj-fin">
              <Seg label="Acabado" value={fin} options={[['completo', 'Completo'], ['suave', 'Suave']]} onChange={f => set('pulses', FINISH[f])} />
              <button type="button" className="btn" aria-expanded={detail} onClick={() => setDetail(!detail)}><span>Detalle</span><Icon name={detail ? 'expand_less' : 'expand_more'} /></button>
            </span>
          </Row>
          {detail && <div className="aj-detail">{FINISH_KEYS.map(([k, l]) => <label key={k} className="aj-chk"><input type="checkbox" checked={cfg.pulses[k]} onChange={e => set('pulses', { ...cfg.pulses, [k]: e.target.checked } as Config['pulses'])} />{l}</label>)}</div>}
        </Section>

        <Section id="sonidos" title="Sonidos">
          <Row title="Sonidos"><Switch label="Sonidos" on={cfg.sounds.enabled} onChange={enabled => set('sounds', { enabled })} /></Row>
          <Row title="Volumen"><Slider label="Volumen" value={cfg.sounds.volume} min={0} max={100} step={5} onChange={volume => set('sounds', { volume })} /></Row>
          <Row title="Suenan" sub={silentLine(cfg.sounds.per_event)}><MultiSeg label="Qué eventos suenan" values={cfg.sounds.per_event} options={EVENTS.map(([k, l]) => [k, l === 'Te espera' ? 'Espera' : l])} onChange={(k, on) => set('sounds', { per_event: { ...cfg.sounds.per_event, [k]: on } })} /></Row>
        </Section>

        <Section id="foco" title="Modo foco">
          <Row title="Activar solo" sub="Si lo apagas a mano, vuelve cuando cambie lo que lo activó"><Switch label="Activar solo" on={cfg.focus.auto} onChange={auto => set('focus', { auto })} /></Row>
          <Row title="En llamadas"><Switch label="En llamadas" on={cfg.focus.detect_call} onChange={detect_call => set('focus', { detect_call })} /></Row>
          <Row title="En pantalla completa o presentación"><Switch label="En pantalla completa o presentación" on={cfg.focus.detect_fullscreen} onChange={detect_fullscreen => set('focus', { detect_fullscreen })} /></Row>
          <Row title="En un horario" sub={`${cfg.focus.from} a ${cfg.focus.to}, todos los días`}><Switch label="En un horario" on={cfg.focus.schedule} onChange={schedule => set('focus', { schedule })} /></Row>
          {cfg.focus.schedule && <Row title="Desde y hasta"><span className="aj-fin">
            <input type="time" className="aj-time" aria-label="Desde" value={cfg.focus.from} onChange={e => e.target.value && set('focus', { from: e.target.value })} />
            <input type="time" className="aj-time" aria-label="Hasta" value={cfg.focus.to} onChange={e => e.target.value && set('focus', { to: e.target.value })} />
          </span></Row>}
          {EVENTS.map(([k, l]) => <Row key={k} title={l}><Seg label={`${l} en foco`} value={cfg.focus.rules[k] ?? 'normal'} options={RULES} onChange={r => set('focus', { rules: { ...cfg.focus.rules, [k]: r } })} /></Row>)}
        </Section>

        <Section id="teclado" title="Teclado">
          <Row title="Razer Chroma" sub="F1 a F3 se encienden con la sesión que te espera"><Switch label="Razer Chroma" on={cfg.keyboard.enabled} onChange={enabled => set('keyboard', { enabled })} /></Row>
          <Row title="F1 a F3 por sesión"><Switch label="F1 a F3 por sesión" on={cfg.keyboard.session_keys} onChange={session_keys => set('keyboard', { session_keys })} /></Row>
          <Row title="Brillo de fondo"><Slider label="Brillo de fondo" value={cfg.keyboard.base} min={0} max={100} onChange={base => set('keyboard', { base })} /></Row>
        </Section>

        <Section id="apariencia" title="Apariencia">
          <Row title="Tema" sub="Bolsillo, el menú de la bandeja y esta ventana. La isla siempre es oscura."><Seg label="Tema" value={cfg.appearance.theme} options={[['dark', 'Oscuro'], ['system', 'Como el sistema'], ['light', 'Claro']]} onChange={theme => set('appearance', { theme })} /></Row>
        </Section>
      </div>
    </main>
  </div>;
}
