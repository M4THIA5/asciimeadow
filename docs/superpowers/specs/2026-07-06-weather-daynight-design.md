# Weather & Day/Night Cycle — Design

Date: 2026-07-06

## Goal

Add ambient bad weather (rain, storm, wind) and an automatic day/night cycle to
asciimeadow. At night: stars appear, the sun is replaced by a moon, fireflies
drift near the tree, and the owl appears only at night. Day-only creatures (bees,
butterflies) fade out at night. Everything advances automatically — no input
required — with weather changing at random.

## Decisions (locked during brainstorming)

- **Control model:** automatic. Day/night advances on a timer; weather shifts at
  random. No keyboard control added.
- **Night visuals:** additive only. New night elements are drawn on top; no global
  palette dimming or tinting. Engine palette is untouched.
- **Lightning:** a bolt sprite that flashes in the sky. No full-screen flash.

## Architecture (Approach A)

A pure `Environment` object owns the global time-of-day clock and the weather
state machine. It is created by the scene, held on `world.env`, and ticked once
per frame in `spawn.step`. Spawners read `world.env` to gate what appears.
`engine.py` stays weather-agnostic (one generic `env` slot only). This mirrors the
existing split: pure `engine` vs. `scene`/`spawn` domain logic.

Rejected alternatives:

- **B — module-level globals in `scene.py`:** not isolatable, untestable, breaks
  the pure-module pattern.
- **C — controller `Entity`:** overloads `Entity` (non-drawable), hides state
  inside a sprite, awkward to query "is it night".

## Components

### 1. `asciimeadow/environment.py` (new, pure — no curses)

```python
class Environment:
    def __init__(self, day_length=90.0, rng=random):
        # day_length: seconds for a full day+night cycle
        # rng: injectable for deterministic tests; defaults to module `random`
        ...
    def update(self, dt): ...        # advance clock + weather timer

    # time of day
    @property
    def phase(self):    ...          # (t / day_length) % 1.0  -> 0..1
    @property
    def is_night(self): ...          # phase >= 0.5 (day = first half)

    # weather queries
    @property
    def raining(self):  ...          # weather in {RAIN, STORM}
    @property
    def windy(self):    ...          # weather in {WIND, STORM}
    @property
    def storming(self): ...          # weather == STORM
    @property
    def wind_dx(self):  ...          # signed slant magnitude when windy else 0
```

**Weather state machine.** States: `CLEAR`, `WIND`, `RAIN`, `STORM`. Each
`update(dt)` decrements a `weather_timer`; on expiry it picks the next state via a
weighted random choice (`CLEAR` weighted highest so calm is the default mood) and
a fresh random dwell duration in `8.0–20.0` s. `STORM` implies rain **and** wind
**and** lightning. All randomness goes through `self.rng`, so a seeded rng gives a
deterministic sequence. The default `rng=random` is already seeded by `--seed`.

**Time of day.** `phase` is a continuous `0..1` clock. `is_night` is binary
(`phase >= 0.5`); no twilight gradient. `day_length` default `90.0` s.

### 2. `engine.py` (minimal touch)

`World.__init__` gains `self.env = None` — a generic slot. No other engine change;
the engine never imports or reasons about weather.

### 3. `spawn.py`

- `step(world, spawner, dt)`: `if world.env: world.env.update(dt)` →
  `world.advance(dt)` → `spawner.tick(world, dt)`. Env ticks **first** so spawners
  observe the current state this frame.
- `SpawnSpec.target` may now be an `int` **or** a `callable(world) -> int`.
  `Spawner.tick` resolves it: `target = spec.target(world) if callable(spec.target)
  else spec.target`. This is the only change to `Spawner`. `chance` is unchanged.

### 4. `scene.py` — two new behaviors

- `make_env_cull(world, pred)` → returns an `update_fn` that sets
  `e.alive = False` when `pred(world)` is false. The closure captures `world`, so
  it reads live `world.env` each frame. Used by day/night-restricted entities.
- `make_lifespan(seconds)` → returns an `update_fn` that kills the entity after N
  seconds. Used by lightning bolts and blowing leaves.

Gating pattern for restricted entities = **dynamic target** (controls spawning) +
**`make_env_cull`** (controls despawning when the condition flips).

## Feature behavior

### Day/night elements (additive; each env-culled when its window ends)

| entity | appears when | target | notes |
|---|---|---|---|
| sun | day | `1` | moved out of `build_meadow` into a spawner, day-gated |
| moon | night | `1` | new `MOON`/`MOON_MASK` crescent, top-right (sun's slot) |
| stars | night | `~width // 8` | scattered in upper sky, static glyphs `. * + '`, white/yellow |
| fireflies | night | `4–8` | near tree/ground, yellow, blink via 2-frame `["*", " "]`, gentle zigzag drift |
| owl | night | `1` | was always-on; now night-gated + culled at day |
| bees | day | as today | now day-gated + culled at night |
| butterflies | day | as today | now day-gated + culled at night |

### Weather elements

| entity | active when | behavior |
|---|---|---|
| rain | `raining` | many falling glyph entities; `dy` down, `dx = env.wind_dx` slant; glyph chosen from `RAIN_CHARS` by wind; cyan/blue. Auto-culled offscreen (existing `World._offscreen`, `dy>0`). Count + `chance` scale up from `RAIN` to `STORM` |
| wind leaves | `windy` and **not** `raining` | a few `, ~ '` glyphs blowing across fast; `make_lifespan` |
| lightning | `storming` | `LIGHTNING` bolt sprite at random sky x; `make_lifespan(~0.3 s)`; target `0–1`; yellow. Bolt only — no flash |

Rain stopping needs no explicit despawn: target → 0, in-flight drops finish
falling and leave the screen naturally.

### 5. `art.py` — new data

`MOON` + `MOON_MASK`, `LIGHTNING` + `LIGHTNING_MASK`, `STAR_CHARS`, `RAIN_CHARS`,
`FIREFLY` frames, `WIND_CHARS`. All colors used (cyan, blue, yellow, white) already
exist in `engine.MASK_COLORS`.

### 6. CLI (`__main__.py`)

Add `--day-length SEC` (default `90`). `build_meadow` constructs
`Environment(day_length=...)` and assigns `world.env`. `--seed` already seeds
module `random`, making the weather sequence deterministic.

## Data flow (per frame)

```
step(world, spawner, dt)
  └─ world.env.update(dt)          # advance clock + weather
  └─ world.advance(dt)             # move entities, cull offscreen/dead
  └─ spawner.tick(world, dt)       # resolve dynamic targets vs world.env, spawn
```

Restricted entities self-despawn via their `make_env_cull` update_fn during
`world.advance`.

## Testing

- **`tests/test_environment.py` (new):**
  - `phase` wraps within `0..1`; advancing by `day_length` returns to start.
  - `is_night` boundary at `phase == 0.5`.
  - Seeded rng ⇒ deterministic weather transition sequence.
  - `raining` / `windy` / `storming` truth per state; `STORM` sets all three.
  - `wind_dx` is `0` when not windy, nonzero (signed) when windy.
- **`tests/test_scene.py` (extend):**
  - owl target `0` under a day env, `1` under a night env.
  - bees + butterflies target `0` under a night env.
  - rain spawns only when `env.raining`.
  - `make_env_cull` kills the owl when env flips to day.
  - sun present + moon absent by day; moon present + sun absent at night.
  - Fix existing assertions that assume the sun is a permanent `build_meadow`
    entity.
- **`tests/test_spawn.py` (extend):**
  - callable `target` resolves through `tick`.
  - count gating: no spawn when resolved target is `0`.

## Scope guard (YAGNI — explicitly out)

- No twilight/dawn gradient (binary day/night).
- No global palette dimming or tinting.
- No full-screen lightning flash.
- No puddles, snow, or weather accumulation.
- No keyboard control of time/weather.
