# Pulse

Windows desktop app (Tauri v2) that shows what Claude Code is doing outside the terminal. What the product is and who it's for: `PRODUCT.md`. How it looks: `DESIGN.md` once it exists; until then, the current code is the starting point, not a rule.

## Commands

```powershell
cargo test --workspace                 # Rust: protocol, CLI, core
npm test --prefix app                  # Vitest: UI logic
npm run tauri dev --prefix app         # app in development (Vite on :5173)
pwsh scripts/install.ps1               # build, install to %LOCALAPPDATA%\Pulse and register hooks
pwsh scripts/uninstall.ps1             # restore settings.json from the backup
```

## Architecture

- Claude Code hook → `pulse.exe` (`crates/pulse-cli`) → named pipe `\\.\pipe\pulse` → app (`app/src-tauri`). Types and hook translation live in `crates/pulse-protocol`.
- The core keeps the state and sends it to every window over two events: `pulse://state` (full snapshot) and `pulse://fx` (one-off effect). Windows call the core through `app/ui/src/shared/bridge.ts`.
- Edges (`overlay`) and the island: TypeScript with no framework. Bolsillo (`panel`) and Mosaico (`dashboard`): React 18 + Tailwind 4.
- User data in `~/.claude/pulse/` (`config.json`, `log.jsonl`).

## Rules that break Claude Code if you break them

- `pulse.exe` always exits with code 0.
- `pulse emit` never writes to stdout and takes under 4 s in the worst case.
- `pulse ask` writes to stdout only the decision JSON, or nothing. With no answer it decides nothing and Claude asks in the terminal.
- The installer backs up `~/.claude/settings.json` (with the date) before touching it, and only replaces entries that call `pulse.exe` or `sounds/play.ps1`.

## Conventions

- UI text in Spanish.
- `prefers-reduced-motion` respected in every window; an event's meaning never depends on color alone.
- `docs/spec.md` and `docs/plans/` are the history of a finished plan, not rules. `docs/artboards/` were the visual starting point and don't bind visual changes.

## Working with the user

The user is new to Rust and Tauri. Before a technical decision (crate, framework, approach), explain the alternatives and what each one gives up instead of deciding silently.
