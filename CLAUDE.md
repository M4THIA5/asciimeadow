# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`asciimeadow` is a terminal screensaver: an animated ASCII meadow (tree, animals, weather, day/night cycle) rendered with `crossterm`. Dependencies are limited to `crossterm` and `rand`.

The codebase and its comments are written in **French** — match that when adding comments or docstrings.

## Commands

```bash
cargo run                                  # run the animation (needs a real TTY)
cargo run -- --seed 42                     # deterministic run
cargo run -- --fps 30 --day-length 60
cargo test                                 # run all tests (lib + bin)
cargo test --lib scene                     # run one module's tests
```

Runtime keys: `q` quit, `p` pause, `r` redraw; `Ctrl+C` quits cleanly; terminal resize rebuilds the world.

## Release

Releases are automatic on push to `main` via the "Publish" CI
(cocogitto: bump `Cargo.toml`/`.spec`, tag `vX.Y.Z`, trigger COPR). See
`docs/RELEASING.md`. Never reintroduce a version `sed` in `.copr/Makefile`:
the bump is done by the CI.

## Architecture

The hard rule is a **pure core / terminal shell split**. The library crate (`src/lib.rs` → `engine`, `environment`, `spawn`, `scene`, `art`) is pure Rust with no `crossterm` and no I/O, which is why the whole simulation is unit-testable without a terminal. `src/main.rs` (the binary) is the only `crossterm`-aware module. Preserve this: keep new simulation logic in the library modules, and never use `crossterm` outside `main.rs`.

**Frame pipeline** (`spawn::step`, once per frame at `FPS`):
1. `env.update(dt)` — advance the day/night clock + weather state machine *first*.
2. `world.advance(dt)` — build an `EnvSnapshot`, advance every entity (integrate + animate + run its behaviors), then cull dead/offscreen ones (firing `on_death`).
3. `spawner.tick(world, dt)` — top up populations toward their targets.

Then `world.render()` composites a `Buffer` and `Display::draw` paints only changed cells.

**Rendering / depth** (`engine.rs`): `composite` draws entities sorted by `depth` **descending** (painter's algorithm) with a stable sort. Depths are named `i32` consts (`DEPTH_SUN=90` … `DEPTH_FOREGROUND=30`). A `Buffer` holds parallel `chars` and `colors` grids. Spaces are transparent **except** when `entity.opaque` is true, where interior gaps overwrite the background (ground animals hiding grass).

**Entities & behaviors** (`engine::Entity`, factories in `scene.rs`): an `Entity` is `frames` + position/velocity + `behaviors: Vec<Behavior>`. `Behavior` is a data-oriented enum (`Fall`, `Hop`, `Orbit`, `Zigzag`, `Lifespan`, `EnvCull`) whose `apply` is the per-frame update; chaining = the order of the `Vec`. Behaviors read a `Copy` `EnvSnapshot` passed by value each frame (built by `World::env_snapshot`), which is how day/night gating works without borrow conflicts.

**Color masks** (asciiquarium-style): art in `art.rs` is pure data. A sprite can carry a `color_mask` (parallel to `frames`) where each char maps through `engine::mask_color` (`g`→green, `n`→brown, …). Masks align with frames char-for-char; `flip_horizontal` flips frame and mask together for entities entering from the right.

**Spawning** (`spawn::Spawner` + `scene::register_spawners`): each spec has a `target` (`Target::Fixed(i32)` or `Target::Dynamic(fn(&World)->i32)` for day/night/weather gating — returns 0 to drain a population) and a per-second `chance`. A factory returns `None` to decline (weather factories do this when the weather doesn't hold).

**Environment** (`environment.rs`): global clock + weather on `world.env`. `phase` runs 0→1 over `day_length`; first half day, second half night. Weather is a weighted state machine (`Clear`/`Wind`/`Rain`/`Storm`) with dwell timers. Each `Environment` owns its own seedable `StdRng`; `World` owns the master `StdRng`.

## Module map

- `engine.rs` — `Color`, `mask_color`, depth consts, `flip_horizontal`, `EnvSnapshot`, `Behavior`, `Entity`, `Buffer`, `composite`, `World` (pure).
- `scene.rs` — meadow construction, geometry helpers, `spawn_*` factories, `register_spawners`.
- `spawn.rs` — `Target`, `Spawner`, `step`.
- `environment.rs` — day/night + weather model.
- `art.rs` — ASCII art and color-mask data only, no logic.
- `main.rs` — arg parsing, crossterm display, main loop (the only crossterm-aware module).

## Design docs

`docs/superpowers/specs/` and `docs/superpowers/plans/` hold the design + implementation plan for each feature (dated). The Rust rewrite is `2026-07-06-rust-rewrite-design.md` / `2026-07-06-rust-rewrite.md`; earlier dated docs describe the original Python implementation and are kept as history.
