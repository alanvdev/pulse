# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

(Desktop app for Windows 11 built on Tauri v2. Every surface is HTML/CSS/TypeScript running in WebView2, so the design language is web, not native Windows controls.)

## Users

Right now the main user is the person who built it: a developer who runs Claude Code in the terminal on Windows and spends that time in other windows (browser, editor, calls) instead of watching the terminal. A public release for any Claude Code user on Windows is possible later but not decided (see Capabilities and Constraints).

## Product Purpose

Pulse shows what Claude Code is doing outside the terminal, so the user can look away from it without losing time. It does two jobs above all:

1. **Know when it finished.** A clear, glanceable signal when a session stops, without having to check the terminal.
2. **Don't lose permissions.** When Claude stops to ask for permission, the user notices right away and can answer from wherever they are, instead of coming back minutes later to a session that has been waiting.

Success: no minutes lost to a session that finished or stalled on a permission without the user noticing.

Following several sessions and subagents at once, and protecting concentration (focus mode), are supporting jobs, not the main reason it exists.

## Positioning

Pulse is fed directly by Claude Code's own hooks, so its signals are exact (this session is asking for permission, this one finished, this one failed), not guesses based on terminal output. It can answer a permission request from outside the terminal (island, palette, global shortcuts), and when nobody answers it leaves the decision to the terminal as usual. It reaches the user through several channels at once, each with a different level of intrusiveness: light at the screen edges, the island, sound, the tray, and the keyboard.

## Operating Context

- Claude Code runs in one or more terminals; the user is usually in another app, sometimes in a call, a presentation, or full screen.
- Surfaces, from least to most attention: edge pulses (transparent, click-through, on the active monitor); the island at the top center (Comando mode by default, Orbe as the alternative, never both at once); the Bolsillo panel from the tray icon; the full Mosaico dashboard.
- Input happens without opening a window: `Ctrl+Alt+Space` (palette), `Ctrl+Alt+Y` (allow), `Ctrl+Alt+N` (deny). Sensitive commands are approved by holding for 900 ms.
- Starts on demand with the first hook and closes itself 5 minutes after the last activity. It does not register with Windows startup.
- Logbook in `~/.claude/pulse/log.jsonl`, feeding the "Hoy" (Today) section and `/daily-report`.

## Capabilities and Constraints

- Events and what they mean: `start` (prompt sent), `permission` (waiting for a decision), `done` (finished), `idle` (waiting for the user), `error` (tool failed), `subagent` (subagent finished). `tool` and `session-*` feed activity and the session count but produce no pulse. The meaning of each event is product truth; its color is not.
- A permission is never decided silently: if nobody answers within the time limit (30 s), Pulse decides nothing and Claude asks in the terminal. If the user answers in the terminal, the island stops waiting.
- Focus mode: turns on automatically (microphone in use, full screen/presentation, schedule) or by hand; per-event rule Normal / Sin sonido (no sound) / Nada (nothing) / Después (later).
- Razer Chroma keyboard: F1–F3 assigned to sessions. The current user's computer has it; you cannot assume other users will.
- Not possible (platform limits): "Go to terminal" and "Reply 'continue'" (hooks don't say which window a session runs in, and there is no API to write to Claude), and detecting Windows "Do not disturb".
- All visible text is in Spanish. Respect `prefers-reduced-motion` in every window.
- **Visual source of truth:** the artboards in `docs/artboards/` (and their live copies) were the starting point for building the first version. They are **not** a binding reference for visual changes from here on; future visual work may move away from them. The current code is the incumbent implementation, not a rule.
- **Open:** public release. If it goes ahead, it has to work without a Razer keyboard, with one monitor, and with a first run that explains the hooks and the installation.

## Brand Commitments

- Name: **Pulse**. Pieces with their own names in Spanish: Comando, Orbe (island), Bolsillo (tray panel), Mosaico (dashboard), Hoy (logbook).
- Voice: plain, short, everyday Spanish, as in the current UI text.
- Sounds: the four `.wav` files in `assets/sounds/` (credits in `CREDITS.md`).
- The Geist and Geist Mono fonts come from the starting artboards; since the artboards no longer bind, the fonts are not a brand commitment either.

## Evidence on Hand

- Working code for all four surfaces in `app/ui/src/` and the core in `app/src-tauri/src/`.
- Real logbook of the user's own events in `~/.claude/pulse/log.jsonl` (local data, not public).
- There are no testimonials, users other than the author, metrics, or public screenshots. Do not make them up.

## Product Principles

1. **Signal before decoration.** Every light, sound, or animation means a specific Claude Code event; nothing moves without a reason.
2. **Notice without being interrupted.** Pick the lowest level of intrusiveness that does the job (edge → island → sound → panel), and step up only when Claude needs the user.
3. **Answer from where you are.** A permission can be resolved without going to the terminal or opening a window.
4. **Never decide for the user.** If there is no answer, the terminal stays in charge; Pulse does not approve or deny anything by default.
5. **Invisible when there's nothing going on.** At rest it takes up no space and closes on its own.

## Accessibility & Inclusion

- `prefers-reduced-motion` respected in every window (the pulses must still communicate without full animation).
- The meaning of an event must not depend on color alone: the pulse shape, the sound, and the island text carry it too.
