# asciimeadow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Construire un économiseur d'écran terminal type asciiquarium, décor de prairie (ciel, arbre central, sol) avec créatures animées qui apparaissent en continu.

**Architecture:** Moteur pur sans curses (`Entity`, `Buffer`, `World`, compositor z-order) testable headless ; données ASCII isolées dans `art.py` ; assemblage de la scène + comportements dans `scene.py` ; population dans `spawn.py` ; I/O curses + boucle dans `__main__.py`.

**Tech Stack:** Python 3.8+, bibliothèque standard `curses`, `pytest` pour les tests. Aucune dépendance externe runtime.

## Global Constraints

- Python 3.8+ uniquement, **zéro dépendance runtime** (seul `curses` stdlib).
- `pytest` est la seule dépendance de dev.
- Le moteur (`engine.py`) et `spawn.py` ne doivent **jamais** importer `curses` — ils restent testables headless.
- Le caractère espace `' '` est toujours transparent au rendu.
- Convention de profondeur : **valeur plus grande = plus loin**, dessinée en premier ; plus petite = premier plan.
- Couleurs référencées par nom (str) ; ensemble canonique défini dans `engine.COLOR_NAMES`.
- Commits fréquents, un par tâche minimum, format Conventional Commits.

---

## File Structure

```
asciimeadow/
  __init__.py      vide (marque le package)
  engine.py        Entity, Buffer, World, composite, flip_horizontal, constantes depth/couleur
  art.py           constantes ASCII + masques couleur (données seules)
  spawn.py         SpawnSpec, Spawner, step()
  scene.py         build_meadow(), behaviors (make_*), factories, register_spawners()
  __main__.py      CursesDisplay, run(), boucle principale, CLI
tests/
  test_engine.py
  test_spawn.py
  test_scene.py
pyproject.toml     config pytest minimale
```

---

### Task 1: Scaffold + Entity (mouvement & animation)

**Files:**
- Create: `asciimeadow/__init__.py`
- Create: `asciimeadow/engine.py`
- Create: `tests/test_engine.py`
- Create: `pyproject.toml`

**Interfaces:**
- Produces:
  - `engine.COLOR_NAMES: list[str]`
  - `engine.MASK_COLORS: dict[str, str]`
  - `engine.DEPTH_SUN, DEPTH_CLOUD, DEPTH_SKY_CREATURE, DEPTH_TREE, DEPTH_TREE_CREATURE, DEPTH_GROUND_ANIMAL, DEPTH_FOREGROUND: int`
  - `engine.Entity(frames, x, y, dx=0.0, dy=0.0, depth=0, frame_rate=0.0, color="white", color_mask=None, on_death=None, name=None, update_fn=None)`
  - `Entity.advance(dt)`, `Entity.current_frame() -> str`, `Entity.current_mask() -> str | None`, `Entity.width() -> int`, `Entity.height() -> int`, attrs `.x .y .dx .dy .depth .alive .name`

- [ ] **Step 1: Write `pyproject.toml`**

```toml
[tool.pytest.ini_options]
testpaths = ["tests"]
```

- [ ] **Step 2: Create empty package marker**

`asciimeadow/__init__.py` — empty file.

- [ ] **Step 3: Write the failing tests**

`tests/test_engine.py`:

```python
from asciimeadow.engine import Entity


def test_entity_moves_by_velocity():
    e = Entity(frames=["x"], x=0.0, y=0.0, dx=2.0, dy=-1.0)
    e.advance(0.5)
    assert e.x == 1.0
    assert e.y == -0.5


def test_entity_animation_advances_frames():
    e = Entity(frames=["A", "B"], x=0, y=0, frame_rate=2.0)  # 2 fps => 0.5s/frame
    assert e.current_frame() == "A"
    e.advance(0.5)
    assert e.current_frame() == "B"
    e.advance(0.5)
    assert e.current_frame() == "A"  # wraps


def test_entity_single_frame_does_not_animate():
    e = Entity(frames=["A"], x=0, y=0, frame_rate=5.0)
    e.advance(10.0)
    assert e.current_frame() == "A"


def test_entity_dimensions():
    e = Entity(frames=["abc\nde"], x=0, y=0)
    assert e.width() == 3
    assert e.height() == 2


def test_entity_update_fn_runs_after_integration():
    seen = {}

    def upd(ent, dt):
        seen["x"] = ent.x  # x already integrated

    e = Entity(frames=["x"], x=0.0, y=0.0, dx=4.0, update_fn=upd)
    e.advance(1.0)
    assert seen["x"] == 4.0
```

- [ ] **Step 4: Run tests, verify they fail**

Run: `python -m pytest tests/test_engine.py -q`
Expected: FAIL (`ModuleNotFoundError` / `Entity` undefined).

- [ ] **Step 5: Implement `engine.py` (Entity + constants)**

`asciimeadow/engine.py`:

```python
"""Moteur pur (sans curses) : entités, buffer, compositor."""

COLOR_NAMES = [
    "white", "green", "brown", "yellow",
    "red", "cyan", "blue", "magenta", "black",
]

# Caractère de masque -> nom de couleur (style asciiquarium).
MASK_COLORS = {
    "w": "white", "g": "green", "n": "brown", "y": "yellow",
    "r": "red", "c": "cyan", "b": "blue", "m": "magenta", "k": "black",
}

# Profondeur : plus grand = plus loin (dessiné en premier).
DEPTH_SUN = 90
DEPTH_CLOUD = 80
DEPTH_SKY_CREATURE = 70
DEPTH_TREE = 60
DEPTH_TREE_CREATURE = 50
DEPTH_GROUND_ANIMAL = 40
DEPTH_FOREGROUND = 30


class Entity:
    def __init__(self, frames, x, y, dx=0.0, dy=0.0, depth=0,
                 frame_rate=0.0, color="white", color_mask=None,
                 on_death=None, name=None, update_fn=None):
        self.frames = frames
        self.x = float(x)
        self.y = float(y)
        self.dx = float(dx)
        self.dy = float(dy)
        self.depth = depth
        self.frame_rate = frame_rate
        self.color = color
        self.color_mask = color_mask  # list[str] parallèle à frames, ou None
        self.on_death = on_death
        self.name = name
        self.update_fn = update_fn
        self.alive = True
        self._frame_idx = 0
        self._anim_accum = 0.0

    def current_frame(self):
        return self.frames[self._frame_idx]

    def current_mask(self):
        if not self.color_mask:
            return None
        return self.color_mask[self._frame_idx]

    def height(self):
        return len(self.current_frame().split("\n"))

    def width(self):
        return max((len(line) for line in self.current_frame().split("\n")),
                   default=0)

    def advance(self, dt):
        self.x += self.dx * dt
        self.y += self.dy * dt
        if self.frame_rate > 0 and len(self.frames) > 1:
            self._anim_accum += dt
            step = 1.0 / self.frame_rate
            while self._anim_accum >= step:
                self._anim_accum -= step
                self._frame_idx = (self._frame_idx + 1) % len(self.frames)
        if self.update_fn:
            self.update_fn(self, dt)
```

- [ ] **Step 6: Run tests, verify pass**

Run: `python -m pytest tests/test_engine.py -q`
Expected: PASS (5 tests).

- [ ] **Step 7: Commit**

```bash
git add asciimeadow/__init__.py asciimeadow/engine.py tests/test_engine.py pyproject.toml
git commit -m "feat: entity model with movement and animation"
```

---

### Task 2: `flip_horizontal`

**Files:**
- Modify: `asciimeadow/engine.py`
- Modify: `tests/test_engine.py`

**Interfaces:**
- Produces: `engine.flip_horizontal(frame: str) -> str` (miroir gauche/droite, échange les caractères directionnels).

- [ ] **Step 1: Write failing test (append to `tests/test_engine.py`)**

```python
from asciimeadow.engine import flip_horizontal


def test_flip_reverses_and_swaps_chars():
    assert flip_horizontal("<o--") == "--o>"
    assert flip_horizontal("(_)") == "(_)"  # symétrique


def test_flip_multiline_each_line():
    assert flip_horizontal("ab\ncd") == "ba\ndc"
```

- [ ] **Step 2: Run, verify fail**

Run: `python -m pytest tests/test_engine.py -q`
Expected: FAIL (`flip_horizontal` undefined).

- [ ] **Step 3: Implement (append to `engine.py`)**

```python
_FLIP_TABLE = str.maketrans("<>[](){}/\\bdpq", "><][)(}{\\/dbqp")


def flip_horizontal(frame):
    return "\n".join(
        line[::-1].translate(_FLIP_TABLE) for line in frame.split("\n")
    )
```

- [ ] **Step 4: Run, verify pass**

Run: `python -m pytest tests/test_engine.py -q`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add asciimeadow/engine.py tests/test_engine.py
git commit -m "feat: horizontal flip helper for mirrored sprites"
```

---

### Task 3: `Buffer` + `draw_entity` (transparence, clipping, masque couleur)

**Files:**
- Modify: `asciimeadow/engine.py`
- Modify: `tests/test_engine.py`

**Interfaces:**
- Produces:
  - `engine.Buffer(width: int, height: int)` avec attrs `.width .height .chars (list[list[str]]) .colors (list[list[str]])`
  - `Buffer.draw_entity(entity)` — espace transparent, clip aux bords, applique `color_mask` sinon `entity.color`.

- [ ] **Step 1: Write failing tests (append)**

```python
from asciimeadow.engine import Buffer, Entity


def test_buffer_init_blank():
    b = Buffer(3, 2)
    assert b.chars == [[" ", " ", " "], [" ", " ", " "]]


def test_draw_entity_places_chars_and_skips_spaces():
    b = Buffer(5, 2)
    e = Entity(frames=["a b"], x=1, y=0, color="green")
    b.draw_entity(e)
    assert b.chars[0][1] == "a"
    assert b.chars[0][2] == " "   # espace transparent => inchangé
    assert b.chars[0][3] == "b"
    assert b.colors[0][1] == "green"


def test_draw_entity_clips_at_edges():
    b = Buffer(3, 1)
    e = Entity(frames=["xyz"], x=2, y=0)  # déborde à droite
    b.draw_entity(e)
    assert b.chars[0][2] == "x"   # seul le 1er char tient


def test_draw_entity_color_mask_overrides():
    b = Buffer(3, 1)
    e = Entity(frames=["ab"], x=0, y=0, color="white", color_mask=["r "])
    b.draw_entity(e)
    assert b.colors[0][0] == "red"     # masque 'r'
    assert b.colors[0][1] == "white"   # masque espace => couleur défaut
```

- [ ] **Step 2: Run, verify fail**

Run: `python -m pytest tests/test_engine.py -q`
Expected: FAIL (`Buffer` undefined).

- [ ] **Step 3: Implement (append to `engine.py`)**

```python
class Buffer:
    def __init__(self, width, height):
        self.width = width
        self.height = height
        self.chars = [[" "] * width for _ in range(height)]
        self.colors = [["white"] * width for _ in range(height)]

    def draw_entity(self, e):
        lines = e.current_frame().split("\n")
        mask = e.current_mask()
        mlines = mask.split("\n") if mask else None
        ox, oy = int(e.x), int(e.y)
        for r, line in enumerate(lines):
            y = oy + r
            if y < 0 or y >= self.height:
                continue
            for c, ch in enumerate(line):
                if ch == " ":
                    continue
                x = ox + c
                if x < 0 or x >= self.width:
                    continue
                self.chars[y][x] = ch
                color = e.color
                if mlines and r < len(mlines) and c < len(mlines[r]):
                    mc = mlines[r][c]
                    if mc != " ":
                        color = MASK_COLORS.get(mc, e.color)
                self.colors[y][x] = color
```

- [ ] **Step 4: Run, verify pass**

Run: `python -m pytest tests/test_engine.py -q`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add asciimeadow/engine.py tests/test_engine.py
git commit -m "feat: buffer with transparent compositing and color masks"
```

---

### Task 4: `composite` (z-order)

**Files:**
- Modify: `asciimeadow/engine.py`
- Modify: `tests/test_engine.py`

**Interfaces:**
- Produces: `engine.composite(buffer, entities)` — dessine les entités triées par `depth` décroissant (loin d'abord), donc la plus petite profondeur écrase.

- [ ] **Step 1: Write failing test (append)**

```python
from asciimeadow.engine import composite


def test_composite_nearer_entity_wins():
    b = Buffer(1, 1)
    far = Entity(frames=["F"], x=0, y=0, depth=80)
    near = Entity(frames=["N"], x=0, y=0, depth=30)
    composite(b, [near, far])  # ordre d'entrée indifférent
    assert b.chars[0][0] == "N"
```

- [ ] **Step 2: Run, verify fail**

Run: `python -m pytest tests/test_engine.py -q`
Expected: FAIL (`composite` undefined).

- [ ] **Step 3: Implement (append to `engine.py`)**

```python
def composite(buffer, entities):
    for e in sorted(entities, key=lambda ent: ent.depth, reverse=True):
        buffer.draw_entity(e)
```

- [ ] **Step 4: Run, verify pass**

Run: `python -m pytest tests/test_engine.py -q`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add asciimeadow/engine.py tests/test_engine.py
git commit -m "feat: depth-ordered compositor"
```

---

### Task 5: `World` (add, advance, culling directionnel, on_death, render)

**Files:**
- Modify: `asciimeadow/engine.py`
- Modify: `tests/test_engine.py`

**Interfaces:**
- Produces:
  - `engine.World(width, height)` avec `.width .height .entities (list)`
  - `World.add(entity) -> entity`
  - `World.advance(dt)` — advance chaque entité, retire celles mortes (`alive False`) ou sorties **du côté de leur direction**, appelle `on_death(entity, world)` sur celles retirées.
  - `World.render() -> Buffer`
- Note culling : une entité n'est retirée que si elle dépasse complètement le bord **vers lequel elle se dirige** (permet d'entrer depuis hors-écran). `dx==dy==0` ⇒ jamais retirée (décor & créatures résidentes).

- [ ] **Step 1: Write failing tests (append)**

```python
from asciimeadow.engine import World


def test_world_culls_entity_exiting_its_direction():
    w = World(width=10, height=5)
    e = w.add(Entity(frames=["x"], x=9.0, y=0, dx=1.0))
    w.advance(2.0)  # x -> 11, dépasse à droite
    assert e not in w.entities


def test_world_allows_entry_from_offscreen():
    w = World(width=10, height=5)
    e = w.add(Entity(frames=["xxx"], x=-3.0, y=0, dx=1.0))  # hors-écran à gauche
    w.advance(1.0)  # x -> -2, entre toujours
    assert e in w.entities


def test_world_keeps_static_entity():
    w = World(width=10, height=5)
    e = w.add(Entity(frames=["x"], x=0, y=0))  # dx=dy=0
    w.advance(100.0)
    assert e in w.entities


def test_world_on_death_called_when_culled():
    w = World(width=5, height=5)
    called = {}
    e = Entity(frames=["x"], x=4.0, y=0, dx=1.0,
               on_death=lambda ent, world: called.setdefault("hit", ent))
    w.add(e)
    w.advance(5.0)
    assert called.get("hit") is e


def test_world_removes_dead_flag_entities():
    w = World(width=5, height=5)
    e = w.add(Entity(frames=["x"], x=1, y=1))
    e.alive = False
    w.advance(0.1)
    assert e not in w.entities


def test_world_render_composites():
    w = World(width=2, height=1)
    w.add(Entity(frames=["a"], x=0, y=0, depth=10))
    buf = w.render()
    assert buf.chars[0][0] == "a"
```

- [ ] **Step 2: Run, verify fail**

Run: `python -m pytest tests/test_engine.py -q`
Expected: FAIL (`World` undefined).

- [ ] **Step 3: Implement (append to `engine.py`)**

```python
class World:
    def __init__(self, width, height):
        self.width = width
        self.height = height
        self.entities = []

    def add(self, entity):
        self.entities.append(entity)
        return entity

    def _offscreen(self, e):
        if e.dx > 0 and e.x >= self.width:
            return True
        if e.dx < 0 and e.x + e.width() <= 0:
            return True
        if e.dy > 0 and e.y >= self.height:
            return True
        if e.dy < 0 and e.y + e.height() <= 0:
            return True
        return False

    def advance(self, dt):
        for e in self.entities:
            e.advance(dt)
        survivors = []
        for e in self.entities:
            if e.alive and not self._offscreen(e):
                survivors.append(e)
            elif e.on_death:
                e.on_death(e, self)
        self.entities = survivors

    def render(self):
        buf = Buffer(self.width, self.height)
        composite(buf, self.entities)
        return buf
```

- [ ] **Step 4: Run, verify pass**

Run: `python -m pytest tests/test_engine.py -q`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add asciimeadow/engine.py tests/test_engine.py
git commit -m "feat: world container with directional culling and render"
```

---

### Task 6: Spawner + `step` (population)

**Files:**
- Create: `asciimeadow/spawn.py`
- Create: `tests/test_spawn.py`

**Interfaces:**
- Consumes: `engine.World`, `engine.Entity`.
- Produces:
  - `spawn.SpawnSpec(name, factory, target, chance)` (dataclass-like ; `factory(world) -> Entity | None`).
  - `spawn.Spawner()` avec `.register(name, factory, target=1, chance=1.0)` et `.tick(world, dt)` — pour chaque spec sous sa cible, ajoute **au plus une** entité par tick avec probabilité `chance * dt` (par seconde).
  - `spawn.step(world, spawner, dt)` — `world.advance(dt)` puis `spawner.tick(world, dt)`.

- [ ] **Step 1: Write failing tests**

`tests/test_spawn.py`:

```python
from asciimeadow.engine import World, Entity
from asciimeadow.spawn import Spawner, step


def make_world():
    return World(width=20, height=10)


def test_spawner_fills_to_target():
    w = make_world()
    sp = Spawner()
    sp.register("bug", lambda world: Entity(frames=["x"], x=1, y=1, name="bug"),
                target=2, chance=1.0)
    for _ in range(5):
        sp.tick(w, dt=1.0)  # chance*dt = 1.0 => spawn garanti tant que < cible
    assert sum(1 for e in w.entities if e.name == "bug") == 2


def test_spawner_respects_chance_zero():
    w = make_world()
    sp = Spawner()
    sp.register("bug", lambda world: Entity(frames=["x"], x=1, y=1, name="bug"),
                target=3, chance=0.0)
    sp.tick(w, dt=1.0)
    assert sum(1 for e in w.entities if e.name == "bug") == 0


def test_step_advances_then_spawns():
    w = make_world()
    sp = Spawner()
    sp.register("bug", lambda world: Entity(frames=["x"], x=1, y=1, name="bug"),
                target=1, chance=1.0)
    moving = w.add(Entity(frames=["x"], x=1.0, y=1.0, dx=1.0, name="mover"))
    step(w, sp, dt=1.0)
    assert moving.x == 2.0  # advance a tourné
    assert sum(1 for e in w.entities if e.name == "bug") == 1  # spawn aussi
```

- [ ] **Step 2: Run, verify fail**

Run: `python -m pytest tests/test_spawn.py -q`
Expected: FAIL (module `spawn` absent).

- [ ] **Step 3: Implement `spawn.py`**

```python
"""Gestion de population : maintient un nombre cible d'entités par type."""
import random


class SpawnSpec:
    def __init__(self, name, factory, target, chance):
        self.name = name
        self.factory = factory
        self.target = target
        self.chance = chance


class Spawner:
    def __init__(self):
        self.specs = []

    def register(self, name, factory, target=1, chance=1.0):
        self.specs.append(SpawnSpec(name, factory, target, chance))

    def tick(self, world, dt):
        for spec in self.specs:
            count = sum(1 for e in world.entities if e.name == spec.name)
            if count < spec.target and random.random() < spec.chance * dt:
                entity = spec.factory(world)
                if entity is not None:
                    world.add(entity)


def step(world, spawner, dt):
    world.advance(dt)
    spawner.tick(world, dt)
```

- [ ] **Step 4: Run, verify pass**

Run: `python -m pytest tests/test_spawn.py -q`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add asciimeadow/spawn.py tests/test_spawn.py
git commit -m "feat: population spawner and world step"
```

---

### Task 7: `art.py` (données ASCII + validation des masques)

**Files:**
- Create: `asciimeadow/art.py`
- Create: `tests/test_scene.py` (démarre ici avec la validation d'art)

**Interfaces:**
- Produces (constantes, toutes des `str` multi-lignes sauf indication) :
  - Décor : `SUN`, `SUN_MASK`, `TREE`, `TREE_MASK`, `GRASS_TUFT`, `FLOWERS (list[str])`.
  - Créatures (frames = `list[str]`) : `BIRD`, `CLOUD (str)`, `BUTTERFLY`, `SQUIRREL`, `OWL`, `OWL_MASK`, `BEE`, `APPLE (str)`, `LEAF (str)`, `RABBIT`, `FOX (str)`, `HEDGEHOG (str)`, `MOUSE (str)`, `SNAIL (str)`.
- Note : un masque, quand présent, a exactement les mêmes lignes/longueurs que sa frame.

- [ ] **Step 1: Write failing test**

`tests/test_scene.py`:

```python
from asciimeadow import art


def test_sun_mask_matches_shape():
    sun_lines = art.SUN.split("\n")
    mask_lines = art.SUN_MASK.split("\n")
    assert len(sun_lines) == len(mask_lines)
    for s, m in zip(sun_lines, mask_lines):
        assert len(s) == len(m)


def test_tree_mask_matches_shape():
    t_lines = art.TREE.split("\n")
    m_lines = art.TREE_MASK.split("\n")
    assert len(t_lines) == len(m_lines)
    for s, m in zip(t_lines, m_lines):
        assert len(s) == len(m)


def test_bird_has_two_flap_frames():
    assert len(art.BIRD) == 2


def test_flowers_non_empty():
    assert len(art.FLOWERS) >= 1
    assert all(isinstance(f, str) and f for f in art.FLOWERS)
```

- [ ] **Step 2: Run, verify fail**

Run: `python -m pytest tests/test_scene.py -q`
Expected: FAIL (module `art` absent).

- [ ] **Step 3: Implement `art.py`**

Chaque sprite porte un commentaire d'intention. Masque présent ⇒ mêmes lignes et longueurs que sa frame (validé par les tests du Step 1).

```python
"""Données ASCII pures (aucune logique). Masques: voir engine.MASK_COLORS."""

SUN = "\n".join([
    r" \ | / ",
    r"- (O) -",
    r" / | \ ",
])
SUN_MASK = "\n".join([
    " y y y ",
    "y yyy y",
    " y y y ",
])

TREE = "\n".join([
    "   @@@@@   ",
    "  @@@@@@@  ",
    " @@@@@@@@@ ",
    "  @@@@@@@  ",
    "    | |    ",
    "    | |    ",
    "    | |    ",
])
TREE_MASK = "\n".join([
    "   ggggg   ",
    "  ggggggg  ",
    " ggggggggg ",
    "  ggggggg  ",
    "    n n    ",
    "    n n    ",
    "    n n    ",
])

GRASS_TUFT = "vvWv"
FLOWERS = ["*", "o", "@"]

BIRD = ["\\v/", "/v\\"]
CLOUD = "\n".join([
    " .-- . ",
    "(      )",
    " '----' ",
])
BUTTERFLY = ["><", "}{"]

SQUIRREL = ["@^", "@v"]
OWL = ["{o,o}\n|}_{|\n ^ ^ ", "{-,-}\n|}_{|\n ^ ^ "]
OWL_MASK = None
BEE = [">8<", "<8>"]
APPLE = "@"
LEAF = "%"

RABBIT = ["(\\(\\\n(-.-)", "(\\(\\\n(o.o)"]
FOX = "/\\_/\\~"
HEDGEHOG = "(\":/)"
MOUSE = "<:3~"
SNAIL = "@_,"
```

- [ ] **Step 4: Run, verify pass**

Run: `python -m pytest tests/test_scene.py -q`
Expected: PASS (4 tests). Si un test de forme de masque échoue, ajuster les espaces du masque pour égaler la frame.

- [ ] **Step 5: Commit**

```bash
git add asciimeadow/art.py tests/test_scene.py
git commit -m "feat: ASCII art data for scenery and creatures"
```

---

### Task 8: Behaviors (`make_*` dans `scene.py`)

**Files:**
- Create: `asciimeadow/scene.py`
- Modify: `tests/test_scene.py`

**Interfaces:**
- Consumes: `engine.Entity`.
- Produces (chaque fonction retourne un `update_fn(entity, dt)`) :
  - `scene.make_fall(gravity, ground_y) -> fn` — accélère `dy`, met `alive=False` quand le bas du sprite atteint `ground_y`.
  - `scene.make_hop(ground_y, amplitude, period) -> fn` — pose `y` pour un sautillement |sin| ; les pieds restent ≤ `ground_y`.
  - `scene.make_orbit(cx, cy, radius, ang_speed) -> fn` — place `x,y` sur une ellipse autour de (cx,cy).
  - `scene.make_zigzag(top, bottom, vy) -> fn` — initialise/inverse `dy` aux bornes verticales.

- [ ] **Step 1: Write failing tests (append to `tests/test_scene.py`)**

```python
import math
from asciimeadow.engine import Entity
from asciimeadow import scene


def _step(e, dt, n):
    for _ in range(n):
        e.advance(dt)


def test_make_fall_accelerates_and_dies_at_ground():
    e = Entity(frames=["@"], x=0, y=0.0)
    e.update_fn = scene.make_fall(gravity=20.0, ground_y=5)
    y0 = e.y
    e.advance(0.1)
    assert e.y > y0            # tombe
    _step(e, 0.1, 100)         # finit par toucher le sol
    assert e.alive is False
    assert e.y + e.height() <= 5 + 1e-6


def test_make_hop_keeps_feet_near_ground():
    e = Entity(frames=["R"], x=0, y=0, dx=1.0)
    e.update_fn = scene.make_hop(ground_y=10, amplitude=3, period=0.5)
    for _ in range(50):
        e.advance(0.05)
        top = e.y
        bottom = e.y + e.height()
        assert bottom <= 10 + 1e-6           # jamais sous le sol
        assert top >= 10 - 3 - e.height() - 1e-6  # pas plus haut que l'amplitude


def test_make_orbit_stays_within_radius():
    e = Entity(frames=["b"], x=0, y=0)
    e.update_fn = scene.make_orbit(cx=20, cy=10, radius=4, ang_speed=2.0)
    for _ in range(60):
        e.advance(0.05)
        dist = math.hypot(e.x - 20, (e.y - 10) * 2)  # ellipse écrasée en y
        assert dist <= 4 + 1e-6


def test_make_zigzag_inverts_at_bounds():
    e = Entity(frames=["x"], x=0, y=5.0, dx=2.0)
    e.update_fn = scene.make_zigzag(top=2, bottom=8, vy=10.0)
    e.advance(0.01)
    assert e.dy > 0            # initialisé vers le bas
    _step(e, 0.05, 40)         # traverse les bornes plusieurs fois
    # après assez de temps, dy a forcément changé de signe au moins une fois
    saw_up = False
    for _ in range(80):
        e.advance(0.05)
        if e.dy < 0:
            saw_up = True
    assert saw_up
```

- [ ] **Step 2: Run, verify fail**

Run: `python -m pytest tests/test_scene.py -q`
Expected: FAIL (module `scene` ou `make_*` absent).

- [ ] **Step 3: Implement behaviors (`scene.py`)**

```python
"""Construction de la prairie : behaviors, factories, spawners."""
import math
import random

from asciimeadow import art
from asciimeadow import engine
from asciimeadow.engine import Entity, flip_horizontal


def make_fall(gravity, ground_y):
    def update(e, dt):
        e.dy += gravity * dt
        if e.y + e.height() >= ground_y:
            e.y = ground_y - e.height()
            e.alive = False
    return update


def make_hop(ground_y, amplitude, period):
    state = {"t": 0.0}

    def update(e, dt):
        state["t"] += dt
        phase = (state["t"] / period) * math.pi
        e.y = ground_y - e.height() - abs(math.sin(phase)) * amplitude
    return update


def make_orbit(cx, cy, radius, ang_speed):
    state = {"a": 0.0}

    def update(e, dt):
        state["a"] += ang_speed * dt
        e.x = cx + radius * math.cos(state["a"])
        e.y = cy + (radius / 2.0) * math.sin(state["a"])
    return update


def make_zigzag(top, bottom, vy):
    def update(e, dt):
        if e.dy == 0:
            e.dy = vy
        if e.y <= top and e.dy < 0:
            e.dy = abs(e.dy)
        elif e.y >= bottom and e.dy > 0:
            e.dy = -abs(e.dy)
    return update
```

- [ ] **Step 4: Run, verify pass**

Run: `python -m pytest tests/test_scene.py -q`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add asciimeadow/scene.py tests/test_scene.py
git commit -m "feat: creature movement behaviors (fall, hop, orbit, zigzag)"
```

---

### Task 9: Décor statique `build_meadow`

**Files:**
- Modify: `asciimeadow/scene.py`
- Modify: `tests/test_scene.py`

**Interfaces:**
- Consumes: `engine.World`, `art`, constantes `engine.DEPTH_*`.
- Produces:
  - `scene.ground_top(world) -> int` — première ligne du sol (= `world.height - 4`).
  - `scene.tree_origin(world) -> (int, int)` — coin haut-gauche de l'arbre (centré horizontalement, base au sol).
  - `scene.build_meadow(world)` — ajoute soleil, arbre, ligne de sol (largeur complète, 2 frames d'herbe ondulante), fleurs dispersées. Décor = entités `dx=dy=0` (jamais cullées).

- [ ] **Step 1: Write failing tests (append)**

```python
from asciimeadow.engine import World


def test_ground_top():
    w = World(40, 20)
    assert scene.ground_top(w) == 16


def test_build_meadow_adds_scenery():
    w = World(60, 24)
    scene.build_meadow(w)
    names = {e.name for e in w.entities}
    assert "sun" in names
    assert "tree" in names
    assert "ground" in names


def test_tree_is_centered():
    w = World(60, 24)
    ox, oy = scene.tree_origin(w)
    tree_w = max(len(l) for l in art.TREE.split("\n"))
    center = ox + tree_w / 2
    assert abs(center - 30) <= 1   # ~centre de 60


def test_ground_spans_full_width():
    w = World(50, 20)
    scene.build_meadow(w)
    ground = next(e for e in w.entities if e.name == "ground")
    assert ground.width() == 50
```

- [ ] **Step 2: Run, verify fail**

Run: `python -m pytest tests/test_scene.py -q`
Expected: FAIL (`build_meadow` etc. absents).

- [ ] **Step 3: Implement (append to `scene.py`)**

```python
GROUND_ROWS = 4


def ground_top(world):
    return world.height - GROUND_ROWS


def tree_origin(world):
    tree_w = max(len(l) for l in art.TREE.split("\n"))
    tree_h = len(art.TREE.split("\n"))
    ox = world.width // 2 - tree_w // 2
    oy = ground_top(world) - tree_h
    return ox, oy


def _ground_frames(width):
    base = (art.GRASS_TUFT * (width // len(art.GRASS_TUFT) + 1))[:width]
    shifted = base[1:] + base[:1]            # décalage => ondulation
    dirt = " " * width
    frame_a = "\n".join([base] + [dirt] * (GROUND_ROWS - 1))
    frame_b = "\n".join([shifted] + [dirt] * (GROUND_ROWS - 1))
    return [frame_a, frame_b]


def build_meadow(world):
    # Soleil — coin haut-droit
    sun_w = max(len(l) for l in art.SUN.split("\n"))
    world.add(Entity(frames=[art.SUN], color_mask=[art.SUN_MASK],
                     x=world.width - sun_w - 1, y=0,
                     depth=engine.DEPTH_SUN, color="yellow", name="sun"))
    # Arbre — centré, base au sol
    tox, toy = tree_origin(world)
    world.add(Entity(frames=[art.TREE], color_mask=[art.TREE_MASK],
                     x=tox, y=toy, depth=engine.DEPTH_TREE,
                     color="green", name="tree"))
    # Sol — pleine largeur, herbe ondulante
    world.add(Entity(frames=_ground_frames(world.width),
                     x=0, y=ground_top(world), depth=engine.DEPTH_FOREGROUND,
                     frame_rate=2.0, color="green", name="ground"))
    # Fleurs — dispersées sur la ligne de sol
    gy = ground_top(world)
    for _ in range(max(3, world.width // 12)):
        fx = random.randint(0, max(0, world.width - 1))
        flower = random.choice(art.FLOWERS)
        color = random.choice(["red", "yellow", "magenta", "white"])
        world.add(Entity(frames=[flower], x=fx, y=gy,
                         depth=engine.DEPTH_FOREGROUND, color=color,
                         name="flower"))
```

- [ ] **Step 4: Run, verify pass**

Run: `python -m pytest tests/test_scene.py -q`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add asciimeadow/scene.py tests/test_scene.py
git commit -m "feat: static meadow scenery (sun, tree, ground, flowers)"
```

---

### Task 10: Factories de créatures + `register_spawners`

**Files:**
- Modify: `asciimeadow/scene.py`
- Modify: `tests/test_scene.py`

**Interfaces:**
- Consumes: `art`, `engine.DEPTH_*`, behaviors `make_*`, `scene.ground_top`, `scene.tree_origin`, `flip_horizontal`.
- Produces:
  - `scene._cross_factory(...)` interne pour créatures traversantes (côté aléatoire, miroir, dx signé) — détail d'implémentation.
  - Factories nommées : `spawn_bird, spawn_cloud, spawn_butterfly, spawn_squirrel, spawn_owl, spawn_bee, spawn_apple, spawn_rabbit, spawn_fox, spawn_hedgehog, spawn_mouse, spawn_snail` (signature `(world) -> Entity`).
  - `scene.register_spawners(spawner)` — enregistre tous les types avec cibles/chances. Les types « résidents » (arbre) : squirrel(1), owl(1), bee(3). Traversants ciel/sol et objets qui tombent : cibles + chances modérées.

- [ ] **Step 1: Write failing tests (append)**

```python
import random as _random
from asciimeadow.spawn import Spawner


def test_cross_factory_sets_direction_and_onscreen_depth():
    w = World(60, 24)
    _random.seed(1)
    b = scene.spawn_bird(w)
    assert b.name == "bird"
    assert b.dx != 0
    assert b.depth == engine.DEPTH_SKY_CREATURE


def test_apple_spawns_in_canopy_and_falls():
    w = World(60, 24)
    a = scene.spawn_apple(w)
    assert a.name == "apple"
    assert a.update_fn is not None
    tox, toy = scene.tree_origin(w)
    assert toy <= a.y <= toy + len(art.TREE.split("\n"))


def test_register_spawners_populates_world():
    w = World(80, 24)
    scene.build_meadow(w)
    sp = Spawner()
    scene.register_spawners(sp)
    _random.seed(0)
    for _ in range(200):
        sp.tick(w, dt=0.1)
    names = {e.name for e in w.entities}
    # au moins les résidents de l'arbre présents
    assert "owl" in names
    assert "bee" in names


def test_owl_is_resident_single():
    w = World(80, 24)
    o = scene.spawn_owl(w)
    assert o.dx == 0 and o.dy == 0   # résident, jamais cullé
```

- [ ] **Step 2: Run, verify fail**

Run: `python -m pytest tests/test_scene.py -q`
Expected: FAIL (factories absentes).

- [ ] **Step 3: Implement (append to `scene.py`)**

```python
def _cross_factory(world, frames, depth, speed, y, color="white",
                   name=None, color_mask=None, frame_rate=4.0):
    """Crée une entité qui traverse l'écran depuis un côté aléatoire."""
    from_left = random.random() < 0.5
    if from_left:
        x = -max(len(l) for l in frames[0].split("\n"))
        dx = speed
        used = frames
    else:
        width = world.width
        x = width
        dx = -speed
        used = [flip_horizontal(f) for f in frames]
    mask = None
    if color_mask:
        mask = color_mask if from_left else [flip_horizontal(m) for m in color_mask]
    return Entity(frames=used, x=x, y=y, dx=dx, depth=depth,
                  frame_rate=frame_rate, color=color, color_mask=mask,
                  name=name)


def spawn_bird(world):
    y = random.randint(1, max(2, world.height // 3))
    return _cross_factory(world, art.BIRD, engine.DEPTH_SKY_CREATURE,
                          speed=random.uniform(8, 14), y=y,
                          color="white", name="bird")


def spawn_cloud(world):
    y = random.randint(0, max(1, world.height // 4))
    return _cross_factory(world, [art.CLOUD], engine.DEPTH_CLOUD,
                          speed=random.uniform(1.5, 3.0), y=y,
                          color="white", name="cloud", frame_rate=0.0)


def spawn_butterfly(world):
    y = ground_top(world) - random.randint(2, 6)
    e = _cross_factory(world, art.BUTTERFLY, engine.DEPTH_SKY_CREATURE,
                       speed=random.uniform(3, 6), y=y,
                       color="magenta", name="butterfly", frame_rate=6.0)
    e.update_fn = make_zigzag(top=1, bottom=ground_top(world) - 1,
                              vy=random.uniform(3, 6))
    return e


def spawn_squirrel(world):
    tox, toy = tree_origin(world)
    trunk_x = world.width // 2
    e = Entity(frames=art.SQUIRREL, x=trunk_x, y=ground_top(world) - 1,
               depth=engine.DEPTH_TREE_CREATURE, frame_rate=4.0,
               color="brown", name="squirrel")
    # grimpe/descend le tronc en boucle
    top_y = toy + len(art.TREE.split("\n")) - 2
    state = {"dir": -1}

    def climb(ent, dt):
        ent.y += state["dir"] * 6.0 * dt
        if ent.y <= top_y:
            state["dir"] = 1
        elif ent.y >= ground_top(world) - 1:
            state["dir"] = -1
    e.update_fn = climb
    return e


def spawn_owl(world):
    tox, toy = tree_origin(world)
    return Entity(frames=art.OWL, x=tox + 2, y=toy + 1,
                  depth=engine.DEPTH_TREE_CREATURE, frame_rate=0.4,
                  color="brown", name="owl")


def spawn_bee(world):
    cx = world.width // 2
    tox, toy = tree_origin(world)
    cy = toy + 1
    e = Entity(frames=art.BEE, x=cx, y=cy,
               depth=engine.DEPTH_TREE_CREATURE, frame_rate=10.0,
               color="yellow", name="bee")
    e.update_fn = make_orbit(cx=cx, cy=cy,
                             radius=random.uniform(3, 6),
                             ang_speed=random.uniform(2, 4))
    return e


def spawn_apple(world):
    tox, toy = tree_origin(world)
    x = random.randint(tox + 1, tox + max(1, len(art.TREE.split("\n")[0]) - 2))
    frame = art.APPLE if random.random() < 0.6 else art.LEAF
    color = "red" if frame == art.APPLE else "green"
    e = Entity(frames=[frame], x=x, y=toy + 1, dy=0.1,
               depth=engine.DEPTH_TREE_CREATURE, color=color, name="apple")
    e.update_fn = make_fall(gravity=12.0, ground_y=ground_top(world))
    return e


def _ground_hopper(world, frames, color, name, amplitude, speed):
    e = _cross_factory(world, frames, engine.DEPTH_GROUND_ANIMAL,
                       speed=speed, y=ground_top(world), color=color,
                       name=name, frame_rate=6.0)
    gy = ground_top(world)
    e.update_fn = make_hop(ground_y=gy + GROUND_ROWS - 1,
                           amplitude=amplitude, period=0.4)
    return e


def spawn_rabbit(world):
    return _ground_hopper(world, art.RABBIT, "white", "rabbit",
                          amplitude=2, speed=random.uniform(6, 9))


def _ground_walker(world, frame, color, name, speed):
    return _cross_factory(world, [frame], engine.DEPTH_GROUND_ANIMAL,
                          speed=speed, y=ground_top(world) + GROUND_ROWS - 1,
                          color=color, name=name, frame_rate=0.0)


def spawn_fox(world):
    return _ground_walker(world, art.FOX, "red", "fox", random.uniform(5, 8))


def spawn_hedgehog(world):
    return _ground_walker(world, art.HEDGEHOG, "brown", "hedgehog",
                          random.uniform(2, 3))


def spawn_mouse(world):
    return _ground_walker(world, art.MOUSE, "white", "mouse",
                          random.uniform(4, 6))


def spawn_snail(world):
    return _ground_walker(world, art.SNAIL, "yellow", "snail",
                          random.uniform(0.8, 1.5))


def register_spawners(spawner):
    # Résidents de l'arbre (jamais cullés ; remplis une fois)
    spawner.register("squirrel", spawn_squirrel, target=1, chance=0.5)
    spawner.register("owl", spawn_owl, target=1, chance=1.0)
    spawner.register("bee", spawn_bee, target=3, chance=0.8)
    # Ciel
    spawner.register("cloud", spawn_cloud, target=3, chance=0.3)
    spawner.register("bird", spawn_bird, target=5, chance=0.5)
    spawner.register("butterfly", spawn_butterfly, target=3, chance=0.4)
    # Objets qui tombent
    spawner.register("apple", spawn_apple, target=2, chance=0.2)
    # Sol
    spawner.register("rabbit", spawn_rabbit, target=2, chance=0.3)
    spawner.register("fox", spawn_fox, target=1, chance=0.1)
    spawner.register("hedgehog", spawn_hedgehog, target=1, chance=0.15)
    spawner.register("mouse", spawn_mouse, target=2, chance=0.3)
    spawner.register("snail", spawn_snail, target=1, chance=0.1)
```

- [ ] **Step 4: Run, verify pass**

Run: `python -m pytest tests/test_scene.py -q`
Expected: PASS. (Si `test_register_spawners_populates_world` est instable, augmenter le nombre de ticks — les résidents owl/bee ont chance élevée et apparaissent vite.)

- [ ] **Step 5: Commit**

```bash
git add asciimeadow/scene.py tests/test_scene.py
git commit -m "feat: creature factories and spawner registration"
```

---

### Task 11: `__main__.py` — affichage curses, boucle, CLI

**Files:**
- Create: `asciimeadow/__main__.py`
- Modify: `tests/test_scene.py` (test de la map couleurs, sans curses)

**Interfaces:**
- Consumes: `engine.World`, `engine.COLOR_NAMES`, `spawn.Spawner/step`, `scene.build_meadow/register_spawners`.
- Produces:
  - `__main__.FPS = 20`
  - `__main__.CURSES_COLORS: dict[str, int]` (nom → constante couleur curses ; **importé paresseusement** pour ne pas charger curses au simple import du test ; voir note).
  - `__main__.CursesDisplay(stdscr)` avec `.size() -> (w, h)` et `.draw(buffer)`.
  - `__main__.run(stdscr, seed=None)` — boucle principale.
  - `__main__.main()` — parse args (`--seed`, `--fps`), `curses.wrapper(run, ...)`.

> Note testabilité : ce module importe `curses` (indisponible/headless en CI parfois). Le seul test ici vérifie que **chaque** nom de `engine.COLOR_NAMES` a une entrée dans la table de correspondance, via une fonction `color_map()` qui construit le dict à l'appel (import de curses différé dans la fonction). Le test importe `color_map` et le compare à `COLOR_NAMES`. Le rendu curses lui-même est validé manuellement.

- [ ] **Step 1: Write failing test (append to `tests/test_scene.py`)**

```python
def test_color_map_covers_all_color_names():
    from asciimeadow.__main__ import color_map
    from asciimeadow.engine import COLOR_NAMES
    cmap = color_map()
    for name in COLOR_NAMES:
        assert name in cmap
```

- [ ] **Step 2: Run, verify fail**

Run: `python -m pytest tests/test_scene.py::test_color_map_covers_all_color_names -q`
Expected: FAIL (`__main__` / `color_map` absent).

- [ ] **Step 3: Implement `__main__.py`**

```python
"""Point d'entrée : affichage curses + boucle principale."""
import argparse
import random

from asciimeadow.engine import World, COLOR_NAMES
from asciimeadow.spawn import Spawner, step
from asciimeadow import scene

FPS = 20


def color_map():
    """Nom de couleur -> constante curses. Import différé (curses non requis pour l'import du module)."""
    import curses
    return {
        "white": curses.COLOR_WHITE,
        "green": curses.COLOR_GREEN,
        "brown": curses.COLOR_YELLOW,   # pas de brun natif -> jaune
        "yellow": curses.COLOR_YELLOW,
        "red": curses.COLOR_RED,
        "cyan": curses.COLOR_CYAN,
        "blue": curses.COLOR_BLUE,
        "magenta": curses.COLOR_MAGENTA,
        "black": curses.COLOR_BLACK,
    }


class CursesDisplay:
    def __init__(self, stdscr):
        import curses
        self.stdscr = stdscr
        self._curses = curses
        curses.curs_set(0)
        curses.start_color()
        curses.use_default_colors()
        self.pairs = {}
        for i, name in enumerate(COLOR_NAMES, start=1):
            curses.init_pair(i, color_map()[name], -1)
            self.pairs[name] = curses.color_pair(i)

    def size(self):
        h, w = self.stdscr.getmaxyx()
        return w, h

    def draw(self, buffer):
        curses = self._curses
        for y in range(buffer.height):
            for x in range(buffer.width):
                ch = buffer.chars[y][x]
                if ch == " ":
                    continue
                attr = self.pairs.get(buffer.colors[y][x], 0)
                try:
                    self.stdscr.addstr(y, x, ch, attr)
                except curses.error:
                    pass  # coin bas-droit : addstr y compris lève, ignorer
        self.stdscr.refresh()


def run(stdscr, seed=None):
    import curses
    if seed is not None:
        random.seed(seed)
    disp = CursesDisplay(stdscr)
    stdscr.timeout(int(1000 / FPS))
    w, h = disp.size()
    world = World(w, h)
    scene.build_meadow(world)
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
            scene.build_meadow(world)
            spawner = Spawner()
            scene.register_spawners(spawner)
            stdscr.clear()
        if not paused:
            step(world, spawner, dt)
        stdscr.erase()
        disp.draw(world.render())


def main():
    import curses
    parser = argparse.ArgumentParser(description="asciimeadow — prairie animée en ASCII")
    parser.add_argument("--seed", type=int, default=None, help="graine aléatoire")
    parser.add_argument("--fps", type=int, default=FPS, help="images/seconde")
    args = parser.parse_args()
    global FPS
    FPS = args.fps
    curses.wrapper(run, seed=args.seed)


if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Run, verify pass**

Run: `python -m pytest tests/test_scene.py::test_color_map_covers_all_color_names -q`
Expected: PASS.

- [ ] **Step 5: Full suite green**

Run: `python -m pytest -q`
Expected: tous les tests PASS.

- [ ] **Step 6: Manual smoke test**

Run: `python -m asciimeadow --seed 1`
Vérifier visuellement : soleil en haut-droite, arbre centré avec hibou/abeilles/écureuil, sol avec herbe/fleurs, oiseaux & nuages traversant le ciel, papillons en zigzag, lapins qui sautillent, pommes qui tombent. Tester `p` (pause), `r` (redraw), redimensionner le terminal, `q` (quitter).

- [ ] **Step 7: Commit**

```bash
git add asciimeadow/__main__.py tests/test_scene.py
git commit -m "feat: curses display, main loop and CLI entry point"
```

---

## Self-Review

**1. Spec coverage**

| Exigence spec | Tâche(s) |
| --- | --- |
| Terminal Python / `curses` / `python -m asciimeadow` | 11 |
| Ciel : soleil, nuages, oiseaux, papillons | 9 (soleil), 10 (nuages/oiseaux/papillons) |
| Arbre : statique + écureuil, hibou, abeilles, pommes/feuilles | 9 (arbre), 10 (créatures) |
| Sol : herbe ondulante, fleurs, lapins, renard, hérisson, souris, escargot | 9 (herbe/fleurs), 10 (animaux) |
| Z-order profondeur | 1 (constantes), 4 (composite), 9-10 (depths) |
| Modèle Entity (frames, masque, dx/dy, depth, frame_rate, on_death, transparent) | 1, 3, 5 |
| Spawn : population cible, auto-mort hors-écran, côté aléatoire + miroir | 5 (cull), 6 (spawner), 10 (factories/miroir) |
| Boucle : 20 FPS, q/p/r, KEY_RESIZE | 11 |
| Couleur : init_pair, couleur unie + masque | 3 (masque), 11 (init_pair) |
| Tests headless (mouvement, cull, z-order, population) | 1, 4, 5, 6, 8 |
| YAGNI : pas de jour/nuit, météo, son | respecté (absents) |

Aucune lacune.

**2. Placeholder scan** — aucun TBD/TODO ; chaque step de code montre le code réel et complet (`art.py` = un seul bloc propre, une assignation par nom).

**3. Type consistency** — vérifié : `Entity(...)` signature identique partout ; `update_fn` attribut (set après construction dans factories, OK) ; `make_fall/hop/orbit/zigzag` signatures cohérentes Task 8 ↔ Task 10 ; `ground_top`, `tree_origin`, `GROUND_ROWS` cohérents Task 9 ↔ 10 ; `color_map()` défini Task 11 et testé même tâche ; `step`, `Spawner.register/tick` cohérents Task 6 ↔ 10-11.
