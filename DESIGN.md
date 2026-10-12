---
name: Pulse
description: Grafito, a graphite-neutral interface where only Claude Code's events carry color.
colors:
  side: "#090a09"
  bg: "#0a0b0a"
  surface: "#111311"
  raised: "#181b18"
  barbg: "#0e100e"
  well: "#0d0f0d"
  line: "rgba(234,237,234,.08)"
  line2: "rgba(234,237,234,.16)"
  sel: "rgba(234,237,234,.07)"
  off: "rgba(234,237,234,.12)"
  knob: "#9ea39e"
  work: "rgba(234,237,234,.55)"
  tx: "#eaedea"
  tx2: "#9ea39e"
  tx3: "#7d827d"
  perm: "#e3b05c"
  done: "#8fc3a0"
  err: "#e08a7e"
  sub: "#8fb0d6"
  focus: "#b4a6d9"
  side-light: "#e9ece8"
  bg-light: "#e6e9e6"
  surface-light: "#f6f8f5"
  raised-light: "#ebeeea"
  barbg-light: "#f1f3f0"
  well-light: "#eef1ed"
  line-light: "rgba(31,36,32,.09)"
  line2-light: "rgba(31,36,32,.18)"
  sel-light: "rgba(31,36,32,.06)"
  tx-light: "#1f2420"
  tx2-light: "#545a54"
  tx3-light: "#676d67"
  perm-light: "#9a6512"
  done-light: "#2f6e4a"
  err-light: "#a8473a"
  sub-light: "#4a6f94"
  focus-light: "#6b5aa6"
typography:
  display:
    fontFamily: "Hanken Grotesk, ui-sans-serif, system-ui, Segoe UI, sans-serif"
    fontSize: "26px"
    fontWeight: 500
    letterSpacing: "-0.03em"
  headline:
    fontFamily: "Hanken Grotesk, ui-sans-serif, system-ui, Segoe UI, sans-serif"
    fontSize: "24px"
    fontWeight: 500
    letterSpacing: "-0.025em"
  title:
    fontFamily: "Hanken Grotesk, ui-sans-serif, system-ui, Segoe UI, sans-serif"
    fontSize: "20px"
    fontWeight: 500
    letterSpacing: "-0.02em"
  body:
    fontFamily: "Hanken Grotesk, ui-sans-serif, system-ui, Segoe UI, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.45
    letterSpacing: "-0.005em"
  label:
    fontFamily: "Hanken Grotesk, ui-sans-serif, system-ui, Segoe UI, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.45
  mono:
    fontFamily: "Geist Mono, ui-monospace, Cascadia Mono, Consolas, monospace"
    fontSize: "12px"
    fontWeight: 400
    letterSpacing: "0"
    fontFeature: "tnum"
rounded:
  key: "4px"
  control: "6px"
  row: "7px"
  well: "8px"
  block: "10px"
  flyout: "12px"
  panel: "14px"
  box: "20px"
  canvas: "22px"
spacing:
  xs: "6px"
  sm: "10px"
  md: "14px"
  lg: "20px"
  xl: "26px"
components:
  button:
    backgroundColor: "transparent"
    textColor: "{colors.tx2}"
    rounded: "{rounded.control}"
    padding: "0 9px"
    height: "28px"
  button-hover:
    backgroundColor: "{colors.sel}"
    textColor: "{colors.tx}"
  button-primary:
    backgroundColor: "{colors.sel}"
    textColor: "{colors.tx}"
    rounded: "{rounded.control}"
    padding: "0 9px"
    height: "28px"
  button-primary-hover:
    backgroundColor: "{colors.line2}"
  chip:
    backgroundColor: "transparent"
    textColor: "{colors.tx3}"
    rounded: "{rounded.control}"
    padding: "0 10px"
    height: "28px"
  chip-on:
    backgroundColor: "{colors.sel}"
    textColor: "{colors.tx}"
  segmented:
    backgroundColor: "{colors.well}"
    rounded: "{rounded.well}"
    padding: "3px"
  segmented-on:
    backgroundColor: "{colors.sel}"
    textColor: "{colors.tx}"
    rounded: "{rounded.control}"
    height: "28px"
  switch:
    backgroundColor: "{colors.off}"
    rounded: "11px"
    width: "38px"
    height: "22px"
  switch-on:
    backgroundColor: "{colors.tx}"
  key:
    backgroundColor: "rgba(234,237,234,.04)"
    textColor: "{colors.tx3}"
    rounded: "{rounded.key}"
    padding: "0 4px"
    height: "18px"
  menu-item:
    backgroundColor: "transparent"
    textColor: "{colors.tx}"
    rounded: "{rounded.row}"
    padding: "0 10px"
    height: "34px"
---

# Design System: Pulse

## Overview

**Creative North Star: "Grafito"**

Pulse is a set of quiet graphite surfaces that sit beside Claude Code and speak only when something happens. The interface itself has no brand color: neutrals in a faintly green-gray graphite carry every surface, every control and every primary action. Color is reserved for the six events (permission amber, done green, error coral, subagent blue, plus neutral light for start and idle) and for one state, focus mode, in lavender. When color appears, it means something.

Density is that of a desktop utility: 12 to 14 px text, 28 to 48 px rows, tight 6 to 8 px control radii, flat tonal layers with a single soft drop shadow on floating surfaces. Motion is physical (springs for the island, short eased fades and presses everywhere else) and always attached to a state change. Two themes (dark default, light via `data-theme="light"`) share the same token names, so every rule below holds in both; the island and the edge pulses are always dark-tuned.

**Key Characteristics:**
- No brand color; the primary action is ink (text color), never a hue.
- Color only for events and lavender only for focus mode, always paired with an icon, shape or word.
- Session state is read by dot shape first (filled, haloed, ringed), color second.
- Flat tonal layers (side, bg, surface, raised, well) with hairline borders; one soft ambient shadow on floating windows.
- Hanken Grotesk for everything; Geist Mono only for commands, paths, keys of time and numbers.
- Springs and microinteractions exist only to confirm a change; all of them degrade under `prefers-reduced-motion`.

## Colors

A graphite neutral ramp with a hint of green, and five muted event hues that never decorate.

### Primary
- **Ink** (tx): the primary color. Primary buttons are "solo texto" with a `sel` fill and ink text; switches turn ink when on; slider fill and thumb are ink; the selected swatch ring is ink. There is no separate accent.

### Secondary (event colors, meaning only)
- **Permission Amber** (perm): a session is waiting for a decision or a question. Halo dot, countdown bar, hold-to-approve line, urgent countdown, pending icon in Mosaico, striped "esperó" lane in the day timeline.
- **Done Sage** (done): finished. Ringed dot, resolved check ("Permitido"), done flash on rows.
- **Error Coral** (err): a tool failed, or a decision was "Denegado". Error ticks on the timeline, the deny cross.
- **Subagent Blue** (sub): a subagent finished. Used in pulses, logbook and legends, nowhere else.
- Edge pulses read their colors from `config.json` (defaults match the dark tokens; start and idle default to a neutral `#b3bab5`), so a user can recolor events without touching the UI tokens.

### Tertiary
- **Focus Lavender** (focus): focus mode, and nothing else. It always travels with the moon icon (`dark_mode`) or the word "foco": the island moon, the lavender halo that breathes once on entering, the faint lavender inner border while focus lasts, the focus chip when on, the lavender switch in the flyout's focus row, and a soft lavender `barrido` sweep across the screen edges.

### Neutral
- **Side** (side): the Mosaico window background behind the sidebar.
- **Surface** (surface): floating surfaces (island shell, Bolsillo, flyout, Mosaico canvas).
- **Raised** (raised): blocks inside a surface: pending permission, command well, resolved note, palette input, Ajustes boxes.
- **Bar** (barbg): footers and action bars at the bottom of a surface.
- **Well** (well): segmented control trough and time inputs.
- **Line / Line 2** (line, line2): hairline borders and dividers; line2 for stronger edges and hover on primary.
- **Selection** (sel): hover, pressed, selected rows, sliding highlight, primary button fill.
- **Text 2 / Text 3** (tx2, tx3): secondary text and icons; tertiary metadata and placeholders. Light theme lifts `--hi` to white for a top inner highlight and `--knob` to white.
- **Work** (work), **Track** (track), **Off** (off), **Knob** (knob), **Key** (key): timeline bars and lanes, the off switch, the switch knob, and key caps.

### Named Rules
**The No Brand Color Rule.** The UI has no accent. A sage accent was tried and dropped because it competed with "terminó". Primary means ink plus `sel`, in both themes.

**The Color Means an Event Rule.** perm, done, err and sub appear only where that event or its result is being shown. Never use them for decoration, section identity or emphasis.

**The Lavender Means Focus Rule.** focus is used only for focus mode and always next to the moon icon or the word "foco".

## Typography

**Body Font:** Hanken Grotesk (with ui-sans-serif, system-ui, Segoe UI)
**Label/Mono Font:** Geist Mono (with ui-monospace, Cascadia Mono, Consolas)
**Icons:** Material Symbols Rounded, weight 300, 16 to 18 px by default.

**Character:** A plain, slightly soft grotesk at small sizes for an everyday utility voice, with a monospace that marks anything literal: commands, file paths, durations and clocks. All three are bundled locally as variable woff2 files.

### Hierarchy
- **Display** (500, 26px, -0.03em): Ajustes page title only.
- **Headline** (500, 24px, -0.025em): Mosaico screen titles ("Inicio", "Sesiones", "Hoy", "Bitácora").
- **Title** (500, 20px, -0.02em): Bolsillo's headline figure and Mosaico stat values (stats at 400).
- **Body** (400, 13 to 14.5px, 1.45): rows and cards at 13 to 13.5px; the base is 14px; questions and pending commands in Mosaico at 14.5px.
- **Label** (400 to 500, 11 to 12.5px): secondary lines, section headings in Mosaico (13px 500 in tx2), times, legends, footers. Key caps at 10.5px.
- **Mono** (400, 11.5 to 13.5px, tabular figures, 0 tracking): commands, paths, clocks, counts. The island command well is 13.5px with 1.55 line height, clamped to three lines.

### Named Rules
**The Literal Is Mono Rule.** A command, file, path, clock or duration goes in Geist Mono with tabular figures; prose and labels never do.

**The Weight Ceiling Rule.** Weights are 400 and 500; 600 only for the primary button label and fuzzy-match highlights in the palette. No bold display type, no all caps.

## Layout

Each surface has a fixed frame; content stretches within it rather than reflowing.

- **Island** (always dark): a transparent full-width window at the top of the active monitor (height 80% of the monitor, at most 860px). Comando sits top center, 12px from the top: a 24px capsule of session dots that morphs into a 470px summary of 36px rows, a 480px palette (`Ctrl+Alt+Space`) or permission/question cards up to 520px wide (card body padding 18px 20px 16px, action bar 50px). Orbe is a 24px bead at left, center or right (draggable to three dashed zones), peeking to 34px and opening a 42px icon menu of 32px buttons. Comando and Orbe are never shown together.
- **Edges** (overlay): transparent, click-through windows that draw pulses on the screen border of the active monitor (or all, by setting).
- **Tray icon**: drawn in Rust at 32 × 32 px. See Components.
- **Flyout**: 300px wide (324px window with 12px transparent margin), 5px inner padding, 34px menu rows, anchored over the tray icon; height follows content.
- **Bolsillo**: 380 × 580px panel over the tray, 14px corners; scrolling view (padding 20px 18px 12px, 20px gaps) above a 44px tab footer.
- **Mosaico**: 1120 × 760px window (minimum 720 × 520). A 236px sidebar on `side`, and a main canvas inset 10px with 22px corners, padding 26px 30px 30px, sections 26px apart. Rows are 48px, stat tiles at least 150px in an auto-fit grid. Ajustes uses a centered 720px column with 36px between sections. Below 900px the session table drops the time column and settings rows stack.

Spacing has no strict scale; the recurring rhythm is 6, 10, 14, 20, 26px.

## Elevation & Depth

Depth is tonal first: `side` and `bg` behind, `surface` for the floating window, `raised` and `well` for blocks inside it, separated by 1px `line` hairlines. Floating surfaces add one soft ambient shadow and an inset hairline plus top highlight; nothing inside a surface casts a shadow.

### Shadow Vocabulary
- **Floating surface** (`box-shadow: inset 0 0 0 1px var(--line), inset 0 1px 0 var(--hi), 0 24px 48px -12px var(--shadow)`): Bolsillo and Mosaico canvas; the island uses `0 28px 56px -16px`.
- **Short float** (`0 6px 12px -4px var(--shadow)` with the same insets): the flyout, whose window leaves only 12px of transparent margin.
- **Stacked queue** (two zero-blur layers of `raised` peeking 6 and 12px below the island card): only to show that more permissions are queued behind the current one. It is a stack depiction, not a shadow style.
- **Focus border** (inset 1px of focus at 40%): replaces the hairline on a floating surface while focus mode is on.

### Named Rules
**The Flat Inside Rule.** Inside a surface, blocks separate by tone and hairline, never by shadow.

## Shapes

Small, consistent corners that grow with the size of the container: 4px keys and tags, 6px buttons, chips, segments, swatches and the focus outline, 7px rows and the sliding highlight, 8px wells, command blocks and options, 10px raised blocks, 12px flyout, 14px Bolsillo and island cards, 20px Mosaico boxes, 22px Mosaico canvas. The island capsule morphs its radius with its height (8, 10, 12 or 14px). Circles are reserved for meaning: dots, radio marks, the switch knob and slider thumb. The switch is the only pill.

**The No Pill Button Rule.** Buttons, chips and segments use 6px corners. Only the switch (and its knob) is fully rounded.

## Components

### Buttons ("solo texto")
Quiet text buttons; the action lives in the word, not the fill.
- **Shape:** 6px corners, 28px tall (md 32px, lg 36px with 14px padding), 12.5 to 13px label, optional trailing key caps.
- **Secondary:** transparent, tx2; hover fills `sel` and turns ink. On its own (outside a bar) it gets a 1px `line` border.
- **Primary:** `sel` fill, ink text, weight 600; hover deepens to `line2`.
- **Hold:** sensitive commands read "Mantener"; a 2px amber line grows along the bottom over 900ms.
- **Press:** scales to .96. Disabled at 45% opacity.

### Keys
Key caps (`kbd`): 18px tall, 4px corners, `key` fill, hairline border, 10.5px tx3. Shown next to the action they trigger ("Ctrl Alt Y", "↵").

### Chips
- **Style:** 28px, 6px corners, hairline border, tx3 text with an optional 14px icon.
- **State:** `aria-pressed="true"` turns ink on `sel` with `line2` border. The focus chip turns lavender.
- **Behavior:** a ripple fills from the click point (center on keyboard); a changed icon (volume to mute) swaps in with a short turn.

### Switch, Segmented, Slider, Swatches
- **Switch:** 38 × 22px pill (small 28 × 16), `off` fill, ink when on with the knob turning `surface`. A real `input type=checkbox role=switch`; the knob stretches while pressed.
- **Segmented:** a `well` trough with 3px padding and 8px corners; 28px segments, selected one on `sel` in ink. Also used for multi-select (which events sound).
- **Slider:** 170px track (120px in Bolsillo), 6px tall, ink fill to the value, 16px ink thumb with a `raised` border; the thumb and the value readout grow while dragging.
- **Swatches:** 24px squares with 6px corners and an inner hairline; the selected one gets a 2px gap and an ink ring. A radio group.

### Dots (session state)
7px circles; the shape carries the state, color confirms it.
- **fill** (ink): working.
- **halo** (amber with a 3px amber ring at 22%): needs you (permission or question).
- **done** (1.5px sage ring): finished.
- **calm** (1.5px tx3 ring): idle or waiting for you.

### Rows and lists
36px rows in the island summary, 34px in the flyout, 48px in Mosaico, separated by hairlines or 2px gaps. Hover and selection fill `sel`. Rows enter staggered (30 to 40ms apart) and flash sage or amber for 0.9s when their session changes to done or waiting.

### Navigation
Mosaico sidebar links (32px, 7px corners, tx2 with tx3 icons, ink when current) and Bolsillo tabs (26px, tx3, ink when selected) share one sliding `sel` highlight that moves to the active item instead of a per-item fill.

### Permission and resolved blocks
A pending permission is a `raised` block (10px corners in Bolsillo, 20px in Mosaico) with project, command in mono, and "Permitir" / "Denegar" actions with keys. Once decided it becomes a resolved note ("Permitido" with a drawn sage check, "Denegado" with a coral cross, "Sin respuesta: pasa a la terminal"), holds about half a second, then folds away while the content below rises. In the last five seconds the countdown beats in amber.

### Island (signature)
A single shell that morphs between capsule, summary, palette, notices and cards on springs: open k260/d23, close k340/d38, move k200/d24 (fallback `cubic-bezier(.32,.72,0,1)` 520ms). Old content blurs out as new content blurs in. Notices are 40px with an icon in the event color and a 1px tx3 timer line. Events can fire a light beam from the top before the shell ignites.

### Edge pulses (signature)
Each event has its own shape so it reads without color: "respira" (start, idle), "latido" double beat (permission), "ola" rotating sweep (done), "lateral" flash on one side (error), "origen" expanding ring (subagent), "barrido" sweep (focus entering). Optional crisp line, corner glow and color grain. On a light background the color darkens 18% and multiplies. Permission and idle can leave a slowly breathing residual glow until resolved; focus mode turns residuals off.

### Tray icon
Drawn in Rust as up to three dots with the same four shapes: no sessions is one large gray ring; one session is one large dot; two sit side by side; three form a triangle, in slot order. On a light Windows taskbar it switches to dark ink (`tx-light`, `perm-light`, `done-light`, `tx3-light`). Redrawn only when the dots change. Left click opens Bolsillo, right click opens the flyout (no native menu).

### Motion and reduced motion
Easing tokens: `--e-out` `cubic-bezier(.16,1,.3,1)` for entrances and presses, `--e-in` `cubic-bezier(.7,0,.84,0)` for folds. Microinteractions: press scale (.96 controls, .98 menu rows, .9 Orbe buttons), chip ripple, icon swap, focus halo (1.6s in, 0.7s out), resolved fold, urgent countdown, row stagger and flash, sliding highlight (.34s), count-up stats (700ms ease-out cubic), screen fade (.28s). Under `prefers-reduced-motion`: springs and morphs become 120ms fades, every pulse becomes "respira", residuals hold steady, presses, ripples, swaps, stagger and the urgent beat are removed, folds shorten to .12s, flashes and fades remain, and infinite animations play once.

## Do's and Don'ts

### Do:
- **Do** make the primary action ink on `sel` ("solo texto"), in both themes.
- **Do** pair every event color with a shape, icon or word: dot kind, pulse shape, island text.
- **Do** keep text at 4.5:1 or better against the surface it sits on; tx3 is the floor for readable text.
- **Do** use real `button`, `input` and ARIA roles (`switch`, `radiogroup`, `aria-pressed`, `aria-current`) so every control has a keyboard path; the visible focus outline is 2px tx2 with 2px offset (inset -2px on full-width rows).
- **Do** read colors from the tokens (`var(--tx)` or the Tailwind theme aliases) so the light theme follows.
- **Do** give every new animation a reduced-motion fallback in the same stylesheet.
- **Do** keep UI text in plain Spanish, as shipped.

### Don't:
- **Don't** add a brand or accent color, or use an event color for decoration.
- **Don't** use lavender for anything but focus mode.
- **Don't** make buttons, chips or segments pills; the switch is the only pill.
- **Don't** put `data-theme` on the island; it is always the dark version.
- **Don't** wrap `shared/ui.css` in a cascade layer, and don't stack Tailwind utilities on its classes (`btn`, `chip`, `seg`, `sw`, `kbd`): unlayered ui.css wins over Tailwind's utilities layer, so the utility silently does nothing. Add a variant to ui.css or a surface stylesheet instead.
- **Don't** add shadows inside a surface; separate blocks by tone and hairline.
- **Don't** animate anything that does not correspond to a state change.
