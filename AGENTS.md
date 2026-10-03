# AGENTS.md — Kagga2

Inherits `../AGENTS.md` (Rust rules: no `unwrap` outside tests, no `unsafe`, clippy/fmt/test before halting, minimal diffs). The VoxlBit-only parts of that file (NordCraft docs/cheat codes) do **not** apply here.

## Kagga2 rules
- **Work from `tasks.md`.** Do the first undone task unless told otherwise; when finished move its line to Done. Details live in `PLAN.md`.
- **Sim is headless:** nothing under `src/sim/` may import `macroquad`. Rendering is `src/gfx/`.
- **Determinism:** seeded RNG only, no wall-clock or `HashMap` iteration in the sim.
- **Generated assets only.** Never copy art/audio from `REFERENCE_tobe_DELETED/`; port logic only, with a `// ref: file fn` comment (see `REFERENCE_MAP.md`).
- **Keep it simple:** if a feature isn't in `PLAN.md`, note it in one line instead of building it.
- **Docs in sync:** after a task update `tasks.md`, `HANDOFF.md`, and any of `GAME_DESIGN.md` / `ARCHITECTURE.md` / `ASSETS.md` / `PRD.json` the change touched.
- **Version:** `Cargo.toml` `version`; minor bump per finished task, patch per fix. Offer a one-line commit message `vX.Y.Z: …`.
- Never edit `Cargo.lock` by hand.
