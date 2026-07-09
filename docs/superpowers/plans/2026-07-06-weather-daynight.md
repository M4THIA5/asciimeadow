# Weather & Day/Night Cycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an automatic day/night cycle (stars, moon, fireflies, night-only owl) and random bad weather (rain, wind, storm with bolt lightning) to asciimeadow.

**Architecture:** A pure `Environment` object owns a time-of-day clock plus a weather state machine. It lives on `world.env`, is ticked once per frame in `spawn.step`, and is read by spawners to gate what appears. Restricted entities self-despawn via an env-checking `update_fn`. `engine.py` stays weather-agnostic (one generic `env` slot).

**Tech Stack:** Python 3 stdlib only (`curses`, `random`, `math`). Tests with `pytest`.

## Global Constraints

- Python standard library only — no new dependencies.
- `engine.py` stays pure and curses-free; it must not import `environment` or reason about weather beyond holding a generic `env` reference.
- Every `X_MASK` must match its `X` frame grid line-for-line (enforced by `tests/test_art.py`).
- Comments in French, matching the existing codebase style.
- TDD: write the failing test first, watch it fail, implement, watch it pass, commit.
- Run the whole suite with `python -m pytest -q` before each commit.

## File Structure

- Create `asciimeadow/environment.py` — pure `Environment` (clock + weather machine).
- Create `tests/test_environment.py` — unit tests for `Environment`.
- Modify `asciimeadow/engine.py` — add `World.env = None` slot.
- Modify `asciimeadow/spawn.py` — resolve callable `target`; tick `world.env` in `step`.
- Modify `asciimeadow/art.py` — `MOON`, `LIGHTNING`, `STAR_CHARS`, `RAIN_CHARS`, `FIREFLY`, `WIND_CHARS`.
- Modify `asciimeadow/scene.py` — behaviors (`add_env_cull`, `make_lifespan`, `env_target`), day/night + weather spawners, `build_meadow` creates `world.env`, `register_spawners` rewrite.
- Modify `asciimeadow/__main__.py` — `--day-length` CLI, thread it into `build_meadow`.
- Modify `tests/test_engine.py`, `tests/test_spawn.py`, `tests/test_art.py`, `tests/test_scene.py`.

---

### Task 1: Environment (clock + weather machine)

**Files:**
- Create: `asciimeadow/environment.py`
- Test: `tests/test_environment.py`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `Environment(day_length: float = 90.0, rng=None)` — `rng` is a `random.Random`-like object; `None` ⇒ module `random`.
  - `env.update(dt: float) -> None`
  - properties: `phase -> float` (0..1), `is_night -> bool`, `raining -> bool`, `windy -> bool`, `storming -> bool`, `wind_dx -> float`.
  - attributes: `env.t: float`, `env.weather: str` (one of `"CLEAR"|"WIND"|"RAIN"|"STORM"`), `env.day_length: float`.

- [ ] **Step 1: Write the failing tests**

Create `tests/test_environment.py`:

```python
import random
from asciimeadow.environment import Environment


def test_phase_wraps_within_unit_interval():
    env = Environment(day_length=10.0)
    for _ in range(50):
        env.update(0.5)
        assert 0.0 <= env.phase < 1.0


def test_phase_returns_to_start_after_full_day():
    env = Environment(day_length=10.0)
    env.update(10.0)
    assert env.phase == 0.0


def test_is_night_after_half_cycle():
    env = Environment(day_length=10.0)
    assert env.is_night is False        # phase 0 -> jour
    env.update(5.0)                     # phase 0.5 -> nuit (borne incluse)
    assert env.is_night is True
    env.update(2.5)                     # phase 0.75 -> nuit
    assert env.is_night is True


def test_weather_sequence_is_deterministic_with_seeded_rng():
    def run():
        env = Environment(day_length=1000.0, rng=random.Random(42))
        seq = []
        for _ in range(2000):
            env.update(0.1)
            seq.append(env.weather)
        return seq
    assert run() == run()


def test_weather_states_are_valid_and_change():
    env = Environment(day_length=1000.0, rng=random.Random(1))
    seen = set()
    for _ in range(5000):
        env.update(0.1)
        seen.add(env.weather)
    assert seen <= {"CLEAR", "WIND", "RAIN", "STORM"}
    assert len(seen) >= 2               # la météo change dans le temps


def test_storm_sets_all_flags():
    env = Environment()
    env.weather = "STORM"
    assert env.raining and env.windy and env.storming


def test_wind_dx_zero_when_calm_nonzero_when_windy():
    env = Environment()
    env.weather = "CLEAR"
    assert env.wind_dx == 0
    env.weather = "WIND"
    assert env.wind_dx != 0
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python -m pytest tests/test_environment.py -q`
Expected: FAIL with `ModuleNotFoundError: No module named 'asciimeadow.environment'`.

- [ ] **Step 3: Write the implementation**

Create `asciimeadow/environment.py`:

```python
"""État global de l'environnement : horloge jour/nuit + machine météo (pur)."""
import random as _random

WEATHERS = ["CLEAR", "WIND", "RAIN", "STORM"]
_WEIGHTS = [5, 2, 2, 1]        # CLEAR le plus fréquent (temps calme par défaut)
_DWELL_MIN, _DWELL_MAX = 8.0, 20.0
_WIND_SPEED = 6.0


class Environment:
    def __init__(self, day_length=90.0, rng=None):
        self.day_length = float(day_length)
        self.rng = rng if rng is not None else _random
        self.t = 0.0
        self.weather = "CLEAR"
        self._weather_timer = self._new_dwell()

    def _new_dwell(self):
        return self.rng.uniform(_DWELL_MIN, _DWELL_MAX)

    def update(self, dt):
        self.t += dt
        self._weather_timer -= dt
        if self._weather_timer <= 0.0:
            self.weather = self.rng.choices(WEATHERS, weights=_WEIGHTS)[0]
            self._weather_timer = self._new_dwell()

    @property
    def phase(self):
        return (self.t / self.day_length) % 1.0

    @property
    def is_night(self):
        return self.phase >= 0.5

    @property
    def raining(self):
        return self.weather in ("RAIN", "STORM")

    @property
    def windy(self):
        return self.weather in ("WIND", "STORM")

    @property
    def storming(self):
        return self.weather == "STORM"

    @property
    def wind_dx(self):
        return _WIND_SPEED if self.windy else 0.0
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `python -m pytest tests/test_environment.py -q`
Expected: PASS (7 passed).

- [ ] **Step 5: Commit**

```bash
git add asciimeadow/environment.py tests/test_environment.py
git commit -m "feat: Environment clock + weather state machine"
```

---

### Task 2: World env slot + spawner dynamic target + env tick

**Files:**
- Modify: `asciimeadow/engine.py` (`World.__init__`)
- Modify: `asciimeadow/spawn.py` (`Spawner.tick`, `step`)
- Test: `tests/test_engine.py`, `tests/test_spawn.py`

**Interfaces:**
- Consumes: `Environment.update(dt)` from Task 1 (via any object exposing `.update`).
- Produces:
  - `World.env` attribute, defaults to `None`.
  - `Spawner.tick` resolves `spec.target` when it is callable: `spec.target(world) -> int`.
  - `step(world, spawner, dt)` calls `world.env.update(dt)` first when `world.env is not None`.

- [ ] **Step 1: Write the failing tests**

Append to `tests/test_engine.py`:

```python
def test_world_has_env_slot_defaulting_none():
    from asciimeadow.engine import World
    w = World(5, 5)
    assert w.env is None
```

Append to `tests/test_spawn.py`:

```python
def test_callable_target_is_resolved_and_caps_count():
    w = make_world()
    sp = Spawner()
    sp.register("thing",
                lambda world: Entity(frames=["x"], x=0, y=0, name="thing"),
                target=lambda world: 2, chance=1.0)
    for _ in range(50):
        sp.tick(w, dt=1.0)
    assert sum(1 for e in w.entities if e.name == "thing") == 2


def test_zero_callable_target_spawns_nothing():
    w = make_world()
    sp = Spawner()
    sp.register("thing",
                lambda world: Entity(frames=["x"], x=0, y=0, name="thing"),
                target=lambda world: 0, chance=1.0)
    for _ in range(50):
        sp.tick(w, dt=1.0)
    assert not any(e.name == "thing" for e in w.entities)


def test_step_updates_env_first():
    w = make_world()

    class FakeEnv:
        def __init__(self):
            self.updated = 0.0

        def update(self, dt):
            self.updated += dt

    w.env = FakeEnv()
    sp = Spawner()
    step(w, sp, dt=0.25)
    assert w.env.updated == 0.25
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python -m pytest tests/test_engine.py::test_world_has_env_slot_defaulting_none tests/test_spawn.py -q`
Expected: FAIL — `AttributeError: 'World' object has no attribute 'env'` and the callable target raising `TypeError` (target not callable-aware yet).

- [ ] **Step 3: Add the `env` slot to `World`**

In `asciimeadow/engine.py`, `World.__init__` (currently ends with `self.entities = []`):

```python
class World:
    def __init__(self, width, height):
        self.width = width
        self.height = height
        self.entities = []
        self.env = None        # état d'environnement (jour/nuit + météo), optionnel
```

- [ ] **Step 4: Resolve callable targets and tick env in `spawn.py`**

Replace the body of `Spawner.tick` and `step` in `asciimeadow/spawn.py`:

```python
    def tick(self, world, dt):
        for spec in self.specs:
            target = spec.target(world) if callable(spec.target) else spec.target
            count = sum(1 for e in world.entities if e.name == spec.name)
            if count < target and random.random() < spec.chance * dt:
                entity = spec.factory(world)
                if entity is not None:
                    world.add(entity)


def step(world, spawner, dt):
    if world.env is not None:
        world.env.update(dt)
    world.advance(dt)
    spawner.tick(world, dt)
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `python -m pytest tests/test_engine.py tests/test_spawn.py -q`
Expected: PASS (all, including the pre-existing `test_step_advances_then_spawns` which uses `world.env is None`).

- [ ] **Step 6: Commit**

```bash
git add asciimeadow/engine.py asciimeadow/spawn.py tests/test_engine.py tests/test_spawn.py
git commit -m "feat: world.env slot, callable spawn targets, env tick in step"
```

---

### Task 3: Night + weather art data

**Files:**
- Modify: `asciimeadow/art.py`
- Test: `tests/test_art.py`

**Interfaces:**
- Consumes: nothing.
- Produces (module attributes on `asciimeadow.art`):
  - `MOON: str`, `MOON_MASK: str`
  - `LIGHTNING: str`, `LIGHTNING_MASK: str`
  - `STAR_CHARS: list[str]`
  - `RAIN_CHARS: dict[str, str]` with keys `"still"`, `"right"`, `"left"`
  - `FIREFLY: list[str]` (2 frames)
  - `WIND_CHARS: list[str]`

- [ ] **Step 1: Write the failing test + bump the mask count**

In `tests/test_art.py`, change the final assertion of `test_every_mask_matches_its_frames`:

```python
    assert checked >= 11   # + moon, lightning
```

Append a new test to `tests/test_art.py`:

```python
def test_weather_and_night_art_present():
    assert isinstance(art.MOON, str) and isinstance(art.LIGHTNING, str)
    assert len(art.FIREFLY) == 2
    assert set(art.RAIN_CHARS) == {"still", "right", "left"}
    assert art.STAR_CHARS and art.WIND_CHARS
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python -m pytest tests/test_art.py -q`
Expected: FAIL — `AttributeError: module 'asciimeadow.art' has no attribute 'MOON'`, and `checked >= 11` failing.

- [ ] **Step 3: Add the art data**

Append to `asciimeadow/art.py`:

```python
# --- Cycle jour/nuit -------------------------------------------------------
MOON = "\n".join([
    r" __ ",
    r"(  )",
    r" ~~ ",
])
MOON_MASK = "\n".join([
    " ww ",
    "wwww",
    " ww ",
])
STAR_CHARS = [".", "*", "+", "'"]
FIREFLY = ["*", " "]        # clignote : glyphe puis vide (frame_rate faible)

# --- Météo -----------------------------------------------------------------
LIGHTNING = "\n".join([
    r"  /",
    r" / ",
    r"/__",
    r" / ",
    r"/  ",
])
LIGHTNING_MASK = "\n".join([
    "  y",
    " y ",
    "yyy",
    " y ",
    "y  ",
])
# glyphe de goutte selon le vent (droite/gauche/aucun)
RAIN_CHARS = {"still": "|", "right": "\\", "left": "/"}
WIND_CHARS = [",", "~", "'", "`"]
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `python -m pytest tests/test_art.py -q`
Expected: PASS (both tests).

- [ ] **Step 5: Commit**

```bash
git add asciimeadow/art.py tests/test_art.py
git commit -m "feat: moon, stars, firefly, rain, lightning, wind art"
```

---

### Task 4: Scene behaviors (env-cull, lifespan, env-target)

**Files:**
- Modify: `asciimeadow/scene.py` (add three helpers near the other `make_*` behaviors)
- Test: `tests/test_scene.py`

**Interfaces:**
- Consumes: `Entity` (has `update_fn`, `alive`), `World.env`.
- Produces:
  - `add_env_cull(entity, world, pred) -> entity` — wraps `entity.update_fn` so that after running the previous fn, the entity dies when `world.env is not None and not pred(world)`.
  - `make_lifespan(seconds) -> update_fn` — kills the entity after `seconds`.
  - `env_target(pred, count) -> callable(world) -> int` — returns `0` when `world.env is None or not pred(world.env)`, else `count(world)` if callable else `count`.

- [ ] **Step 1: Write the failing tests**

Append to `tests/test_scene.py`:

```python
def test_add_env_cull_kills_when_pred_false_and_runs_prev():
    from asciimeadow.engine import World, Entity
    w = World(20, 10)

    class Env:
        pass
    w.env = Env()
    ticks = {"n": 0}
    e = Entity(frames=["x"], x=0, y=0)
    e.update_fn = lambda ent, dt: ticks.__setitem__("n", ticks["n"] + 1)
    scene.add_env_cull(e, w, lambda world: False)   # pred faux => cull
    e.advance(0.1)
    assert ticks["n"] == 1        # l'ancien update_fn tourne toujours
    assert e.alive is False


def test_add_env_cull_survives_when_pred_true_or_env_none():
    from asciimeadow.engine import World, Entity
    w = World(20, 10)
    e = Entity(frames=["x"], x=0, y=0)
    scene.add_env_cull(e, w, lambda world: False)
    e.advance(0.1)
    assert e.alive is True         # env None => jamais cullé

    class Env:
        pass
    w.env = Env()
    e2 = Entity(frames=["x"], x=0, y=0)
    scene.add_env_cull(e2, w, lambda world: True)
    e2.advance(0.1)
    assert e2.alive is True


def test_make_lifespan_kills_after_duration():
    from asciimeadow.engine import Entity
    e = Entity(frames=["x"], x=0, y=0)
    e.update_fn = scene.make_lifespan(0.3)
    e.advance(0.2)
    assert e.alive is True
    e.advance(0.2)                 # cumul 0.4 > 0.3
    assert e.alive is False


def test_env_target_gates_on_pred_and_resolves_count():
    from asciimeadow.engine import World
    w = World(40, 10)
    t = scene.env_target(lambda env: env.flag, lambda world: world.width // 4)

    class Env:
        flag = False
    w.env = Env()
    assert t(w) == 0               # pred faux
    w.env.flag = True
    assert t(w) == 10              # 40 // 4
    w.env = None
    assert t(w) == 0               # pas d'env
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python -m pytest tests/test_scene.py -k "env_cull or lifespan or env_target" -q`
Expected: FAIL — `AttributeError: module 'asciimeadow.scene' has no attribute 'add_env_cull'`.

- [ ] **Step 3: Implement the helpers**

In `asciimeadow/scene.py`, add after `make_zigzag` (before `GROUND_ROWS`):

```python
def add_env_cull(entity, world, pred):
    """Enveloppe l'update_fn : l'entité meurt quand pred(world) devient faux.

    Le closure capture `world`, donc `world.env` est relu à chaque frame.
    Si `world.env is None`, l'entité n'est jamais cullée (sécurité pour les tests
    et le rendu hors scène)."""
    prev = entity.update_fn

    def update(e, dt):
        if prev is not None:
            prev(e, dt)
        if world.env is not None and not pred(world):
            e.alive = False

    entity.update_fn = update
    return entity


def make_lifespan(seconds):
    state = {"t": 0.0}

    def update(e, dt):
        state["t"] += dt
        if state["t"] >= seconds:
            e.alive = False
    return update


def env_target(pred, count):
    """Cible dynamique pour un spawner : 0 si l'env manque ou pred(env) faux,
    sinon `count` (int) ou `count(world)` (callable)."""
    def target(world):
        if world.env is None or not pred(world.env):
            return 0
        return count(world) if callable(count) else count
    return target
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `python -m pytest tests/test_scene.py -k "env_cull or lifespan or env_target" -q`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add asciimeadow/scene.py tests/test_scene.py
git commit -m "feat: scene behaviors add_env_cull, make_lifespan, env_target"
```

---

### Task 5: Day/night spawners + build_meadow env wiring

**Files:**
- Modify: `asciimeadow/scene.py` (`build_meadow`, new `spawn_sun`/`spawn_moon`/`spawn_star`/`spawn_firefly`, gate `spawn_owl`/`spawn_bee`/`spawn_butterfly`, rewrite `register_spawners`)
- Test: `tests/test_scene.py`

**Interfaces:**
- Consumes: `Environment` (Task 1), `env_target`/`add_env_cull` (Task 4), `art.MOON`/`STAR_CHARS`/`FIREFLY` (Task 3).
- Produces:
  - `build_meadow(world, day_length=90.0)` — now also sets `world.env = Environment(day_length=day_length)` and no longer adds a permanent `sun` entity.
  - `spawn_sun(world)`, `spawn_moon(world)`, `spawn_star(world)`, `spawn_firefly(world)` factories (each returns an `Entity`, night/day-culled).
  - `spawn_owl` night-culled; `spawn_bee`, `spawn_butterfly` day-culled.
  - `register_spawners` registers `sun`/`moon`/`star`/`firefly` and gates `owl`/`bee`/`butterfly` via `env_target`.

- [ ] **Step 1: Write the failing tests + fix pre-existing day/night assumptions**

In `tests/test_scene.py`, change `test_build_meadow_adds_scenery` — replace the sun assertion:

```python
def test_build_meadow_adds_scenery():
    from asciimeadow.engine import World
    w = World(60, 24)
    scene.build_meadow(w)
    names = {e.name for e in w.entities}
    assert w.env is not None          # l'environnement est créé
    assert "tree" in names
    assert "ground" in names
```

Replace `test_register_spawners_populates_world` (the sun/owl at t=0 assumption breaks now that owl is night-only):

```python
def test_register_spawners_populates_world():
    w = World(80, 24)
    scene.build_meadow(w)
    sp = Spawner()
    scene.register_spawners(sp)
    _random.seed(0)
    w.env.t = 0.0                     # jour : abeilles présentes
    for _ in range(200):
        sp.tick(w, dt=0.1)
    assert "bee" in {e.name for e in w.entities}
    w.env.t = w.env.day_length * 0.6  # nuit : hibou présent
    for _ in range(200):
        sp.tick(w, dt=0.1)
    assert "owl" in {e.name for e in w.entities}
```

Append the new day/night tests:

```python
def test_owl_target_only_at_night():
    w = World(80, 24)
    scene.build_meadow(w)
    sp = Spawner()
    scene.register_spawners(sp)
    spec = next(s for s in sp.specs if s.name == "owl")
    w.env.t = 0.0
    assert spec.target(w) == 0
    w.env.t = w.env.day_length * 0.6
    assert spec.target(w) == 1


def test_bee_and_butterfly_target_zero_at_night():
    w = World(80, 24)
    scene.build_meadow(w)
    sp = Spawner()
    scene.register_spawners(sp)
    w.env.t = w.env.day_length * 0.6      # nuit
    for name in ("bee", "butterfly"):
        spec = next(s for s in sp.specs if s.name == name)
        assert spec.target(w) == 0


def test_sun_and_moon_swap_day_night():
    w = World(80, 24)
    scene.build_meadow(w)
    sp = Spawner()
    scene.register_spawners(sp)
    sun = next(s for s in sp.specs if s.name == "sun")
    moon = next(s for s in sp.specs if s.name == "moon")
    w.env.t = 0.0
    assert sun.target(w) == 1 and moon.target(w) == 0
    w.env.t = w.env.day_length * 0.6
    assert sun.target(w) == 0 and moon.target(w) == 1


def test_env_cull_kills_owl_when_day_breaks():
    w = World(80, 24)
    scene.build_meadow(w)
    w.env.t = w.env.day_length * 0.6      # nuit
    owl = scene.spawn_owl(w)
    owl.advance(0.1)
    assert owl.alive is True
    w.env.t = 0.0                         # jour
    owl.advance(0.1)
    assert owl.alive is False


def test_stars_and_fireflies_are_night_only():
    w = World(80, 24)
    scene.build_meadow(w)
    sp = Spawner()
    scene.register_spawners(sp)
    for name in ("star", "firefly"):
        spec = next(s for s in sp.specs if s.name == name)
        w.env.t = 0.0
        assert spec.target(w) == 0
        w.env.t = w.env.day_length * 0.6
        assert spec.target(w) > 0
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python -m pytest tests/test_scene.py -q`
Expected: FAIL — the new tests error on missing `spawn_sun`/`spawn_moon`/etc. and the `owl`/`sun` specs not being gated (`target` still an int, so `spec.target(w)` raises `TypeError`).

- [ ] **Step 3: Import Environment and wire it into `build_meadow`**

In `asciimeadow/scene.py`, add to the imports at the top:

```python
from asciimeadow.environment import Environment
```

Replace `build_meadow` so it creates the env and drops the permanent sun (remove the sun `Entity(...)` block; keep tree, ground, flowers):

```python
def build_meadow(world, day_length=90.0):
    # Environnement : horloge jour/nuit + météo aléatoire
    world.env = Environment(day_length=day_length)
    # Arbre — centré, base au sol, variante selon la taille du terminal
    tf, tm = tree_art(world)
    tox, toy = tree_origin(world)
    world.add(Entity(frames=[tf], color_mask=[tm],
                     x=tox, y=toy, depth=engine.DEPTH_TREE,
                     color="green", name="tree"))
    # Sol — pleine largeur, herbe dense ondulante sur toute la bande,
    # derrière les animaux du sol (DEPTH_GRASS)
    world.add(Entity(frames=_ground_frames(world.width),
                     x=0, y=ground_top(world), depth=engine.DEPTH_GRASS,
                     frame_rate=2.0, color="green", name="ground"))
    # Fleurs — dispersées sur toute la bande d'herbe
    gy = ground_top(world)
    for _ in range(max(3, world.width // 12)):
        fx = random.randint(0, max(0, world.width - 1))
        fy = random.randint(gy, world.height - 1)
        flower = random.choice(art.FLOWERS)
        color = random.choice(["red", "yellow", "magenta", "white"])
        world.add(Entity(frames=[flower], x=fx, y=fy,
                         depth=engine.DEPTH_FOREGROUND, color=color,
                         name="flower"))
```

- [ ] **Step 4: Add the sky-body + night-creature factories**

In `asciimeadow/scene.py`, add near `spawn_bird` / `spawn_cloud`:

```python
def spawn_sun(world):
    sun_w = max(len(l) for l in art.SUN.split("\n"))
    e = Entity(frames=[art.SUN], color_mask=[art.SUN_MASK],
               x=world.width - sun_w - 1, y=0,
               depth=engine.DEPTH_SUN, color="yellow", name="sun")
    return add_env_cull(e, world, lambda w: not w.env.is_night)


def spawn_moon(world):
    moon_w = max(len(l) for l in art.MOON.split("\n"))
    e = Entity(frames=[art.MOON], color_mask=[art.MOON_MASK],
               x=world.width - moon_w - 1, y=0,
               depth=engine.DEPTH_SUN, color="white", name="moon")
    return add_env_cull(e, world, lambda w: w.env.is_night)


def spawn_star(world):
    x = random.randint(0, max(0, world.width - 1))
    y = random.randint(0, max(0, world.height // 3))
    glyph = random.choice(art.STAR_CHARS)
    color = random.choice(["white", "yellow"])
    e = Entity(frames=[glyph], x=x, y=y, depth=engine.DEPTH_SUN,
               color=color, name="star")
    return add_env_cull(e, world, lambda w: w.env.is_night)


def spawn_firefly(world):
    tox, toy = tree_origin(world)
    gy = ground_top(world)
    x = random.randint(max(0, tox - 4), min(max(0, world.width - 1), tox + 20))
    y = random.randint(max(1, toy + 2), max(1, gy - 1))
    e = Entity(frames=art.FIREFLY, x=x, y=y, depth=engine.DEPTH_FOREGROUND,
               frame_rate=random.uniform(2.0, 4.0), color="yellow",
               name="firefly")
    e.update_fn = make_zigzag(top=max(1, toy), bottom=max(2, gy - 1),
                              vy=random.uniform(1.0, 2.5))
    return add_env_cull(e, world, lambda w: w.env.is_night)
```

- [ ] **Step 5: Gate the owl, bee, and butterfly**

In `spawn_owl`, replace the `return Entity(...)` at the end so the entity is captured and night-culled:

```python
def spawn_owl(world):
    tox, toy = tree_origin(world)
    tf, _ = tree_art(world)
    lines = tf.split("\n")
    tree_w = max(len(l) for l in lines)
    tree_h = len(lines)
    owl_w = max(len(l) for l in art.OWL[0].split("\n"))
    x = tox + tree_w // 2 - owl_w // 2
    y = toy + max(1, tree_h // 2 - 3)   # centré dans la canopée
    e = Entity(frames=art.OWL, x=x, y=y, color_mask=art.OWL_MASK,
               depth=engine.DEPTH_TREE_CREATURE, frame_rate=0.4,
               color="brown", name="owl")
    return add_env_cull(e, world, lambda w: w.env.is_night)
```

In `spawn_bee`, add a day-cull before `return e`:

```python
    e.update_fn = make_orbit(cx=cx, cy=cy,
                             radius=random.uniform(3, 6),
                             ang_speed=random.uniform(2, 4))
    return add_env_cull(e, world, lambda w: not w.env.is_night)
```

In `spawn_butterfly`, replace its final `return e`:

```python
    e.update_fn = make_zigzag(top=1, bottom=ground_top(world) - 1,
                              vy=random.uniform(3, 6))
    return add_env_cull(e, world, lambda w: not w.env.is_night)
```

- [ ] **Step 6: Rewrite `register_spawners` (day/night entries; weather added in Task 6)**

Replace `register_spawners` in `asciimeadow/scene.py`:

```python
def register_spawners(spawner):
    # Corps célestes : bascule jour/nuit
    spawner.register("sun", spawn_sun,
                     target=env_target(lambda e: not e.is_night, 1), chance=1.0)
    spawner.register("moon", spawn_moon,
                     target=env_target(lambda e: e.is_night, 1), chance=1.0)
    spawner.register("star", spawn_star,
                     target=env_target(lambda e: e.is_night,
                                       lambda w: w.width // 8), chance=8.0)
    spawner.register("firefly", spawn_firefly,
                     target=env_target(lambda e: e.is_night, 6), chance=2.0)
    # Résidents de l'arbre
    spawner.register("owl", spawn_owl,
                     target=env_target(lambda e: e.is_night, 1), chance=1.0)
    spawner.register("bee", spawn_bee,
                     target=env_target(lambda e: not e.is_night, 3), chance=0.8)
    # Ciel
    spawner.register("cloud", spawn_cloud, target=3, chance=0.3)
    spawner.register("bird", spawn_bird, target=5, chance=0.5)
    spawner.register("butterfly", spawn_butterfly,
                     target=env_target(lambda e: not e.is_night, 3), chance=0.4)
    # Objets qui tombent
    spawner.register("apple", spawn_apple, target=2, chance=0.2)
    # Sol
    spawner.register("rabbit", spawn_rabbit, target=2, chance=0.3)
    spawner.register("fox", spawn_fox, target=1, chance=0.1)
    spawner.register("hedgehog", spawn_hedgehog, target=1, chance=0.15)
    spawner.register("mouse", spawn_mouse, target=2, chance=0.3)
    spawner.register("snail", spawn_snail, target=1, chance=0.1)
```

- [ ] **Step 7: Run the full suite to verify it passes**

Run: `python -m pytest -q`
Expected: PASS (all tests, including the rewritten day/night ones).

- [ ] **Step 8: Commit**

```bash
git add asciimeadow/scene.py tests/test_scene.py
git commit -m "feat: day/night spawners (sun/moon/stars/fireflies), night-only owl"
```

---

### Task 6: Weather spawners (rain, wind leaves, lightning)

**Files:**
- Modify: `asciimeadow/scene.py` (new `spawn_rain`/`spawn_windleaf`/`spawn_lightning`, register them)
- Test: `tests/test_scene.py`

**Interfaces:**
- Consumes: `env_target`/`make_lifespan` (Task 4), `art.RAIN_CHARS`/`WIND_CHARS`/`LIGHTNING` (Task 3), `Environment.wind_dx`/`raining`/`windy`/`storming` (Task 1).
- Produces: `spawn_rain(world)`, `spawn_windleaf(world)`, `spawn_lightning(world)` factories, plus `rain`/`windleaf`/`lightning` spawner registrations gated by weather.

- [ ] **Step 1: Write the failing tests**

Append to `tests/test_scene.py`:

```python
def test_rain_target_scales_with_weather():
    w = World(80, 24)
    scene.build_meadow(w)
    sp = Spawner()
    scene.register_spawners(sp)
    spec = next(s for s in sp.specs if s.name == "rain")
    w.env.weather = "CLEAR"
    assert spec.target(w) == 0
    w.env.weather = "RAIN"
    assert spec.target(w) == 80 // 4
    w.env.weather = "STORM"
    assert spec.target(w) == 80 // 2


def test_lightning_target_only_when_storming():
    w = World(80, 24)
    scene.build_meadow(w)
    sp = Spawner()
    scene.register_spawners(sp)
    spec = next(s for s in sp.specs if s.name == "lightning")
    w.env.weather = "RAIN"
    assert spec.target(w) == 0
    w.env.weather = "STORM"
    assert spec.target(w) == 1


def test_windleaf_target_only_when_windy_and_dry():
    w = World(80, 24)
    scene.build_meadow(w)
    sp = Spawner()
    scene.register_spawners(sp)
    spec = next(s for s in sp.specs if s.name == "windleaf")
    w.env.weather = "WIND"
    assert spec.target(w) == 4
    w.env.weather = "STORM"          # venteux mais pluvieux -> pas de feuilles
    assert spec.target(w) == 0


def test_lightning_bolt_is_masked_and_dies():
    w = World(80, 24)
    scene.build_meadow(w)
    w.env.weather = "STORM"
    bolt = scene.spawn_lightning(w)
    assert bolt.color_mask is not None
    for _ in range(20):
        bolt.advance(0.1)            # 2 s >> durée de vie max (0.4 s)
    assert bolt.alive is False


def test_raindrop_falls_and_uses_wind_slant():
    w = World(80, 24)
    scene.build_meadow(w)
    w.env.weather = "STORM"          # windy -> wind_dx != 0
    drop = scene.spawn_rain(w)
    assert drop.dy > 0
    assert drop.dx == w.env.wind_dx
    assert drop.name == "rain"
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python -m pytest tests/test_scene.py -k "rain or lightning or windleaf" -q`
Expected: FAIL — `AttributeError: module 'asciimeadow.scene' has no attribute 'spawn_rain'`.

- [ ] **Step 3: Implement the weather factories**

In `asciimeadow/scene.py`, add after `spawn_firefly`:

```python
def spawn_rain(world):
    x = random.randint(0, max(0, world.width - 1))
    wind = world.env.wind_dx if world.env else 0.0
    if wind > 0:
        glyph = art.RAIN_CHARS["right"]
    elif wind < 0:
        glyph = art.RAIN_CHARS["left"]
    else:
        glyph = art.RAIN_CHARS["still"]
    return Entity(frames=[glyph], x=x, y=0,
                  dy=random.uniform(18.0, 26.0), dx=wind,
                  depth=engine.DEPTH_FOREGROUND, color="cyan", name="rain")


def spawn_windleaf(world):
    glyph = random.choice(art.WIND_CHARS)
    y = random.randint(1, max(2, world.height // 2))
    speed = random.uniform(10.0, 18.0)
    wind = world.env.wind_dx if world.env else 1.0
    dx = speed if wind >= 0 else -speed
    x = -1 if dx > 0 else world.width
    return Entity(frames=[glyph], x=x, y=y, dx=dx,
                  depth=engine.DEPTH_SKY_CREATURE, color="green",
                  name="windleaf")


def spawn_lightning(world):
    bolt_w = max(len(l) for l in art.LIGHTNING.split("\n"))
    x = random.randint(0, max(0, world.width - bolt_w))
    e = Entity(frames=[art.LIGHTNING], color_mask=[art.LIGHTNING_MASK],
               x=x, y=1, depth=engine.DEPTH_FOREGROUND,
               color="yellow", name="lightning")
    e.update_fn = make_lifespan(random.uniform(0.2, 0.4))
    return e
```

- [ ] **Step 4: Register the weather spawners**

In `register_spawners`, add a `# Météo` block immediately before the `# Objets qui tombent` line:

```python
    # Météo
    spawner.register("rain", spawn_rain,
                     target=env_target(lambda e: e.raining,
                                       lambda w: w.width // 2 if w.env.storming
                                       else w.width // 4),
                     chance=30.0)
    spawner.register("windleaf", spawn_windleaf,
                     target=env_target(lambda e: e.windy and not e.raining, 4),
                     chance=3.0)
    spawner.register("lightning", spawn_lightning,
                     target=env_target(lambda e: e.storming, 1), chance=1.5)
```

- [ ] **Step 5: Run the full suite to verify it passes**

Run: `python -m pytest -q`
Expected: PASS (all).

- [ ] **Step 6: Commit**

```bash
git add asciimeadow/scene.py tests/test_scene.py
git commit -m "feat: weather spawners (rain, wind leaves, bolt lightning)"
```

---

### Task 7: CLI --day-length + headless integration smoke

**Files:**
- Modify: `asciimeadow/__main__.py` (`run` signature, both `build_meadow` calls, `--day-length` arg)
- Test: `tests/test_scene.py`

**Interfaces:**
- Consumes: `build_meadow(world, day_length=...)` (Task 5), `step` (Task 2), `register_spawners` (Tasks 5–6).
- Produces: `--day-length SEC` CLI flag (default `90.0`); `run(stdscr, seed=None, day_length=90.0)`.

- [ ] **Step 1: Write the failing integration test**

Append to `tests/test_scene.py`:

```python
def test_full_day_night_weather_cycle_runs_headless():
    from asciimeadow.spawn import step
    _random.seed(7)
    w = World(80, 24)
    scene.build_meadow(w, day_length=20.0)   # cycles rapides
    sp = Spawner()
    scene.register_spawners(sp)
    seen = set()
    for _ in range(2000):                    # 200 s => plusieurs jours + météos
        step(w, sp, 0.1)
        seen |= {e.name for e in w.entities}
    assert "sun" in seen or "moon" in seen
    assert "owl" in seen                     # la nuit a été atteinte
    w.render()                               # le rendu ne lève pas
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `python -m pytest tests/test_scene.py::test_full_day_night_weather_cycle_runs_headless -q`
Expected: FAIL — `build_meadow()` currently rejects the `day_length` keyword (`TypeError`) only if Task 5 is not present; if Task 5 is present this test should already pass except that we still want the CLI plumbing. If it already passes, keep it and proceed — the failing target for this task is the CLI flag exercised in Step 4.

- [ ] **Step 3: Add the `--day-length` flag and thread it through `run`**

In `asciimeadow/__main__.py`, update `run` to accept and use `day_length`:

```python
def run(stdscr, seed=None, day_length=90.0):
    import curses
    if seed is not None:
        random.seed(seed)
    disp = CursesDisplay(stdscr)
    stdscr.timeout(int(1000 / FPS))
    w, h = disp.size()
    world = World(w, h)
    scene.build_meadow(world, day_length=day_length)
    spawner = Spawner()
    scene.register_spawners(spawner)
    dt = 1.0 / FPS
    paused = False
    while True:
        ch = stdscr.getch()
        if ch in (ord("q"), ord("Q")):
            break
        elif ch in (ord("p"), ord("P")):
            paused = not paused
        elif ch in (ord("r"), ord("R")):
            stdscr.clear()
        elif ch == curses.KEY_RESIZE:
            w, h = disp.size()
            world = World(w, h)
            scene.build_meadow(world, day_length=day_length)
            spawner = Spawner()
            scene.register_spawners(spawner)
            stdscr.clear()
        if not paused:
            step(world, spawner, dt)
        stdscr.erase()
        disp.draw(world.render())
```

In `main`, add the argument and pass it to the wrapper:

```python
    parser.add_argument("--day-length", type=float, default=90.0,
                        help="durée d'un cycle jour/nuit complet (s)")
    args = parser.parse_args()
    if args.fps < 1:
        parser.error("--fps must be >= 1")
    if args.day_length <= 0:
        parser.error("--day-length must be > 0")
    FPS = args.fps
    curses.wrapper(run, seed=args.seed, day_length=args.day_length)
```

- [ ] **Step 4: Verify the CLI parses and the suite passes**

Run: `python -m asciimeadow --help`
Expected: help text lists `--day-length`.

Run: `python -m pytest -q`
Expected: PASS (all).

- [ ] **Step 5: Commit**

```bash
git add asciimeadow/__main__.py tests/test_scene.py
git commit -m "feat: --day-length CLI flag + headless cycle smoke test"
```

---

## Self-Review

**Spec coverage:**
- Auto day/night cycle → Task 1 (`phase`/`is_night`), Task 5 (sun/moon/stars swap).
- Random weather (CLEAR/WIND/RAIN/STORM) → Task 1 weather machine, Task 6 spawners.
- Stars at night → Task 3 art, Task 5 `spawn_star`.
- Owl only at night → Task 5 (`env_target` night + `add_env_cull`).
- Fireflies at night → Task 3 art, Task 5 `spawn_firefly`.
- Rain (slanted by wind) → Task 6 `spawn_rain` using `wind_dx`.
- Wind (blowing leaves) → Task 6 `spawn_windleaf`.
- Storm = rain+wind+bolt lightning → Task 1 `storming` flags, Task 6 `spawn_lightning`.
- Additive only, no palette change → no engine draw change; only `World.env` slot (Task 2).
- Bolt lightning, no flash → Task 6 (sprite + `make_lifespan`).
- Auto control, no keyboard → env ticks in `step` (Task 2); no key handling added.
- `--day-length` config → Task 7.

**Placeholder scan:** none — every code step is complete.

**Type consistency:** `env_target(pred, count)` predicate receives `env` (checked via `pred(world.env)`); `add_env_cull(entity, world, pred)` predicate receives `world` (checked via `pred(world)`) — intentional and consistent within each helper's tests and call sites. `build_meadow(world, day_length=90.0)`, `run(stdscr, seed, day_length)`, and `Environment(day_length=...)` all agree. `world.env.weather` is a settable `str` attribute used by tests. Sprite/mask names (`MOON`/`MOON_MASK`, `LIGHTNING`/`LIGHTNING_MASK`) match Task 3.
