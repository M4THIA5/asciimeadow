# Réécriture de asciimeadow en Rust — design

Date : 2026-07-06

## Objectif

Réécrire `asciimeadow` (économiseur d'écran ASCII terminal) de Python vers
Rust, de façon **idiomatique** (pas un portage ligne à ligne). Comportement et
fonctionnalités identiques : arbre, animaux, météo, cycle jour/nuit, rendu
double-buffer avec diff. Le code Python et ses tests sont **supprimés** ; les
tests sont **portés** en Rust pour garantir la parité de comportement.

Le code et les commentaires restent en **français** (comme la base Python).

## Contraintes et décisions

- **Rewrite idiomatique** — structuré à la Rust (enums, ownership, traits si
  utile), pas fonction-pour-fonction.
- **Backend terminal : `crossterm` brut.** Pas de framework TUI (ratatui). On
  garde notre propre `Buffer` + diff-draw, ce qui colle au modèle actuel.
- **Dépendances lean : `crossterm` + `rand` uniquement.** Arguments CLI
  (`--seed`, `--fps`, `--day-length`) parsés à la main via `std::env::args`.
- **Python supprimé, tests portés.** Dépôt mono-langage Rust à la fin.
- **Règle dure préservée : cœur pur / coquille terminal.** Seul `main.rs`
  importe `crossterm` ou fait des I/O. Tout le reste est du Rust pur, testable
  sans TTY.

## Structure du crate

Crate binaire unique. `Cargo.toml` : `crossterm`, `rand`.

| Module Rust | ← Python | rôle |
|-------------|----------|------|
| `engine.rs` | `engine.py` | `Entity`, `Buffer`, `World`, `composite`, constantes de profondeur, `MASK_COLORS`, `flip_horizontal` |
| `scene.rs` | `scene.py` | construction de la prairie, factories de comportements, factories de spawn, `register_spawners` |
| `spawn.rs` | `spawn.py` | `Spawner` (contrôle de population) et le tick `step` |
| `environment.rs` | `environment.py` | modèle jour/nuit + météo |
| `art.rs` | `art.py` | art ASCII et chaînes de masque couleur (données seules, aucune logique) |
| `main.rs` | `__main__.py` | argparse, affichage crossterm, boucle principale (seul module « terminal-aware ») |
| `lib.rs` | `__init__.py` | câblage des modules ; expose le cœur pur aux tests sans terminal |

## Pipeline de frame — inchangé

`step(world, spawner, dt)`, appelé une fois par frame à `FPS` :

1. `env.update(dt)` — avance l'horloge jour/nuit + la machine à états météo
   **en premier**, pour que les spawners voient l'état courant.
2. `world.advance(dt)` — déplace chaque entité, exécute son comportement, puis
   supprime les entités mortes/hors écran (déclenche `on_death`).
3. `spawner.tick(world, dt)` — remonte les populations vers leurs cibles.

Puis `world.render()` compose un `Buffer` et `Display::draw` le peint. crossterm
ne redessine que les cellules changées (respecte l'intention double-buffer
actuelle).

## Rendu / profondeur / masques

- **Compositing** : `composite` dessine les entités triées par `depth`
  **décroissant** — `depth` plus grand = plus loin = dessiné en premier
  (algorithme du peintre). Profondeurs = constantes nommées `i32`
  (`DEPTH_SUN = 90` … `DEPTH_FOREGROUND = 30`).
- **Buffer** : grilles parallèles `chars` et `colors`. Un espace dans un sprite
  est transparent, **sauf** si `entity.opaque = true`, où les trous intérieurs
  entre le premier et le dernier glyphe d'une ligne écrasent le fond (utilisé
  par les animaux au sol pour que l'herbe ne transparaisse pas).
- **Masques couleur** (style asciiquarium) : l'art dans `art.rs` est de la
  donnée pure. Un sprite peut porter un `color_mask` — une chaîne (ou liste
  parallèle aux `frames`) où chaque code d'un caractère est mappé via
  `MASK_COLORS` (`g`→vert, `n`→marron, `y`→jaune, …) vers une couleur par
  glyphe. Pas de caractère de masque → la `color` de base de l'entité. Les
  masques s'alignent caractère par caractère avec les frames ;
  `flip_horizontal` retourne frame et masque **ensemble** (entités entrant par
  la droite).
- **Couleur** : type `Color` mappé vers `crossterm::style::Color` uniquement
  dans la coquille ; le cœur ne connaît que les codes/`MASK_COLORS`.

## Entités et comportements — modèle enum (fork retenu : A)

Une `Entity` = `frames` (liste de chaînes multi-lignes) + position/vitesse +
`behaviors: Vec<Behavior>` + drapeaux (`opaque`, `alive`, `on_death`, `depth`,
`color`, `color_mask`).

`Behavior` est un **enum** portant l'état mutable par variante :

- `Fall` — chute (cf. `make_fall`)
- `Hop { phase, … }` — saut au sol (`make_hop`)
- `Orbit { angle, … }` — orbite (`make_orbit`, ex. soleil/lune)
- `Zigzag { … }` — zigzag (`make_zigzag`)
- `Lifespan { remaining }` — durée de vie (`make_lifespan`)
- `EnvCull { is_day }` — supprime selon jour/nuit (`make_env_cull`)

Une fonction centrale `apply(entity, dt, &EnvSnapshot) -> bool` fait un `match`
et renvoie « vivant ? ». Le **chaînage** (`_chain`) devient simplement l'ordre du
`Vec<Behavior>` : on applique chaque comportement, l'entité meurt dès qu'un
renvoie « mort ».

**Lecture de l'environnement sans conflit de borrow** : `World` possède le
`Vec<Entity>` **et** l'`Env`. Emprunter `&env` en mutant `&mut entity` en
parallèle est interdit. Solution : construire un `EnvSnapshot` `Copy`
(`{ is_day, is_night, raining, windy, storming, wind_dx, … }`) une fois par
frame et le passer **par valeur** à chaque `apply`. Idiomatique, pas de
`Rc`/`RefCell`, pas de dyn dispatch.

*(Alternative écartée — B : `Box<dyn FnMut(&mut Entity, f32, &EnvSnapshot) ->
bool>`, plus proche de Python mais dispatch dynamique et moins inspectable/
testable.)*

## Spawning

`Spawner` + `register_spawners`. Chaque spec a une `target` et une `chance` :

- `target` : un enum `Target::{Fixed(i32), Dynamic(fn(&World) -> i32)}`. Les
  cibles dynamiques (`day_target`, `night_target`, `rain_target`) ne capturent
  rien (elles lisent seulement `world`), donc un pointeur de fonction nu
  `fn(&World) -> i32` suffit — pas de `Box<dyn>`. Elles renvoient 0 quand la
  condition ne tient pas, ce qui laisse la population se vider.
- `chance` : probabilité par seconde (multipliée par `dt`).
- Une factory peut renvoyer `None` pour décliner le spawn (les factories météo
  le font quand il ne pleut/vente pas).

## Environnement

Horloge globale + météo, créées par `build_meadow`, stockées sur `world.env`.
`phase` va de 0→1 sur `day_length` ; première moitié = jour, seconde = nuit
(`is_night`). Météo = machine à états pondérée (`Clear`/`Wind`/`Rain`/`Storm`)
avec timers de dwell ; expose `raining`/`windy`/`storming`/`wind_dx`. `rng`
(`StdRng`) injectable pour tests déterministes.

## Déterminisme

`rand::rngs::StdRng` initialisé depuis `--seed`. `Env` détient un `StdRng`
injectable (équivalent de `env.rng`). Les tests seedent directement. **Pas**
byte-identique à Python — reproductible pour un même seed *au sein de Rust*.

## Coquille terminal (`main.rs`)

- Parse `--seed`, `--fps`, `--day-length` à la main.
- crossterm en mode raw + écran alterné ; restauration propre en sortie
  (y compris sur `Ctrl+C` — un guard `Drop` rétablit le terminal).
- Boucle principale à `FPS` : lit les touches (`q` quitter, `p` pause,
  `r` redraw), gère le resize (reconstruit le monde), appelle `step`, diffe le
  `Buffer` et peint.

## Tests

Les 6 fichiers de tests Python sont portés en tests Rust (unitaires `#[cfg(test)]`
dans chaque module et/ou `tests/` d'intégration) :

- `test_art` — cohérence art/masques (alignement caractère par caractère).
- `test_engine` — `composite`, profondeur, transparence/`opaque`,
  `flip_horizontal`, `Buffer`.
- `test_environment` — progression `phase`, `is_night`, transitions météo
  déterministes avec `rng` seedé.
- `test_scene` — factories de comportements (ex. « hop garde les pieds près du
  sol »), gating jour/nuit/pluie.
- `test_spawn` — `Spawner` remonte vers `target`, cibles dynamiques à 0.
- `test_main` — parsing d'arguments et logique de boucle testable hors TTY.

Objectif : parité de comportement vérifiable, exécution rapide sans terminal.

## Hors périmètre (YAGNI)

- Pas de sortie byte-identique à Python.
- Pas de nouvelle fonctionnalité (météo, créatures, etc.) — parité seule.
- Pas de framework TUI, pas de bindings ncurses, pas de dépendances au-delà de
  `crossterm` + `rand`.
- Pas de support Windows spécifique au-delà de ce que crossterm offre
  gratuitement.
