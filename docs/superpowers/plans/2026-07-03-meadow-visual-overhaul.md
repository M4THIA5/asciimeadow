# Refonte visuelle de la prairie — Plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Arbre adaptatif (2 variantes), bande d'herbe dense sur 4 lignes, et sprites détaillés avec masques couleur pour bird, squirrel, owl, rabbit, fox, hedgehog, mouse.

**Architecture:** Aucun changement structurel — on enrichit `art.py` (données), on ajoute une constante de profondeur dans `engine.py`, et on adapte les factories de `scene.py`. Le moteur, le spawner et `__main__.py` sont inchangés. Spec : `docs/superpowers/specs/2026-07-03-meadow-visual-overhaul-design.md`.

**Tech Stack:** Python stdlib uniquement, pytest (headless), curses seulement dans `__main__.py`.

## Global Constraints

- Python n'est **pas** sur le PATH : toujours `./venv/bin/python -m pytest …`.
- Commentaires et docstrings en **français**.
- Aucun `import curses` hors de `__main__.py` (et uniquement en lazy import).
- Aucune dépendance tierce.
- Les comportements de déplacement sont des closures assignées à `Entity.update_fn` — ne jamais ajouter de champ de mouvement à `Entity`.
- Un espace `' '` dans une frame est transparent ; un espace dans un masque = couleur unie de l'entité.
- Sprites dessinés orientés **droite** ; `_cross_factory` gère le miroir. Les masques ne contiennent que des lettres de `MASK_COLORS` et des espaces (sûrs au flip).
- Chaque ligne de masque doit avoir exactement la même longueur que la ligne de frame correspondante (test d'intégrité générique, Task 2).
- Papillon, abeille, escargot, nuages, soleil : **inchangés**.

---

### Task 1: Constante de profondeur `DEPTH_GRASS`

L'herbe dense sera dessinée à `DEPTH_FOREGROUND` (30) → dessinée en dernier → elle écraserait les animaux du sol (40). On insère une couche `DEPTH_GRASS = 45` entre les créatures d'arbre (50) et les animaux du sol (40) : l'herbe est derrière les animaux.

**Files:**
- Modify: `asciimeadow/engine.py:15-21` (bloc des constantes de profondeur)
- Test: `tests/test_engine.py`

**Interfaces:**
- Produces: `engine.DEPTH_GRASS = 45` (int), utilisé par Task 3.

- [ ] **Step 1: Écrire le test qui échoue**

Ajouter à la fin de `tests/test_engine.py` :

```python
def test_depth_grass_between_ground_animals_and_tree_creatures():
    from asciimeadow import engine
    assert engine.DEPTH_GROUND_ANIMAL < engine.DEPTH_GRASS < engine.DEPTH_TREE_CREATURE
```

- [ ] **Step 2: Vérifier l'échec**

Run: `./venv/bin/python -m pytest tests/test_engine.py::test_depth_grass_between_ground_animals_and_tree_creatures -v`
Expected: FAIL — `AttributeError: module 'asciimeadow.engine' has no attribute 'DEPTH_GRASS'`

- [ ] **Step 3: Implémenter**

Dans `asciimeadow/engine.py`, remplacer le bloc :

```python
DEPTH_TREE_CREATURE = 50
DEPTH_GROUND_ANIMAL = 40
DEPTH_FOREGROUND = 30
```

par :

```python
DEPTH_TREE_CREATURE = 50
DEPTH_GRASS = 45          # herbe dense : derrière les animaux du sol
DEPTH_GROUND_ANIMAL = 40
DEPTH_FOREGROUND = 30
```

- [ ] **Step 4: Vérifier le passage**

Run: `./venv/bin/python -m pytest tests/test_engine.py -v`
Expected: tous PASS

- [ ] **Step 5: Commit**

```bash
git add asciimeadow/engine.py tests/test_engine.py
git commit -m "feat: DEPTH_GRASS layer between tree creatures and ground animals"
```

---

### Task 2: Arbre adaptatif (`TREE_SMALL` / `TREE_LARGE`)

Deux variantes d'arbre dans `art.py` ; `scene.tree_art(world)` choisit selon la taille du monde. Tout ce qui lisait `art.TREE` en dur passe par `tree_art`. Un test d'intégrité générique (nouveau fichier `tests/test_art.py`) valide **tous** les masques par convention de nommage `X` / `X_MASK` — il remplace les tests unitaires `test_sun_mask_matches_shape` et `test_tree_mask_matches_shape`.

**Files:**
- Modify: `asciimeadow/art.py:14-31` (TREE → TREE_SMALL + ajout TREE_LARGE)
- Modify: `asciimeadow/scene.py:62-67` (`tree_origin`), `scene.py:85-89` (arbre dans `build_meadow`), `scene.py:190-198` (`spawn_apple`)
- Create: `tests/test_art.py`
- Test: `tests/test_scene.py` (3 tests mis à jour, 2 supprimés, 2 ajoutés)

**Interfaces:**
- Consumes: rien de nouveau.
- Produces:
  - `art.TREE_SMALL: str`, `art.TREE_SMALL_MASK: str` (l'ancien TREE, renommé — `art.TREE` disparaît)
  - `art.TREE_LARGE: str`, `art.TREE_LARGE_MASK: str` (23 colonnes × 14 lignes)
  - `scene.tree_art(world) -> tuple[str, str]` — renvoie `(frame, mask)` ; `TREE_LARGE` si `world.height >= 28 and world.width >= 40`, sinon `TREE_SMALL`.
  - `scene.tree_origin(world) -> tuple[int, int]` — signature inchangée, géométrie basée sur `tree_art`.

- [ ] **Step 1: Écrire les tests qui échouent**

Créer `tests/test_art.py` :

```python
"""Intégrité des sprites : chaque masque X_MASK épouse exactement la grille de X."""
from asciimeadow import art


def _frames_list(x):
    return x if isinstance(x, list) else [x]


def test_every_mask_matches_its_frames():
    checked = 0
    for name in dir(art):
        if not name.endswith("_MASK"):
            continue
        mask = getattr(art, name)
        if mask is None:
            continue
        frames = _frames_list(getattr(art, name[: -len("_MASK")]))
        masks = _frames_list(mask)
        assert len(frames) == len(masks), name
        for f, m in zip(frames, masks):
            f_lines, m_lines = f.split("\n"), m.split("\n")
            assert len(f_lines) == len(m_lines), name
            for a, b in zip(f_lines, m_lines):
                assert len(a) == len(b), name
        checked += 1
    assert checked >= 3   # SUN, TREE_SMALL, TREE_LARGE au minimum
```

Dans `tests/test_scene.py` :

1. **Supprimer** `test_sun_mask_matches_shape` (lignes 61-66) et `test_tree_mask_matches_shape` (lignes 69-74) — remplacés par `test_art.py`.
2. **Mettre à jour** `test_tree_is_centered` :

```python
def test_tree_is_centered():
    from asciimeadow.engine import World
    w = World(60, 24)
    ox, oy = scene.tree_origin(w)
    frame, _ = scene.tree_art(w)
    tree_w = max(len(l) for l in frame.split("\n"))
    center = ox + tree_w / 2
    assert abs(center - 30) <= 1   # ~centre de 60
```

3. **Mettre à jour** `test_apple_spawns_in_canopy_and_falls` :

```python
def test_apple_spawns_in_canopy_and_falls():
    w = World(60, 24)
    a = scene.spawn_apple(w)
    assert a.name == "apple"
    assert a.update_fn is not None
    tox, toy = scene.tree_origin(w)
    frame, _ = scene.tree_art(w)
    assert toy <= a.y <= toy + len(frame.split("\n"))
```

4. **Ajouter** :

```python
def test_tree_art_selects_variant_by_size():
    small = World(80, 24)          # hauteur < 28 => petit arbre
    large = World(100, 35)         # 35 >= 28 et 100 >= 40 => grand arbre
    narrow = World(39, 35)         # trop étroit => petit arbre
    assert scene.tree_art(small)[0] is art.TREE_SMALL
    assert scene.tree_art(large)[0] is art.TREE_LARGE
    assert scene.tree_art(narrow)[0] is art.TREE_SMALL


def test_large_tree_fits_on_screen():
    w = World(100, 35)
    ox, oy = scene.tree_origin(w)
    assert oy >= 0
    assert ox >= 0
```

- [ ] **Step 2: Vérifier l'échec**

Run: `./venv/bin/python -m pytest tests/test_art.py tests/test_scene.py -v`
Expected: FAIL — `test_every_mask_matches_its_frames` échoue sur `checked >= 3` (TREE_SMALL absent), `test_tree_art_selects_variant_by_size` échoue avec `AttributeError: ... no attribute 'tree_art'`.

- [ ] **Step 3: Implémenter — art.py**

Dans `asciimeadow/art.py`, remplacer le bloc `TREE = …` / `TREE_MASK = …` (lignes 14-31) par :

```python
TREE_SMALL = "\n".join([
    "   @@@@@   ",
    "  @@@@@@@  ",
    " @@@@@@@@@ ",
    "  @@@@@@@  ",
    "    | |    ",
    "    | |    ",
    "    | |    ",
])
TREE_SMALL_MASK = "\n".join([
    "   ggggg   ",
    "  ggggggg  ",
    " ggggggggg ",
    "  ggggggg  ",
    "    n n    ",
    "    n n    ",
    "    n n    ",
])

TREE_LARGE = "\n".join([
    r"        .@@@@@@.       ",
    r"     @@@@@@@@@@@@@@    ",
    r"   @@@@@@@@@@@@@@@@@@  ",
    r"  @@@@@@@@@@@@@@@@@@@@ ",
    r" @@@@@@@@@@@@@@@@@@@@@@",
    r"  @@@@@@@@@@@@@@@@@@@@ ",
    r"    @@@@@@@@@@@@@@@@   ",
    r"      '@@@@@@@@@@'     ",
    r"         \|  |/        ",
    r"          |  |         ",
    r"         /|  |\        ",
    r"        / |  | \       ",
    r"          |  |         ",
    r"       ___|  |___      ",
])
TREE_LARGE_MASK = "\n".join([
    r"        gggggggg       ",
    r"     gggggggggggggg    ",
    r"   gggggggggggggggggg  ",
    r"  gggggggggggggggggggg ",
    r" gggggggggggggggggggggg",
    r"  gggggggggggggggggggg ",
    r"    gggggggggggggggg   ",
    r"      gggggggggggg     ",
    r"         nn  nn        ",
    r"          n  n         ",
    r"         nn  nn        ",
    r"        n n  n n       ",
    r"          n  n         ",
    r"       nnnn  nnnn      ",
])
```

Toutes les lignes de `TREE_LARGE` font exactement 23 caractères (rembourrées d'espaces) ; le masque doit être identique en grille — le test de la Task 2 Step 1 le vérifie.

- [ ] **Step 4: Implémenter — scene.py**

Dans `asciimeadow/scene.py`, remplacer `tree_origin` (lignes 62-67) par :

```python
def tree_art(world):
    """Variante d'arbre selon la taille du monde (le resize resélectionne)."""
    if world.height >= 28 and world.width >= 40:
        return art.TREE_LARGE, art.TREE_LARGE_MASK
    return art.TREE_SMALL, art.TREE_SMALL_MASK


def tree_origin(world):
    frame, _ = tree_art(world)
    lines = frame.split("\n")
    tree_w = max(len(l) for l in lines)
    tree_h = len(lines)
    ox = world.width // 2 - tree_w // 2
    oy = ground_top(world) - tree_h
    return ox, oy
```

Dans `build_meadow`, remplacer le bloc arbre (lignes 85-89) par :

```python
    # Arbre — centré, base au sol, variante selon la taille du terminal
    tf, tm = tree_art(world)
    tox, toy = tree_origin(world)
    world.add(Entity(frames=[tf], color_mask=[tm],
                     x=tox, y=toy, depth=engine.DEPTH_TREE,
                     color="green", name="tree"))
```

Remplacer `spawn_apple` (lignes 190-198) par :

```python
def spawn_apple(world):
    tox, toy = tree_origin(world)
    tf, _ = tree_art(world)
    tw = max(len(l) for l in tf.split("\n"))
    x = random.randint(tox + 1, tox + tw - 2)
    frame = art.APPLE if random.random() < 0.6 else art.LEAF
    color = "red" if frame == art.APPLE else "green"
    e = Entity(frames=[frame], x=x, y=toy + 1, dy=0.1,
               depth=engine.DEPTH_TREE_CREATURE, color=color, name="apple")
    e.update_fn = make_fall(gravity=12.0, ground_y=ground_top(world))
    return e
```

- [ ] **Step 5: Vérifier le passage**

Run: `./venv/bin/python -m pytest -v`
Expected: tous PASS (plus aucune référence à `art.TREE` — vérifier avec `grep -rn "art\.TREE\b" asciimeadow/ tests/` : aucun résultat).

- [ ] **Step 6: Commit**

```bash
git add asciimeadow/art.py asciimeadow/scene.py tests/test_art.py tests/test_scene.py
git commit -m "feat: adaptive tree, large variant on big terminals"
```

---

### Task 3: Herbe dense sur toute la bande + fleurs dispersées

Les 4 lignes de `GROUND_ROWS` se remplissent de touffes variées (motifs différents par ligne, plus clairsemés vers le bas), ondulation par décalage sur toute la bande. La bande passe à `DEPTH_GRASS`. Les fleurs se dispersent sur toute la bande.

**Files:**
- Modify: `asciimeadow/art.py:33` (`GRASS_TUFT` → `GRASS_ROWS`)
- Modify: `asciimeadow/scene.py:70-76` (`_ground_frames`), `scene.py:90-102` (sol + fleurs dans `build_meadow`)
- Test: `tests/test_scene.py`

**Interfaces:**
- Consumes: `engine.DEPTH_GRASS` (Task 1).
- Produces:
  - `art.GRASS_ROWS: list[str]` — 4 motifs de touffes (un par ligne de la bande). `art.GRASS_TUFT` disparaît.
  - `scene._ground_frames(width) -> list[str]` — 2 frames de `GROUND_ROWS` lignes pleines, chacune large de `width`.

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter à `tests/test_scene.py` :

```python
def test_ground_frames_fill_all_rows():
    frames = scene._ground_frames(30)
    assert len(frames) == 2
    for f in frames:
        lines = f.split("\n")
        assert len(lines) == scene.GROUND_ROWS
        for line in lines:
            assert len(line) == 30
            assert line.strip() != ""      # chaque ligne contient de l'herbe


def test_ground_band_at_grass_depth():
    w = World(50, 20)
    scene.build_meadow(w)
    ground = next(e for e in w.entities if e.name == "ground")
    assert ground.depth == engine.DEPTH_GRASS


def test_flowers_spread_across_band():
    _random.seed(5)
    w = World(60, 24)
    scene.build_meadow(w)
    flowers = [e for e in w.entities if e.name == "flower"]
    gy = scene.ground_top(w)
    assert flowers
    assert all(gy <= e.y <= w.height - 1 for e in flowers)
```

- [ ] **Step 2: Vérifier l'échec**

Run: `./venv/bin/python -m pytest tests/test_scene.py -v -k "ground_frames or grass_depth or flowers_spread"`
Expected: FAIL — `test_ground_frames_fill_all_rows` échoue sur `line.strip() != ""` (lignes de terre vides), `test_ground_band_at_grass_depth` échoue (`DEPTH_FOREGROUND` != `DEPTH_GRASS`). `test_flowers_spread_across_band` peut passer (fleurs déjà sur `gy`) — c'est un test de non-régression pour la dispersion.

- [ ] **Step 3: Implémenter — art.py**

Remplacer `GRASS_TUFT = "vvWv"` (ligne 33) par :

```python
# Un motif de touffes par ligne de la bande d'herbe (répété sur la largeur).
GRASS_ROWS = [
    "vWv,vw'vvW.wv,v",
    ",w'v.vW,v'wv,W.",
    "v.,'vv,w.'v,.v'",
    ",'.,v.',.,'v.,.",
]
```

- [ ] **Step 4: Implémenter — scene.py**

Remplacer `_ground_frames` (lignes 70-76) par :

```python
def _ground_frames(width):
    rows = [(p * (width // len(p) + 1))[:width] for p in art.GRASS_ROWS]
    shifted = [r[1:] + r[:1] for r in rows]  # décalage => ondulation
    return ["\n".join(rows), "\n".join(shifted)]
```

Note : `GROUND_ROWS` vaut 4 et `art.GRASS_ROWS` a 4 motifs — si l'un change, l'autre doit suivre (le test de Step 1 le verrouille).

Dans `build_meadow`, remplacer le bloc sol + fleurs (lignes 90-102) par :

```python
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

- [ ] **Step 5: Vérifier le passage**

Run: `./venv/bin/python -m pytest -v`
Expected: tous PASS (vérifier aussi `grep -rn "GRASS_TUFT" asciimeadow/ tests/` : aucun résultat).

- [ ] **Step 6: Commit**

```bash
git add asciimeadow/art.py asciimeadow/scene.py tests/test_scene.py
git commit -m "feat: dense grass band with scattered flowers, behind animals"
```

---

### Task 4: Animaux du sol détaillés (rabbit, fox, hedgehog, mouse)

Sprites 3 lignes avec masques couleur. `_ground_walker` place les pieds sur la dernière ligne de l'écran quel que soit le nombre de lignes du sprite, et accepte un masque. Le lapin (hopper) reçoit aussi son masque. L'escargot ne change pas.

**Files:**
- Modify: `asciimeadow/art.py:51-55` (RABBIT, FOX, HEDGEHOG, MOUSE + masques ; SNAIL inchangé)
- Modify: `asciimeadow/scene.py:201-238` (`_ground_hopper`, `spawn_rabbit`, `_ground_walker`, `spawn_fox`, `spawn_hedgehog`, `spawn_mouse`)
- Test: `tests/test_scene.py`

**Interfaces:**
- Consumes: `_cross_factory(world, frames, depth, speed, y, color, name, color_mask, frame_rate)` (existant, gère déjà le flip des masques).
- Produces:
  - `art.RABBIT: list[str]` (2 frames 7×3), `art.RABBIT_MASK: list[str]`
  - `art.FOX: str` (13×3), `art.FOX_MASK: str`
  - `art.HEDGEHOG: str` (13×3), `art.HEDGEHOG_MASK: str`
  - `art.MOUSE: str` (9×3), `art.MOUSE_MASK: str`
  - `scene._ground_walker(world, frame, color, name, speed, mask=None)` — `y = world.height - hauteur(frame)`.
  - `scene._ground_hopper(world, frames, color, name, amplitude, speed, masks=None)`.

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter à `tests/test_scene.py` :

```python
def test_ground_animals_are_multiline_and_masked():
    assert len(art.RABBIT) == 2 and len(art.RABBIT_MASK) == 2
    for sprite in (art.FOX, art.HEDGEHOG, art.MOUSE):
        assert len(sprite.split("\n")) >= 3     # sprites détaillés
    for mask in (art.FOX_MASK, art.HEDGEHOG_MASK, art.MOUSE_MASK):
        assert isinstance(mask, str)


def test_walker_bottom_aligned_to_screen():
    _random.seed(3)
    w = World(60, 24)
    for factory in (scene.spawn_fox, scene.spawn_hedgehog, scene.spawn_mouse):
        e = factory(w)
        assert int(e.y) + e.height() == w.height


def test_ground_animals_carry_masks():
    _random.seed(3)
    w = World(60, 24)
    for factory in (scene.spawn_rabbit, scene.spawn_fox,
                    scene.spawn_hedgehog, scene.spawn_mouse):
        assert factory(w).color_mask is not None
```

- [ ] **Step 2: Vérifier l'échec**

Run: `./venv/bin/python -m pytest tests/test_scene.py -v -k "ground_animals or walker_bottom"`
Expected: FAIL — `AttributeError: module 'asciimeadow.art' has no attribute 'RABBIT_MASK'`, etc.

- [ ] **Step 3: Implémenter — art.py**

Remplacer les lignes 51-55 (`RABBIT` … `SNAIL`) par (SNAIL reste identique) :

```python
RABBIT = [
    "\n".join([
        r" (\_/) ",
        r"(='.'=)",
        r'(")_(")',
    ]),
    "\n".join([
        r" (\_/) ",
        r"(=-.-=)",
        r'(")_(")',
    ]),
]
RABBIT_MASK = [
    "\n".join([
        "       ",
        "   r   ",
        "       ",
    ]),
] * 2

FOX = "\n".join([
    r"(\__      /\ ",
    r" \  \____(o'>",
    r"  \_/|  | |/ ",
])
FOX_MASK = "\n".join([
    "wwrr      rr ",
    " r  rrrrrrkwk",
    "  rrrk  k kk ",
])

HEDGEHOG = "\n".join([
    r"  ,;;;;;;;,  ",
    r" ;;;;;;;;;(o>",
    r'  " "   " "  ',
])
HEDGEHOG_MASK = "\n".join([
    "  nnnnnnnnn  ",
    " nnnnnnnnnwky",
    "  n n   n n  ",
])

MOUSE = "\n".join([
    r"      (\ ",
    r"~~(___o,>",
    r'   " "   ',
])
MOUSE_MASK = "\n".join([
    "      ww ",
    "wwwwwwkwk",
    "   w w   ",
])

SNAIL = "@_,"
```

Repères de design : lapin = longues oreilles `(\_/)` + museau rouge ; renard = queue touffue à gauche (bout blanc `ww`), oreille `/\`, museau pointu `'>`, pattes noires ; hérisson = dos de piquants `;` marron, museau clair `(o>` ; souris = oreille ronde `(\`, longue queue `~~`, œil et nez noirs. Le test générique de `tests/test_art.py` valide automatiquement chaque grille de masque.

- [ ] **Step 4: Implémenter — scene.py**

Remplacer les lignes 201-238 (`_ground_hopper` … `spawn_snail`) par :

```python
def _ground_hopper(world, frames, color, name, amplitude, speed, masks=None):
    e = _cross_factory(world, frames, engine.DEPTH_GROUND_ANIMAL,
                       speed=speed, y=ground_top(world), color=color,
                       name=name, frame_rate=6.0, color_mask=masks)
    gy = ground_top(world)
    e.update_fn = make_hop(ground_y=gy + GROUND_ROWS - 1,
                           amplitude=amplitude, period=0.4)
    return e


def spawn_rabbit(world):
    return _ground_hopper(world, art.RABBIT, "white", "rabbit",
                          amplitude=2, speed=random.uniform(6, 9),
                          masks=art.RABBIT_MASK)


def _ground_walker(world, frame, color, name, speed, mask=None):
    height = len(frame.split("\n"))
    masks = [mask] if mask else None
    return _cross_factory(world, [frame], engine.DEPTH_GROUND_ANIMAL,
                          speed=speed, y=world.height - height,
                          color=color, name=name, frame_rate=0.0,
                          color_mask=masks)


def spawn_fox(world):
    return _ground_walker(world, art.FOX, "red", "fox",
                          random.uniform(5, 8), mask=art.FOX_MASK)


def spawn_hedgehog(world):
    return _ground_walker(world, art.HEDGEHOG, "brown", "hedgehog",
                          random.uniform(2, 3), mask=art.HEDGEHOG_MASK)


def spawn_mouse(world):
    return _ground_walker(world, art.MOUSE, "white", "mouse",
                          random.uniform(4, 6), mask=art.MOUSE_MASK)


def spawn_snail(world):
    return _ground_walker(world, art.SNAIL, "yellow", "snail",
                          random.uniform(0.8, 1.5))
```

- [ ] **Step 5: Vérifier le passage**

Run: `./venv/bin/python -m pytest -v`
Expected: tous PASS (y compris `tests/test_art.py` qui valide les 4 nouveaux masques).

- [ ] **Step 6: Commit**

```bash
git add asciimeadow/art.py asciimeadow/scene.py tests/test_scene.py
git commit -m "feat: detailed masked sprites for ground animals"
```

---

### Task 5: Animaux du ciel et de l'arbre (bird, squirrel, owl)

Oiseau 3 lignes à 2 frames de battement ; écureuil 3 lignes à queue touffue qui grimpe pieds posés sur l'herbe ; hibou 4 lignes à aigrettes, perché au centre de la canopée de la variante d'arbre courante. Le plancher du test d'intégrité passe à 10 masques.

**Files:**
- Modify: `asciimeadow/art.py:36-47` (BIRD, SQUIRREL, OWL + masques)
- Modify: `asciimeadow/scene.py:126-130` (`spawn_bird`), `scene.py:150-167` (`spawn_squirrel`), `scene.py:170-174` (`spawn_owl`)
- Test: `tests/test_scene.py` (1 test mis à jour, 2 ajoutés), `tests/test_art.py` (plancher)

**Interfaces:**
- Consumes: `scene.tree_art` / `scene.tree_origin` (Task 2).
- Produces:
  - `art.BIRD: list[str]` (2 frames 6×3), `art.BIRD_MASK: list[str]`
  - `art.SQUIRREL: list[str]` (2 frames 4×3), `art.SQUIRREL_MASK: list[str]`
  - `art.OWL: list[str]` (2 frames 5×4), `art.OWL_MASK: list[str]` (remplace `OWL_MASK = None`)

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `tests/test_art.py`, remplacer la dernière ligne `assert checked >= 3` par :

```python
    assert checked >= 10   # sun, 2 arbres, bird, squirrel, owl, rabbit, fox, hedgehog, mouse
```

Dans `tests/test_scene.py`, **mettre à jour** `test_squirrel_climbs_full_trunk_range` (l'écureuil fait maintenant 3 lignes, sa course s'arrête pieds sur l'herbe) :

```python
def test_squirrel_climbs_full_trunk_range():
    from asciimeadow.engine import World
    w = World(60, 24)
    sq = scene.spawn_squirrel(w)
    ys = []
    for _ in range(400):
        sq.advance(0.05)
        ys.append(sq.y)
    tox, toy = scene.tree_origin(w)
    gt = scene.ground_top(w)
    assert min(ys) <= toy + 2                      # atteint le haut de l'arbre
    assert max(ys) + sq.height() >= gt - 1         # redescend pieds sur l'herbe
```

**Ajouter** :

```python
def test_owl_perched_in_canopy():
    for w in (World(60, 24), World(100, 35)):
        o = scene.spawn_owl(w)
        tox, toy = scene.tree_origin(w)
        frame, _ = scene.tree_art(w)
        tree_h = len(frame.split("\n"))
        assert toy < o.y <= toy + tree_h // 2 + 1  # dans la canopée
        assert o.color_mask is not None


def test_bird_and_squirrel_have_masked_flap_frames():
    assert len(art.BIRD) == 2 and len(art.BIRD_MASK) == 2
    assert len(art.SQUIRREL) == 2 and len(art.SQUIRREL_MASK) == 2
    _random.seed(1)
    w = World(60, 24)
    assert scene.spawn_bird(w).color_mask is not None
    assert scene.spawn_squirrel(w).color_mask is not None
```

- [ ] **Step 2: Vérifier l'échec**

Run: `./venv/bin/python -m pytest tests/test_art.py tests/test_scene.py -v -k "mask or owl_perched or squirrel"`
Expected: FAIL — `checked >= 10` (9 masques seulement, `OWL_MASK` est `None`), `AttributeError: ... no attribute 'BIRD_MASK'`.

- [ ] **Step 3: Implémenter — art.py**

Remplacer la ligne 36 (`BIRD = …`) par :

```python
BIRD = [
    "\n".join([
        r"  \ \ ",
        r"__( o>",
        r"      ",
    ]),
    "\n".join([
        r"      ",
        r"__( o>",
        r"  / / ",
    ]),
]
BIRD_MASK = [
    "\n".join([
        "  w w ",
        "www ky",
        "      ",
    ]),
    "\n".join([
        "      ",
        "www ky",
        "  w w ",
    ]),
]
```

(Les deux frames partagent la même boîte 6×3, corps sur la ligne du milieu : pas de saut vertical au battement.)

Remplacer les lignes 44-46 (`SQUIRREL = …`, `OWL = …`, `OWL_MASK = None`) par :

```python
SQUIRREL = [
    "\n".join([
        r"(\  ",
        r"(o\ ",
        r"@@) ",
    ]),
    "\n".join([
        r"(\  ",
        r"(o\ ",
        r"@@( ",
    ]),
]
SQUIRREL_MASK = [
    "\n".join([
        "nn  ",
        "nkn ",
        "nnn ",
    ]),
] * 2

OWL = [
    "\n".join([
        r" ^ ^ ",
        r"(O,O)",
        r"(:v:)",
        r' " " ',
    ]),
    "\n".join([
        r" ^ ^ ",
        r"(-,-)",
        r"(:v:)",
        r' " " ',
    ]),
]
OWL_MASK = [
    "\n".join([
        " n n ",
        "nyyyn",
        "nnwnn",
        " y y ",
    ]),
] * 2
```

Repères de design : oiseau = bec jaune `>`, œil noir, ailes qui alternent haut/bas ; écureuil = pose verticale de grimpe, oreille `(\`, queue touffue `@@` qui fouette (`)`/`(`) ; hibou = aigrettes `^ ^`, grands yeux jaunes `O,O` qui clignent (`-,-`), poitrail `:v:`, serres jaunes.

- [ ] **Step 4: Implémenter — scene.py**

Remplacer `spawn_bird` (lignes 126-130) par :

```python
def spawn_bird(world):
    y = random.randint(1, max(2, world.height // 3))
    return _cross_factory(world, art.BIRD, engine.DEPTH_SKY_CREATURE,
                          speed=random.uniform(8, 14), y=y,
                          color="white", name="bird",
                          color_mask=art.BIRD_MASK)
```

Remplacer `spawn_squirrel` (lignes 150-167) par :

```python
def spawn_squirrel(world):
    tox, toy = tree_origin(world)
    trunk_x = world.width // 2 - 1
    e = Entity(frames=art.SQUIRREL, x=trunk_x, y=ground_top(world) - 3,
               depth=engine.DEPTH_TREE_CREATURE, frame_rate=4.0,
               color="brown", color_mask=art.SQUIRREL_MASK, name="squirrel")
    # grimpe/descend le tronc en boucle, pieds posés sur l'herbe en bas
    top_y = toy + 1
    bottom_y = ground_top(world) - 3   # 3 = hauteur du sprite
    state = {"dir": -1}

    def climb(ent, dt):
        ent.y += state["dir"] * 6.0 * dt
        if ent.y <= top_y:
            state["dir"] = 1
        elif ent.y >= bottom_y:
            state["dir"] = -1
    e.update_fn = climb
    return e
```

Remplacer `spawn_owl` (lignes 170-174) par :

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
    return Entity(frames=art.OWL, x=x, y=y, color_mask=art.OWL_MASK,
                  depth=engine.DEPTH_TREE_CREATURE, frame_rate=0.4,
                  color="brown", name="owl")
```

- [ ] **Step 5: Vérifier le passage**

Run: `./venv/bin/python -m pytest -v`
Expected: tous PASS.

- [ ] **Step 6: Commit**

```bash
git add asciimeadow/art.py asciimeadow/scene.py tests/test_art.py tests/test_scene.py
git commit -m "feat: detailed masked sprites for bird, squirrel and owl"
```

---

### Task 6: Vérification visuelle et finitions

La reconnaissabilité des sprites ne se teste qu'à l'œil : lancer l'app dans un vrai terminal, petit et grand, et corriger l'art si un sprite rend mal (grille cassée au flip, couleurs illisibles, chevauchements).

**Files:**
- Modify: `asciimeadow/art.py` (retouches éventuelles uniquement)
- Modify: `CLAUDE.md` (nombre de tests dans « Testing approach »)

**Interfaces:** rien de nouveau.

- [ ] **Step 1: Suite complète**

Run: `./venv/bin/python -m pytest`
Expected: tous PASS (~51 tests : 41 initiaux − 2 supprimés + 10 ajoutés).

- [ ] **Step 2: Vérification visuelle**

Utiliser la skill `run-asciimeadow` (l'app exige un vrai tty) avec `--seed 42` pour un rendu reproductible. Vérifier deux géométries :

1. **Petit terminal** (~80×24) : petit arbre, bande d'herbe pleine sur 4 lignes, animaux devant l'herbe.
2. **Grand terminal** (≥100×35) : grand arbre (canopée arrondie, tronc branchu), hibou perché dans la canopée, écureuil sur le tronc.

Points de contrôle : chaque animal reconnaissable dans les deux sens (flip), masques alignés (pas de couleur qui déborde d'un caractère), lapin qui saute sans passer sous l'herbe, renard/hérisson/souris pieds sur la dernière ligne.

- [ ] **Step 3: Retouches éventuelles**

Si un sprite rend mal : ne toucher **que** `art.py` (frames + masque en même grille), relancer `./venv/bin/python -m pytest tests/test_art.py` après chaque retouche, puis re-vérifier visuellement.

- [ ] **Step 4: Mettre à jour CLAUDE.md**

Dans la section « Testing approach », remplacer `41 tests` par le compte réel affiché par pytest au Step 1.

- [ ] **Step 5: Commit final**

```bash
git add asciimeadow/art.py CLAUDE.md
git commit -m "polish: visual pass on new sprites, update test count"
```

(Si aucune retouche : committer seulement CLAUDE.md.)
