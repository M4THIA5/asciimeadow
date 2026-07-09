# Rust Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port `asciimeadow` (terminal ASCII screensaver) from Python to idiomatic Rust with the same behavior and features, deleting the Python code and porting its tests.

**Architecture:** Cargo package with a pure library crate (`src/lib.rs` → `engine`, `environment`, `spawn`, `scene`, `art`) that has zero `crossterm` usage, plus a binary (`src/main.rs`) that is the only `crossterm`-aware module (terminal shell). Entity behaviors are a data-oriented `Behavior` enum applied through a central `apply`; behaviors read a `Copy` `EnvSnapshot` passed by value each frame to avoid borrow conflicts.

**Tech Stack:** Rust (edition 2021), `crossterm` (terminal), `rand` (seedable `StdRng`). No other dependencies. CLI args hand-parsed.

## Global Constraints

- Edition 2021. Dependencies limited to `crossterm` and `rand` — nothing else.
- **Pure core / terminal shell split is a hard rule:** modules `engine`, `environment`, `spawn`, `scene`, `art` MUST NOT reference `crossterm` or do any I/O. Only `src/main.rs` may use `crossterm`.
- Code and comments are written in **French** (matches the Python original). Test names may be English.
- Depths are named `i32` consts (`DEPTH_SUN=90` … `DEPTH_FOREGROUND=30`); never literals.
- `composite` draws entities sorted by `depth` **descending** (painter's algorithm) with a **stable** sort.
- Masks align with frames character-for-character; `flip_horizontal` flips frame and mask together.
- Determinism is per-seed within Rust (NOT byte-identical to Python). `StdRng` is the RNG everywhere; `Environment` owns its own `StdRng`, `World` owns the master `StdRng`.
- Runtime keys: `q` quit, `p` pause, `r` redraw; `Ctrl+C` quits cleanly; terminal resize rebuilds the world.
- Default FPS 20, default `day_length` 90.0.

---

### Task 1: Cargo scaffold + crate skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/main.rs`
- Create: `src/engine.rs`, `src/environment.rs`, `src/spawn.rs`, `src/scene.rs`, `src/art.rs` (empty stubs)

**Interfaces:**
- Produces: crate `asciimeadow` (lib) with modules `art`, `engine`, `environment`, `spawn`, `scene`; binary `asciimeadow`.

- [ ] **Step 1: Write `Cargo.toml`**

```toml
[package]
name = "asciimeadow"
version = "0.1.0"
edition = "2021"
description = "Terminal ASCII meadow screensaver"

[lib]
name = "asciimeadow"
path = "src/lib.rs"

[[bin]]
name = "asciimeadow"
path = "src/main.rs"

[dependencies]
crossterm = "0.28"
rand = "0.8"
```

- [ ] **Step 2: Write `src/lib.rs`**

```rust
//! asciimeadow — cœur pur (sans terminal). Voir `main.rs` pour la coquille crossterm.
pub mod art;
pub mod engine;
pub mod environment;
pub mod scene;
pub mod spawn;
```

- [ ] **Step 3: Write empty module stubs**

`src/engine.rs`, `src/environment.rs`, `src/spawn.rs`, `src/scene.rs`, `src/art.rs` each start with a single doc comment line (content filled by later tasks), e.g. `src/art.rs`:

```rust
//! Données ASCII pures (aucune logique). Masques : voir engine::mask_color.
```

- [ ] **Step 4: Write minimal `src/main.rs`**

```rust
//! Point d'entrée : coquille crossterm + boucle principale (seul module terminal-aware).
fn main() {
    println!("asciimeadow");
}
```

- [ ] **Step 5: Verify build and empty test run**

Run: `cargo build`
Expected: compiles clean.
Run: `cargo test`
Expected: builds, runs 0 tests, exits 0.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/
git commit -m "chore: scaffold Rust crate (lib + bin skeleton)"
```

---

### Task 2: engine — colors, masks, depths, flip_horizontal

**Files:**
- Modify: `src/engine.rs`
- Test: `src/engine.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Produces:
  - `pub enum Color { White, Green, Brown, Yellow, Red, Cyan, Blue, Magenta, Black }` (derives `Clone, Copy, PartialEq, Eq, Debug`)
  - `pub const COLOR_NAMES: [Color; 9]`
  - `pub fn mask_color(c: char) -> Option<Color>`
  - depth consts: `DEPTH_SUN, DEPTH_CLOUD, DEPTH_SKY_CREATURE, DEPTH_TREE, DEPTH_TREE_CREATURE, DEPTH_GRASS, DEPTH_GROUND_ANIMAL, DEPTH_FOREGROUND` (all `pub const … : i32`)
  - `pub fn flip_horizontal(frame: &str) -> String`

- [ ] **Step 1: Write the failing tests** (append to `src/engine.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flip_reverses_and_swaps_chars() {
        assert_eq!(flip_horizontal("<o--"), "--o>");
        assert_eq!(flip_horizontal("(_)"), "(_)");
    }

    #[test]
    fn flip_multiline_each_line() {
        assert_eq!(flip_horizontal("ab\ncd"), "ba\ndc");
    }

    #[test]
    fn flip_swaps_braces_and_brackets() {
        assert_eq!(flip_horizontal("{a"), "a}");
        assert_eq!(flip_horizontal("[x"), "x]");
        assert_eq!(flip_horizontal("(y"), "y)");
    }

    #[test]
    fn depth_grass_between_ground_animals_and_tree_creatures() {
        assert!(DEPTH_GROUND_ANIMAL < DEPTH_GRASS && DEPTH_GRASS < DEPTH_TREE_CREATURE);
    }

    #[test]
    fn mask_color_known_and_unknown() {
        assert_eq!(mask_color('g'), Some(Color::Green));
        assert_eq!(mask_color('n'), Some(Color::Brown));
        assert_eq!(mask_color(' '), None);
        assert_eq!(mask_color('?'), None);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib engine::tests`
Expected: FAIL to compile (`Color`, `flip_horizontal`, depth consts, `mask_color` not defined).

- [ ] **Step 3: Implement** (top of `src/engine.rs`, above the tests)

```rust
//! Moteur pur (sans crossterm) : couleurs, entités, buffer, compositor.

/// Palette logique. Le mapping vers crossterm vit dans la coquille (main.rs).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    White,
    Green,
    Brown,
    Yellow,
    Red,
    Cyan,
    Blue,
    Magenta,
    Black,
}

pub const COLOR_NAMES: [Color; 9] = [
    Color::White,
    Color::Green,
    Color::Brown,
    Color::Yellow,
    Color::Red,
    Color::Cyan,
    Color::Blue,
    Color::Magenta,
    Color::Black,
];

/// Caractère de masque -> couleur (style asciiquarium). `None` => couleur de base.
pub fn mask_color(c: char) -> Option<Color> {
    match c {
        'w' => Some(Color::White),
        'g' => Some(Color::Green),
        'n' => Some(Color::Brown),
        'y' => Some(Color::Yellow),
        'r' => Some(Color::Red),
        'c' => Some(Color::Cyan),
        'b' => Some(Color::Blue),
        'm' => Some(Color::Magenta),
        'k' => Some(Color::Black),
        _ => None,
    }
}

// Profondeur : plus grand = plus loin (dessiné en premier).
pub const DEPTH_SUN: i32 = 90;
pub const DEPTH_CLOUD: i32 = 80;
pub const DEPTH_SKY_CREATURE: i32 = 70;
pub const DEPTH_TREE: i32 = 60;
pub const DEPTH_TREE_CREATURE: i32 = 50;
pub const DEPTH_GRASS: i32 = 45; // herbe dense : derrière les animaux du sol
pub const DEPTH_GROUND_ANIMAL: i32 = 40;
pub const DEPTH_FOREGROUND: i32 = 30;

fn flip_char(c: char) -> char {
    match c {
        '<' => '>',
        '>' => '<',
        '[' => ']',
        ']' => '[',
        '(' => ')',
        ')' => '(',
        '{' => '}',
        '}' => '{',
        '/' => '\\',
        '\\' => '/',
        other => other,
    }
}

/// Retourne un sprite horizontalement : inverse chaque ligne et permute les
/// glyphes directionnels. Sert aux entités entrant par la droite.
pub fn flip_horizontal(frame: &str) -> String {
    frame
        .split('\n')
        .map(|line| line.chars().rev().map(flip_char).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib engine::tests`
Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/engine.rs
git commit -m "feat(engine): colors, mask map, depth consts, flip_horizontal"
```

---

### Task 3: engine — EnvSnapshot, Behavior enum, Entity

**Files:**
- Modify: `src/engine.rs`
- Test: `src/engine.rs`

**Interfaces:**
- Consumes: `Color`, `mask_color` (Task 2).
- Produces:
  - `pub struct EnvSnapshot { pub is_day: bool, pub is_night: bool, pub raining: bool, pub windy: bool, pub storming: bool, pub wind_dx: f64 }` (derives `Clone, Copy`); `EnvSnapshot::none() -> Self`.
  - `pub enum Behavior { Fall{gravity:f64, ground_y:f64, first:bool}, Hop{ground_y:f64, amplitude:f64, period:f64, t:f64}, Orbit{cx:f64, cy:f64, radius:f64, ang_speed:f64, a:f64}, Zigzag{top:f64, bottom:f64, vy:f64}, Lifespan{seconds:f64, t:f64}, EnvCull{day:bool} }` with `fn apply(&mut self, e:&mut Entity, dt:f64, env:EnvSnapshot)`.
  - `pub struct Entity` (public fields listed below) with builder `Entity::new(frames: Vec<String>)` and fluent setters `pos, vel, with_depth, with_color, with_frame_rate, with_mask, with_name, opaque, with_behavior, on_death`; methods `current_frame(&self)->&str`, `current_mask(&self)->Option<&str>`, `height(&self)->usize`, `width(&self)->usize`, `advance(&mut self, dt:f64, env:EnvSnapshot)`.
- Note: `on_death` type is `Option<Box<dyn FnMut(&mut Entity, &mut World)>>`; `World` is defined in Task 6. Because this task compiles before `World` exists, add a forward declaration in this step: place `use crate::engine::World;`-style reference by defining the field with the fully-qualified path `Option<Box<dyn FnMut(&mut Entity, &mut crate::engine::World)>>`. `World` will exist in the same module by Task 6; until then this task's code will not compile standalone, so **implement Task 3 and Task 6 in the same working session** and run tests only after Task 6. (Tasks 3–6 all live in `engine.rs`.)

- [ ] **Step 1: Write the failing tests** (append inside the existing `mod tests`)

```rust
    #[test]
    fn entity_moves_by_velocity() {
        let mut e = Entity::new(vec!["x".into()]).pos(0.0, 0.0).vel(2.0, -1.0);
        e.advance(0.5, EnvSnapshot::none());
        assert_eq!(e.x, 1.0);
        assert_eq!(e.y, -0.5);
    }

    #[test]
    fn entity_animation_advances_frames() {
        let mut e = Entity::new(vec!["A".into(), "B".into()]).with_frame_rate(2.0);
        assert_eq!(e.current_frame(), "A");
        e.advance(0.5, EnvSnapshot::none());
        assert_eq!(e.current_frame(), "B");
        e.advance(0.5, EnvSnapshot::none());
        assert_eq!(e.current_frame(), "A");
    }

    #[test]
    fn entity_single_frame_does_not_animate() {
        let mut e = Entity::new(vec!["A".into()]).with_frame_rate(5.0);
        e.advance(10.0, EnvSnapshot::none());
        assert_eq!(e.current_frame(), "A");
    }

    #[test]
    fn entity_dimensions() {
        let e = Entity::new(vec!["abc\nde".into()]);
        assert_eq!(e.width(), 3);
        assert_eq!(e.height(), 2);
    }

    #[test]
    fn entity_current_mask_none_vs_present() {
        let e = Entity::new(vec!["ab".into()]).with_mask(vec!["rg".into()]);
        assert_eq!(e.current_mask(), Some("rg"));
        let e2 = Entity::new(vec!["ab".into()]);
        assert_eq!(e2.current_mask(), None);
    }

    #[test]
    fn behavior_fall_accelerates_and_dies_at_ground() {
        let mut e = Entity::new(vec!["@".into()]).pos(0.0, 0.0)
            .with_behavior(Behavior::Fall { gravity: 20.0, ground_y: 5.0, first: true });
        let y0 = e.y;
        e.advance(0.1, EnvSnapshot::none());
        assert!(e.y > y0);
        for _ in 0..100 { e.advance(0.1, EnvSnapshot::none()); }
        assert!(!e.alive);
        assert!(e.y + e.height() as f64 <= 5.0 + 1e-6);
    }

    #[test]
    fn behavior_hop_keeps_feet_near_ground() {
        let mut e = Entity::new(vec!["R".into()]).vel(1.0, 0.0)
            .with_behavior(Behavior::Hop { ground_y: 10.0, amplitude: 3.0, period: 0.5, t: 0.0 });
        for _ in 0..50 {
            e.advance(0.05, EnvSnapshot::none());
            let bottom = e.y + e.height() as f64;
            assert!(bottom <= 10.0 + 1e-6);
            assert!(e.y >= 10.0 - 3.0 - e.height() as f64 - 1e-6);
        }
    }

    #[test]
    fn behavior_orbit_stays_within_radius() {
        let mut e = Entity::new(vec!["b".into()])
            .with_behavior(Behavior::Orbit { cx: 20.0, cy: 10.0, radius: 4.0, ang_speed: 2.0, a: 0.0 });
        for _ in 0..60 {
            e.advance(0.05, EnvSnapshot::none());
            let dist = ((e.x - 20.0).powi(2) + ((e.y - 10.0) * 2.0).powi(2)).sqrt();
            assert!(dist <= 4.0 + 1e-6);
        }
    }

    #[test]
    fn behavior_zigzag_inverts_at_bounds() {
        let mut e = Entity::new(vec!["x".into()]).pos(0.0, 5.0).vel(2.0, 0.0)
            .with_behavior(Behavior::Zigzag { top: 2.0, bottom: 8.0, vy: 10.0 });
        e.advance(0.01, EnvSnapshot::none());
        assert!(e.dy > 0.0);
        let mut saw_up = false;
        for _ in 0..120 {
            e.advance(0.05, EnvSnapshot::none());
            if e.dy < 0.0 { saw_up = true; }
        }
        assert!(saw_up);
    }

    #[test]
    fn behavior_lifespan_kills_after_delay() {
        let mut e = Entity::new(vec!["x".into()])
            .with_behavior(Behavior::Lifespan { seconds: 0.3, t: 0.0 });
        e.advance(0.2, EnvSnapshot::none());
        assert!(e.alive);
        e.advance(0.2, EnvSnapshot::none());
        assert!(!e.alive);
    }

    #[test]
    fn behavior_env_cull_day_dies_at_night() {
        let day = EnvSnapshot { is_day: true, is_night: false, raining: false, windy: false, storming: false, wind_dx: 0.0 };
        let night = EnvSnapshot { is_day: false, is_night: true, raining: false, windy: false, storming: false, wind_dx: 0.0 };
        let mut e = Entity::new(vec!["s".into()]).with_behavior(Behavior::EnvCull { day: true });
        e.advance(0.1, day);
        assert!(e.alive);
        e.advance(0.1, night);
        assert!(!e.alive);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib engine::tests` (will only pass after Task 6 compiles `World`)
Expected: FAIL to compile (`Entity`, `Behavior`, `EnvSnapshot` not defined).

- [ ] **Step 3: Implement** (append to `src/engine.rs`, after Task 2 code)

```rust
/// Vue immuable de l'environnement, passée par valeur aux comportements chaque
/// frame (évite d'emprunter `world.env` en mutant une entité).
#[derive(Clone, Copy)]
pub struct EnvSnapshot {
    pub is_day: bool,
    pub is_night: bool,
    pub raining: bool,
    pub windy: bool,
    pub storming: bool,
    pub wind_dx: f64,
}

impl EnvSnapshot {
    /// Défaut hors env : plein jour, temps clair.
    pub fn none() -> Self {
        EnvSnapshot {
            is_day: true,
            is_night: false,
            raining: false,
            windy: false,
            storming: false,
            wind_dx: 0.0,
        }
    }
}

/// Comportement d'entité (mouvement / durée de vie / cull). L'état mutable vit
/// dans la variante ; le chaînage = l'ordre du `Vec<Behavior>` de l'entité.
pub enum Behavior {
    Fall { gravity: f64, ground_y: f64, first: bool },
    Hop { ground_y: f64, amplitude: f64, period: f64, t: f64 },
    Orbit { cx: f64, cy: f64, radius: f64, ang_speed: f64, a: f64 },
    Zigzag { top: f64, bottom: f64, vy: f64 },
    Lifespan { seconds: f64, t: f64 },
    /// `day=true` : vit le jour, meurt la nuit. `day=false` : l'inverse.
    EnvCull { day: bool },
}

impl Behavior {
    fn apply(&mut self, e: &mut Entity, dt: f64, env: EnvSnapshot) {
        match self {
            Behavior::Fall { gravity, ground_y, first } => {
                e.dy += *gravity * dt;
                if *first {
                    e.y += 0.5 * *gravity * dt * dt;
                    *first = false;
                }
                let h = e.height() as f64;
                if e.y + h >= *ground_y {
                    e.y = *ground_y - h;
                    e.alive = false;
                }
            }
            Behavior::Hop { ground_y, amplitude, period, t } => {
                *t += dt;
                let phase = (*t / *period) * std::f64::consts::PI;
                e.y = *ground_y - e.height() as f64 - phase.sin().abs() * *amplitude;
            }
            Behavior::Orbit { cx, cy, radius, ang_speed, a } => {
                *a += *ang_speed * dt;
                e.x = *cx + *radius * a.cos();
                e.y = *cy + (*radius / 2.0) * a.sin();
            }
            Behavior::Zigzag { top, bottom, vy } => {
                if e.dy == 0.0 {
                    e.dy = *vy;
                }
                if e.y <= *top && e.dy < 0.0 {
                    e.dy = e.dy.abs();
                } else if e.y >= *bottom && e.dy > 0.0 {
                    e.dy = -e.dy.abs();
                }
            }
            Behavior::Lifespan { seconds, t } => {
                *t += dt;
                if *t >= *seconds {
                    e.alive = false;
                }
            }
            Behavior::EnvCull { day } => {
                let keep = if *day { env.is_day } else { env.is_night };
                if !keep {
                    e.alive = false;
                }
            }
        }
    }
}

/// Entité : frames multi-lignes + position/vitesse + comportements + drapeaux.
pub struct Entity {
    pub frames: Vec<String>,
    pub x: f64,
    pub y: f64,
    pub dx: f64,
    pub dy: f64,
    pub depth: i32,
    pub frame_rate: f64,
    pub color: Color,
    pub color_mask: Option<Vec<String>>,
    pub name: Option<String>,
    pub opaque: bool,
    pub alive: bool,
    pub behaviors: Vec<Behavior>,
    pub on_death: Option<Box<dyn FnMut(&mut Entity, &mut World)>>,
    frame_idx: usize,
    anim_accum: f64,
}

impl Entity {
    pub fn new(frames: Vec<String>) -> Self {
        Entity {
            frames,
            x: 0.0,
            y: 0.0,
            dx: 0.0,
            dy: 0.0,
            depth: 0,
            frame_rate: 0.0,
            color: Color::White,
            color_mask: None,
            name: None,
            opaque: false,
            alive: true,
            behaviors: Vec::new(),
            on_death: None,
            frame_idx: 0,
            anim_accum: 0.0,
        }
    }

    pub fn pos(mut self, x: f64, y: f64) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    pub fn vel(mut self, dx: f64, dy: f64) -> Self {
        self.dx = dx;
        self.dy = dy;
        self
    }
    pub fn with_depth(mut self, d: i32) -> Self {
        self.depth = d;
        self
    }
    pub fn with_color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }
    pub fn with_frame_rate(mut self, r: f64) -> Self {
        self.frame_rate = r;
        self
    }
    pub fn with_mask(mut self, m: Vec<String>) -> Self {
        self.color_mask = Some(m);
        self
    }
    pub fn with_name(mut self, n: &str) -> Self {
        self.name = Some(n.to_string());
        self
    }
    pub fn opaque(mut self, v: bool) -> Self {
        self.opaque = v;
        self
    }
    pub fn with_behavior(mut self, b: Behavior) -> Self {
        self.behaviors.push(b);
        self
    }
    pub fn on_death(mut self, f: Box<dyn FnMut(&mut Entity, &mut World)>) -> Self {
        self.on_death = Some(f);
        self
    }

    pub fn current_frame(&self) -> &str {
        self.frames[self.frame_idx].as_str()
    }
    pub fn current_mask(&self) -> Option<&str> {
        self.color_mask.as_ref().map(|m| m[self.frame_idx].as_str())
    }
    pub fn height(&self) -> usize {
        self.current_frame().split('\n').count()
    }
    pub fn width(&self) -> usize {
        self.current_frame()
            .split('\n')
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0)
    }

    pub fn advance(&mut self, dt: f64, env: EnvSnapshot) {
        self.x += self.dx * dt;
        self.y += self.dy * dt;
        if self.frame_rate > 0.0 && self.frames.len() > 1 {
            self.anim_accum += dt;
            let step = 1.0 / self.frame_rate;
            while self.anim_accum >= step {
                self.anim_accum -= step;
                self.frame_idx = (self.frame_idx + 1) % self.frames.len();
            }
        }
        // Comportements chaînés : on sort le Vec pour lever l'alias self/behaviors.
        let mut behaviors = std::mem::take(&mut self.behaviors);
        for b in &mut behaviors {
            b.apply(self, dt, env);
        }
        self.behaviors = behaviors;
    }
}
```

- [ ] **Step 4: Defer verification** — these tests compile/pass only once `World` exists (Task 6). Proceed to Task 4.

- [ ] **Step 5: Commit** (after Task 6 is green; see Task 6 Step 5)

---

### Task 4: engine — Buffer, draw_entity, composite

**Files:**
- Modify: `src/engine.rs`
- Test: `src/engine.rs`

**Interfaces:**
- Consumes: `Entity`, `Color`, `mask_color`.
- Produces:
  - `pub struct Buffer { pub width: usize, pub height: usize, pub chars: Vec<Vec<char>>, pub colors: Vec<Vec<Color>> }` (derives `Clone`); `Buffer::new(width, height)`, `draw_entity(&mut self, e: &Entity)`.
  - `pub fn composite(buf: &mut Buffer, entities: &[&Entity])` — sorts by depth descending (stable), draws each.

- [ ] **Step 1: Write the failing tests** (append inside `mod tests`)

```rust
    #[test]
    fn buffer_init_blank() {
        let b = Buffer::new(3, 2);
        assert_eq!(b.chars, vec![vec![' '; 3]; 2]);
    }

    #[test]
    fn draw_entity_places_chars_and_skips_spaces() {
        let mut b = Buffer::new(5, 2);
        let e = Entity::new(vec!["a b".into()]).pos(1.0, 0.0).with_color(Color::Green);
        b.draw_entity(&e);
        assert_eq!(b.chars[0][1], 'a');
        assert_eq!(b.chars[0][2], ' '); // espace transparent
        assert_eq!(b.chars[0][3], 'b');
        assert_eq!(b.colors[0][1], Color::Green);
    }

    #[test]
    fn draw_entity_clips_at_edges() {
        let mut b = Buffer::new(3, 1);
        let e = Entity::new(vec!["xyz".into()]).pos(2.0, 0.0);
        b.draw_entity(&e);
        assert_eq!(b.chars[0][2], 'x');
    }

    #[test]
    fn draw_entity_color_mask_overrides() {
        let mut b = Buffer::new(3, 1);
        let e = Entity::new(vec!["ab".into()]).pos(0.0, 0.0).with_color(Color::White)
            .with_mask(vec!["r ".into()]);
        b.draw_entity(&e);
        assert_eq!(b.colors[0][0], Color::Red);
        assert_eq!(b.colors[0][1], Color::White);
    }

    #[test]
    fn composite_nearer_entity_wins() {
        let mut b = Buffer::new(1, 1);
        let far = Entity::new(vec!["F".into()]).with_depth(80);
        let near = Entity::new(vec!["N".into()]).with_depth(30);
        composite(&mut b, &[&near, &far]);
        assert_eq!(b.chars[0][0], 'N');
    }

    #[test]
    fn opaque_animal_hides_grass_in_its_silhouette() {
        let mut buf = Buffer::new(20, 4);
        let grass = Entity::new(vec!["v.,'vv,w.'v,.v'wWvWv".into()]).pos(0.0, 1.0)
            .with_depth(DEPTH_GRASS).with_name("grass");
        let mut animal = Entity::new(vec!["( )".into()]).pos(5.0, 1.0)
            .with_depth(DEPTH_GROUND_ANIMAL).opaque(true);
        composite(&mut buf, &[&grass, &animal]);
        assert_eq!(buf.chars[1][6], ' '); // trou intérieur vidé
        let mut buf2 = Buffer::new(20, 4);
        animal.opaque = false;
        composite(&mut buf2, &[&grass, &animal]);
        assert_ne!(buf2.chars[1][6], ' '); // sans opaque, l'herbe transparaît
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib engine::tests` (compile-blocked until Task 6; verify at Task 6)
Expected: FAIL to compile (`Buffer`, `composite` not defined).

- [ ] **Step 3: Implement** (append to `src/engine.rs`)

```rust
/// Grilles parallèles de caractères et couleurs. Espace = transparent au draw.
#[derive(Clone)]
pub struct Buffer {
    pub width: usize,
    pub height: usize,
    pub chars: Vec<Vec<char>>,
    pub colors: Vec<Vec<Color>>,
}

impl Buffer {
    pub fn new(width: usize, height: usize) -> Self {
        Buffer {
            width,
            height,
            chars: vec![vec![' '; width]; height],
            colors: vec![vec![Color::White; width]; height],
        }
    }

    pub fn draw_entity(&mut self, e: &Entity) {
        let frame = e.current_frame();
        let lines: Vec<&str> = frame.split('\n').collect();
        let mlines: Option<Vec<&str>> = e.current_mask().map(|m| m.split('\n').collect());
        let ox = e.x as i32;
        let oy = e.y as i32;
        for (r, line) in lines.iter().enumerate() {
            let y = oy + r as i32;
            if y < 0 || y >= self.height as i32 {
                continue;
            }
            let chars: Vec<char> = line.chars().collect();
            // Sprite opaque : les trous entre le 1er et le dernier glyphe recouvrent
            // le décor avec du vide au lieu de le laisser transparaître.
            let (mut lo, mut hi): (i32, i32) = (-1, -1);
            if e.opaque {
                let first = chars.iter().position(|&c| c != ' ');
                let last = chars.iter().rposition(|&c| c != ' ');
                if let (Some(f), Some(l)) = (first, last) {
                    lo = f as i32;
                    hi = l as i32;
                }
            }
            for (c, &ch) in chars.iter().enumerate() {
                let ci = c as i32;
                if ch == ' ' && !(lo <= ci && ci <= hi) {
                    continue;
                }
                let x = ox + ci;
                if x < 0 || x >= self.width as i32 {
                    continue;
                }
                let (xu, yu) = (x as usize, y as usize);
                self.chars[yu][xu] = ch; // ch == ' ' => recouvre (opaque)
                let mut color = e.color;
                if ch != ' ' {
                    if let Some(mls) = &mlines {
                        if r < mls.len() {
                            if let Some(mc) = mls[r].chars().nth(c) {
                                if mc != ' ' {
                                    color = mask_color(mc).unwrap_or(e.color);
                                }
                            }
                        }
                    }
                }
                self.colors[yu][xu] = color;
            }
        }
    }
}

/// Dessine les entités triées par profondeur décroissante (algo du peintre).
/// Tri stable : à profondeur égale, l'ordre d'insertion est préservé.
pub fn composite(buf: &mut Buffer, entities: &[&Entity]) {
    let mut order: Vec<&Entity> = entities.to_vec();
    order.sort_by_key(|e| std::cmp::Reverse(e.depth));
    for e in &order {
        buf.draw_entity(e);
    }
}
```

- [ ] **Step 4: Defer verification** — verify at Task 6.

- [ ] **Step 5: Commit** (after Task 6 green).

---

### Task 5: environment forward-need note (no code)

`engine::World` (Task 6) stores `env: Option<crate::environment::Environment>` and builds an `EnvSnapshot` via `Environment::snapshot`. `Environment` is implemented in Task 7. To keep Task 6 compiling before Task 7, Task 6 references `Environment` by path and only calls `.snapshot()` and `.update()` on it — both defined in Task 7. **Therefore implement Tasks 6 and 7 together, and run the first `cargo test` after Task 7.** This task is a checkpoint only — no code, no commit.

- [ ] **Step 1:** Acknowledge the ordering: engine (Tasks 3,4,6) + environment (Task 7) form one compile unit; first green test run is at the end of Task 7.

---

### Task 6: engine — World (add, offscreen, advance, render, env slot)

**Files:**
- Modify: `src/engine.rs`
- Test: `src/engine.rs`

**Interfaces:**
- Consumes: `Entity`, `Buffer`, `composite`, `EnvSnapshot`; `crate::environment::Environment` (Task 7) with methods `snapshot(&self)->EnvSnapshot` and `update(&mut self, dt:f64)`.
- Produces:
  - `pub struct World { pub width: usize, pub height: usize, pub entities: Vec<Entity>, pub env: Option<crate::environment::Environment>, pub rng: rand::rngs::StdRng }`
  - `World::with_rng(width, height, rng) -> World`, `World::new(width, height) -> World` (seed 0), `World::seeded(width, height, seed: u64) -> World`
  - `add(&mut self, e: Entity) -> usize` (returns index)
  - `env_snapshot(&self) -> EnvSnapshot`
  - `advance(&mut self, dt: f64)`, `render(&self) -> Buffer`
  - private `offscreen(&self, e: &Entity) -> bool`

- [ ] **Step 1: Write the failing tests** (append inside `mod tests`)

```rust
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn world_has_generic_env_slot_defaulting_none() {
        let w = World::new(10, 5);
        assert!(w.env.is_none());
    }

    #[test]
    fn world_culls_entity_exiting_its_direction() {
        let mut w = World::new(10, 5);
        w.add(Entity::new(vec!["x".into()]).pos(9.0, 0.0).vel(1.0, 0.0));
        w.advance(2.0);
        assert!(w.entities.is_empty());
    }

    #[test]
    fn world_allows_entry_from_offscreen() {
        let mut w = World::new(10, 5);
        w.add(Entity::new(vec!["xxx".into()]).pos(-3.0, 0.0).vel(1.0, 0.0));
        w.advance(1.0);
        assert_eq!(w.entities.len(), 1);
    }

    #[test]
    fn world_keeps_static_entity() {
        let mut w = World::new(10, 5);
        w.add(Entity::new(vec!["x".into()]).pos(0.0, 0.0));
        w.advance(100.0);
        assert_eq!(w.entities.len(), 1);
    }

    #[test]
    fn world_on_death_called_when_culled() {
        let mut w = World::new(5, 5);
        let hit = Rc::new(Cell::new(false));
        let hit2 = hit.clone();
        let e = Entity::new(vec!["x".into()]).pos(4.0, 0.0).vel(1.0, 0.0)
            .on_death(Box::new(move |_e, _w| hit2.set(true)));
        w.add(e);
        w.advance(5.0);
        assert!(hit.get());
    }

    #[test]
    fn world_removes_dead_flag_entities() {
        let mut w = World::new(5, 5);
        let idx = w.add(Entity::new(vec!["x".into()]).pos(1.0, 1.0));
        w.entities[idx].alive = false;
        w.advance(0.1);
        assert!(w.entities.is_empty());
    }

    #[test]
    fn world_render_composites() {
        let mut w = World::new(2, 1);
        w.add(Entity::new(vec!["a".into()]).pos(0.0, 0.0).with_depth(10));
        let buf = w.render();
        assert_eq!(buf.chars[0][0], 'a');
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib engine` — after Task 7 is also implemented.
Expected: initially FAIL to compile (`World` missing / `Environment` missing).

- [ ] **Step 3: Implement** (append to `src/engine.rs`; add imports at top of file)

Add to the imports region near the top of `src/engine.rs`:

```rust
use rand::rngs::StdRng;
use rand::SeedableRng;
```

Append:

```rust
/// Monde : dimensions, entités, environnement optionnel, RNG maître.
pub struct World {
    pub width: usize,
    pub height: usize,
    pub entities: Vec<Entity>,
    pub env: Option<crate::environment::Environment>,
    pub rng: StdRng,
}

impl World {
    pub fn with_rng(width: usize, height: usize, rng: StdRng) -> Self {
        World {
            width,
            height,
            entities: Vec::new(),
            env: None,
            rng,
        }
    }
    pub fn new(width: usize, height: usize) -> Self {
        World::with_rng(width, height, StdRng::seed_from_u64(0))
    }
    pub fn seeded(width: usize, height: usize, seed: u64) -> Self {
        World::with_rng(width, height, StdRng::seed_from_u64(seed))
    }

    pub fn add(&mut self, e: Entity) -> usize {
        self.entities.push(e);
        self.entities.len() - 1
    }

    pub fn env_snapshot(&self) -> EnvSnapshot {
        match &self.env {
            Some(env) => env.snapshot(),
            None => EnvSnapshot::none(),
        }
    }

    fn offscreen(&self, e: &Entity) -> bool {
        if e.dx > 0.0 && e.x >= self.width as f64 {
            return true;
        }
        if e.dx < 0.0 && e.x + e.width() as f64 <= 0.0 {
            return true;
        }
        if e.dy > 0.0 && e.y >= self.height as f64 {
            return true;
        }
        if e.dy < 0.0 && e.y + e.height() as f64 <= 0.0 {
            return true;
        }
        false
    }

    pub fn advance(&mut self, dt: f64) {
        let snap = self.env_snapshot();
        for e in &mut self.entities {
            e.advance(dt, snap);
        }
        let taken = std::mem::take(&mut self.entities);
        let mut kept: Vec<Entity> = Vec::new();
        let mut dead: Vec<Entity> = Vec::new();
        for e in taken {
            if e.alive && !self.offscreen(&e) {
                kept.push(e);
            } else {
                dead.push(e);
            }
        }
        self.entities = kept;
        for mut e in dead {
            if let Some(mut f) = e.on_death.take() {
                f(&mut e, self);
            }
        }
    }

    pub fn render(&self) -> Buffer {
        let mut buf = Buffer::new(self.width, self.height);
        let refs: Vec<&Entity> = self.entities.iter().collect();
        composite(&mut buf, &refs);
        buf
    }
}
```

- [ ] **Step 4: Run engine tests to verify they pass** (after Task 7 implemented)

Run: `cargo test --lib engine`
Expected: all engine tests (Tasks 2,3,4,6) pass.

- [ ] **Step 5: Commit**

```bash
git add src/engine.rs
git commit -m "feat(engine): Entity, Behavior, Buffer, composite, World"
```

---

### Task 7: environment — Weather + Environment + snapshot

**Files:**
- Modify: `src/environment.rs`
- Test: `src/environment.rs`

**Interfaces:**
- Consumes: `crate::engine::EnvSnapshot`.
- Produces:
  - `pub enum Weather { Clear, Wind, Rain, Storm }` (derives `Clone, Copy, PartialEq, Eq, Debug, Hash`)
  - `pub const WIND_SLANT: f64`
  - `pub struct Environment { pub day_length: f64, pub(crate) rng: StdRng, pub t: f64, pub weather: Weather, pub wind_direction: i32, pub weather_timer: f64 }`
  - `Environment::new(day_length: f64, rng: StdRng) -> Self`, `Environment::seeded(day_length: f64, seed: u64) -> Self`
  - methods: `update(&mut self, dt: f64)`, `phase(&self)->f64`, `is_night(&self)->bool`, `raining/windy/storming(&self)->bool`, `wind_dx(&self)->f64`, `snapshot(&self)->EnvSnapshot`.

- [ ] **Step 1: Write the failing tests** (append to `src/environment.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn phase_wraps_within_unit_interval() {
        let mut env = Environment::seeded(100.0, 0);
        env.update(30.0);
        assert!(env.phase() >= 0.0 && env.phase() < 1.0);
        assert!((env.phase() - 0.3).abs() < 1e-9);
    }

    #[test]
    fn phase_returns_to_start_after_full_day() {
        let mut env = Environment::seeded(50.0, 0);
        env.update(50.0);
        assert!((env.phase() - 0.0).abs() < 1e-9);
    }

    #[test]
    fn is_night_false_in_first_half() {
        let mut env = Environment::seeded(100.0, 0);
        env.update(10.0);
        assert!(!env.is_night());
    }

    #[test]
    fn is_night_true_at_half_phase_boundary() {
        let mut env = Environment::seeded(100.0, 0);
        env.update(50.0);
        assert!(env.is_night());
    }

    #[test]
    fn seeded_rng_gives_deterministic_weather_sequence() {
        fn run() -> Vec<Weather> {
            let mut env = Environment::seeded(1000.0, 1234);
            let mut seq = Vec::new();
            for _ in 0..500 {
                env.update(1.0);
                seq.push(env.weather);
            }
            seq
        }
        assert_eq!(run(), run());
    }

    #[test]
    fn weather_changes_over_time() {
        let mut env = Environment::seeded(10000.0, 1234);
        let mut seen: HashSet<Weather> = HashSet::new();
        for _ in 0..3000 {
            env.update(1.0);
            seen.insert(env.weather);
        }
        assert!(seen.len() > 1);
    }

    #[test]
    fn clear_has_no_weather_effects() {
        let mut env = Environment::seeded(90.0, 0);
        env.weather = Weather::Clear;
        assert!(!env.raining());
        assert!(!env.windy());
        assert!(!env.storming());
        assert_eq!(env.wind_dx(), 0.0);
    }

    #[test]
    fn rain_is_raining_only() {
        let mut env = Environment::seeded(90.0, 0);
        env.weather = Weather::Rain;
        assert!(env.raining());
        assert!(!env.windy());
        assert!(!env.storming());
    }

    #[test]
    fn wind_is_windy_only() {
        let mut env = Environment::seeded(90.0, 0);
        env.weather = Weather::Wind;
        assert!(env.windy());
        assert!(!env.raining());
        assert!(!env.storming());
    }

    #[test]
    fn storm_implies_rain_wind_and_lightning() {
        let mut env = Environment::seeded(90.0, 0);
        env.weather = Weather::Storm;
        assert!(env.raining());
        assert!(env.windy());
        assert!(env.storming());
    }

    #[test]
    fn wind_dx_zero_when_calm_signed_when_windy() {
        let mut env = Environment::seeded(90.0, 0);
        env.weather = Weather::Clear;
        assert_eq!(env.wind_dx(), 0.0);
        env.weather = Weather::Wind;
        assert_ne!(env.wind_dx(), 0.0);
        env.wind_direction = 1;
        assert!(env.wind_dx() > 0.0);
        env.wind_direction = -1;
        assert!(env.wind_dx() < 0.0);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib environment`
Expected: FAIL to compile (`Environment`, `Weather` missing).

- [ ] **Step 3: Implement** (top of `src/environment.rs`)

```rust
//! Environnement global : horloge jour/nuit + météo (pur, sans crossterm).
//!
//! Créé par la scène, posé sur `world.env`, tické une fois par frame dans
//! `spawn::step`. Les spawners lisent `world.env` pour décider quoi faire
//! apparaître. Le moteur reste agnostique de la météo.

use crate::engine::EnvSnapshot;
use rand::distributions::{Distribution, WeightedIndex};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// États de la machine météo.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Weather {
    Clear,
    Wind,
    Rain,
    Storm,
}

const WEATHER_STATES: [Weather; 4] =
    [Weather::Clear, Weather::Wind, Weather::Rain, Weather::Storm];
const WEATHER_WEIGHTS: [u32; 4] = [6, 2, 2, 1]; // CLEAR domine => calme par défaut
const DWELL_MIN: f64 = 8.0;
const DWELL_MAX: f64 = 20.0;
pub const WIND_SLANT: f64 = 6.0; // magnitude du dx (pluie/vent) quand il vente

pub struct Environment {
    pub day_length: f64,       // secondes pour un cycle jour+nuit complet
    pub(crate) rng: StdRng,    // injectable pour des tests déterministes
    pub t: f64,
    pub weather: Weather,
    pub wind_direction: i32,
    pub weather_timer: f64,
}

impl Environment {
    pub fn new(day_length: f64, mut rng: StdRng) -> Self {
        let wind_direction = if rng.gen::<bool>() { 1 } else { -1 };
        let weather_timer = rng.gen_range(DWELL_MIN..DWELL_MAX);
        Environment {
            day_length,
            rng,
            t: 0.0,
            weather: Weather::Clear,
            wind_direction,
            weather_timer,
        }
    }

    pub fn seeded(day_length: f64, seed: u64) -> Self {
        Environment::new(day_length, StdRng::seed_from_u64(seed))
    }

    /// Avance l'horloge et le minuteur météo.
    pub fn update(&mut self, dt: f64) {
        self.t += dt;
        self.weather_timer -= dt;
        if self.weather_timer <= 0.0 {
            self.next_weather();
        }
    }

    fn next_weather(&mut self) {
        let dist = WeightedIndex::new(WEATHER_WEIGHTS).unwrap();
        self.weather = WEATHER_STATES[dist.sample(&mut self.rng)];
        self.wind_direction = if self.rng.gen::<bool>() { 1 } else { -1 };
        self.weather_timer = self.rng.gen_range(DWELL_MIN..DWELL_MAX);
    }

    // --- heure du jour ---
    pub fn phase(&self) -> f64 {
        (self.t / self.day_length).rem_euclid(1.0)
    }
    pub fn is_night(&self) -> bool {
        self.phase() >= 0.5 // jour = première moitié
    }

    // --- requêtes météo ---
    pub fn raining(&self) -> bool {
        matches!(self.weather, Weather::Rain | Weather::Storm)
    }
    pub fn windy(&self) -> bool {
        matches!(self.weather, Weather::Wind | Weather::Storm)
    }
    pub fn storming(&self) -> bool {
        self.weather == Weather::Storm
    }
    pub fn wind_dx(&self) -> f64 {
        if self.windy() {
            self.wind_direction as f64 * WIND_SLANT
        } else {
            0.0
        }
    }

    pub fn snapshot(&self) -> EnvSnapshot {
        EnvSnapshot {
            is_day: !self.is_night(),
            is_night: self.is_night(),
            raining: self.raining(),
            windy: self.windy(),
            storming: self.storming(),
            wind_dx: self.wind_dx(),
        }
    }
}
```

- [ ] **Step 4: Run environment + engine tests to verify they pass**

Run: `cargo test --lib`
Expected: all `engine` and `environment` tests pass (this is the first full green run — Tasks 3,4,6,7 unblock together).

- [ ] **Step 5: Commit**

```bash
git add src/environment.rs
git commit -m "feat(environment): day/night clock + weather state machine"
```

---

### Task 8: spawn — Target, SpawnSpec, Spawner, step

**Files:**
- Modify: `src/spawn.rs`
- Test: `src/spawn.rs`

**Interfaces:**
- Consumes: `crate::engine::{Entity, World}`.
- Produces:
  - `pub enum Target { Fixed(i32), Dynamic(fn(&World) -> i32) }` with `resolve(&self, world: &World) -> i32`
  - `pub struct SpawnSpec { pub name: String, pub factory: fn(&mut World) -> Option<Entity>, pub target: Target, pub chance: f64 }`
  - `pub struct Spawner { pub specs: Vec<SpawnSpec> }` with `new()`, `register(&mut self, name: &str, factory: fn(&mut World)->Option<Entity>, target: Target, chance: f64)`, `tick(&self, world: &mut World, dt: f64)`
  - `pub fn step(world: &mut World, spawner: &Spawner, dt: f64)`

- [ ] **Step 1: Write the failing tests** (append to `src/spawn.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Entity, World};
    use crate::environment::Environment;

    fn make_world() -> World {
        World::new(20, 10)
    }

    fn bug(_w: &mut World) -> Option<Entity> {
        Some(Entity::new(vec!["x".into()]).pos(1.0, 1.0).with_name("bug"))
    }

    fn count(w: &World, name: &str) -> usize {
        w.entities.iter().filter(|e| e.name.as_deref() == Some(name)).count()
    }

    #[test]
    fn spawner_fills_to_target() {
        let mut w = make_world();
        let mut sp = Spawner::new();
        sp.register("bug", bug, Target::Fixed(2), 1.0);
        for _ in 0..5 {
            sp.tick(&mut w, 1.0);
        }
        assert_eq!(count(&w, "bug"), 2);
    }

    #[test]
    fn spawner_respects_chance_zero() {
        let mut w = make_world();
        let mut sp = Spawner::new();
        sp.register("bug", bug, Target::Fixed(3), 0.0);
        sp.tick(&mut w, 1.0);
        assert_eq!(count(&w, "bug"), 0);
    }

    #[test]
    fn step_advances_then_spawns() {
        let mut w = make_world();
        let mut sp = Spawner::new();
        sp.register("bug", bug, Target::Fixed(1), 1.0);
        w.add(Entity::new(vec!["x".into()]).pos(1.0, 1.0).vel(1.0, 0.0).with_name("mover"));
        step(&mut w, &sp, 1.0);
        let mover = w.entities.iter().find(|e| e.name.as_deref() == Some("mover")).unwrap();
        assert_eq!(mover.x, 2.0);
        assert_eq!(count(&w, "bug"), 1);
    }

    #[test]
    fn callable_target_resolves_through_tick() {
        let mut w = make_world();
        let mut sp = Spawner::new();
        sp.register("bug", bug, Target::Dynamic(|_w| 2), 1.0);
        for _ in 0..5 {
            sp.tick(&mut w, 1.0);
        }
        assert_eq!(count(&w, "bug"), 2);
    }

    #[test]
    fn callable_target_zero_blocks_spawn() {
        let mut w = make_world();
        let mut sp = Spawner::new();
        sp.register("bug", bug, Target::Dynamic(|_w| 0), 1.0);
        for _ in 0..5 {
            sp.tick(&mut w, 1.0);
        }
        assert_eq!(count(&w, "bug"), 0);
    }

    // Prouve que step ticke l'env AVANT que le spawner observe l'état :
    // la factory ne spawne que si la phase a déjà avancé (> 0).
    fn phase_gated(w: &mut World) -> Option<Entity> {
        if w.env.as_ref().unwrap().phase() > 0.0 {
            Some(Entity::new(vec!["x".into()]).with_name("bug"))
        } else {
            None
        }
    }

    #[test]
    fn step_ticks_env_before_spawn() {
        let mut w = make_world();
        w.env = Some(Environment::seeded(1000.0, 0));
        let mut sp = Spawner::new();
        sp.register("bug", phase_gated, Target::Fixed(1), 1.0);
        step(&mut w, &sp, 1.0);
        assert_eq!(count(&w, "bug"), 1);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib spawn`
Expected: FAIL to compile (`Spawner`, `Target`, `step` missing).

- [ ] **Step 3: Implement** (top of `src/spawn.rs`)

```rust
//! Gestion de population : maintient un nombre cible d'entités par type.

use crate::engine::{Entity, World};
use rand::Rng;

/// Cible d'une spec : nombre fixe ou fonction dynamique gatée sur `world.env`.
pub enum Target {
    Fixed(i32),
    Dynamic(fn(&World) -> i32),
}

impl Target {
    pub fn resolve(&self, world: &World) -> i32 {
        match self {
            Target::Fixed(n) => *n,
            Target::Dynamic(f) => f(world),
        }
    }
}

pub struct SpawnSpec {
    pub name: String,
    pub factory: fn(&mut World) -> Option<Entity>,
    pub target: Target,
    pub chance: f64,
}

pub struct Spawner {
    pub specs: Vec<SpawnSpec>,
}

impl Spawner {
    pub fn new() -> Self {
        Spawner { specs: Vec::new() }
    }

    pub fn register(
        &mut self,
        name: &str,
        factory: fn(&mut World) -> Option<Entity>,
        target: Target,
        chance: f64,
    ) {
        self.specs.push(SpawnSpec {
            name: name.to_string(),
            factory,
            target,
            chance,
        });
    }

    pub fn tick(&self, world: &mut World, dt: f64) {
        for spec in &self.specs {
            let target = spec.target.resolve(world);
            let count = world
                .entities
                .iter()
                .filter(|e| e.name.as_deref() == Some(spec.name.as_str()))
                .count() as i32;
            if count < target && world.rng.gen::<f64>() < spec.chance * dt {
                if let Some(e) = (spec.factory)(world) {
                    world.add(e);
                }
            }
        }
    }
}

impl Default for Spawner {
    fn default() -> Self {
        Spawner::new()
    }
}

pub fn step(world: &mut World, spawner: &Spawner, dt: f64) {
    if let Some(env) = world.env.as_mut() {
        env.update(dt); // avance horloge+météo avant que les spawners lisent l'état
    }
    world.advance(dt);
    spawner.tick(world, dt);
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib spawn`
Expected: 6 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/spawn.rs
git commit -m "feat(spawn): Spawner population control + step frame tick"
```

---

### Task 9: art — transcribe ASCII data + alignment tests

**Files:**
- Modify: `src/art.rs`
- Reference: `asciimeadow/art.py` (still present — do NOT delete until Task 16)
- Test: `src/art.rs`

**Interfaces:**
- Produces `pub const` items (types below). Single multi-line sprites are `&str`; frame lists are `[&str; N]`.

**Transcription rules (read carefully):**
1. Copy every symbol from `asciimeadow/art.py` **verbatim**, preserving every character including **trailing spaces** (masks align column-for-column; a dropped trailing space breaks the alignment test). Configure your editor to NOT trim trailing whitespace in this file.
2. Use **raw string literals with real embedded newlines** so backslashes stay literal:
   - No double-quote in the art → `r"..."`.
   - Contains a double-quote (`OWL`, `HEDGEHOG`, `MOUSE`, and their masks) → `r#"..."#`.
3. Type mapping (Python → Rust):

| Python name(s) | Rust type |
|---|---|
| `SUN, SUN_MASK, TREE_SMALL(_MASK), TREE_LARGE(_MASK), CLOUD_SMALL(_MASK), CLOUD_LARGE(_MASK), HEDGEHOG(_MASK), MOUSE(_MASK), MOON(_MASK), LIGHTNING(_MASK), APPLE, LEAF, SNAIL` | `pub const X: &str` |
| `GRASS_ROWS` | `pub const GRASS_ROWS: [&str; 4]` |
| `FLOWERS` | `pub const FLOWERS: [&str; 3]` |
| `BIRD(_MASK), OWL(_MASK), RABBIT(_MASK), FOX(_MASK), BUTTERFLY, BEE, FIREFLY` | `pub const X: [&str; 2]` |
| `STAR_CHARS` | `pub const STAR_CHARS: [&str; 4]` |
| `RAIN_CHARS` | `pub const RAIN_CHARS: [&str; 3]` |
| `WIND_CHARS` | `pub const WIND_CHARS: [&str; 3]` |

4. `OWL_MASK` in Python is `[mask] * 2` — write the same mask string **twice** explicitly.
5. `CLOUD = CLOUD_SMALL` retro-compat alias is NOT needed — drop it (nothing references it).

- [ ] **Step 1: Transcribe the data** (fill `src/art.rs`)

Representative examples showing the exact format (transcribe ALL symbols this way):

```rust
//! Données ASCII pures (aucune logique). Masques : voir engine::mask_color.

pub const SUN: &str = r" \ | / 
- (O) -
 / | \ ";
pub const SUN_MASK: &str = r" y y y 
y yyy y
 y y y ";

pub const GRASS_ROWS: [&str; 4] = [
    "vWv,vw'vvW.wv,v",
    ",w'v.vW,v'wv,W.",
    "v.,'vv,w.'v,.v'",
    ",'.,v.',.,'v.,.",
];
pub const FLOWERS: [&str; 3] = ["*", "o", "@"];

pub const BIRD: [&str; 2] = [
r"  \ \ 
__( o>
      ",
r"      
__( o>
  / / ",
];
pub const BIRD_MASK: [&str; 2] = [
r"  w w 
www ky
      ",
r"      
www ky
  w w ",
];

// Contient des guillemets -> r#"..."#
pub const OWL: [&str; 2] = [
r#" ^ ^ 
(O,O)
(:v:)
 " " "#,
r#" ^ ^ 
(-,-)
(:v:)
 " " "#,
];
pub const OWL_MASK: [&str; 2] = [
r#" n n 
nyyyn
nnwnn
 y y "#,
r#" n n 
nyyyn
nnwnn
 y y "#,
];

pub const STAR_CHARS: [&str; 4] = [".", "*", "+", "'"];
pub const RAIN_CHARS: [&str; 3] = ["|", "/", "\\"];
pub const FIREFLY: [&str; 2] = ["*", " "];
pub const WIND_CHARS: [&str; 3] = [",", "~", "'"];
```

Remaining symbols to transcribe from `art.py` following the same rules (do not skip any):
`TREE_SMALL`, `TREE_SMALL_MASK`, `TREE_LARGE`, `TREE_LARGE_MASK`, `CLOUD_SMALL`, `CLOUD_SMALL_MASK`, `CLOUD_LARGE`, `CLOUD_LARGE_MASK`, `BUTTERFLY`, `BEE`, `APPLE`, `LEAF`, `RABBIT`, `RABBIT_MASK`, `FOX`, `FOX_MASK`, `HEDGEHOG`, `HEDGEHOG_MASK`, `MOUSE`, `MOUSE_MASK`, `SNAIL`, `MOON`, `MOON_MASK`, `LIGHTNING`, `LIGHTNING_MASK`.

- [ ] **Step 2: Write the alignment + shape tests** (append to `src/art.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // (frames, masks) à aligner char-pour-char. Chaque paire = un sprite masqué.
    fn masked_pairs() -> Vec<(Vec<&'static str>, Vec<&'static str>)> {
        vec![
            (vec![SUN], vec![SUN_MASK]),
            (vec![TREE_SMALL], vec![TREE_SMALL_MASK]),
            (vec![TREE_LARGE], vec![TREE_LARGE_MASK]),
            (vec![CLOUD_SMALL], vec![CLOUD_SMALL_MASK]),
            (vec![CLOUD_LARGE], vec![CLOUD_LARGE_MASK]),
            (BIRD.to_vec(), BIRD_MASK.to_vec()),
            (OWL.to_vec(), OWL_MASK.to_vec()),
            (RABBIT.to_vec(), RABBIT_MASK.to_vec()),
            (FOX.to_vec(), FOX_MASK.to_vec()),
            (vec![HEDGEHOG], vec![HEDGEHOG_MASK]),
            (vec![MOUSE], vec![MOUSE_MASK]),
            (vec![MOON], vec![MOON_MASK]),
            (vec![LIGHTNING], vec![LIGHTNING_MASK]),
        ]
    }

    #[test]
    fn every_mask_matches_its_frames() {
        let pairs = masked_pairs();
        assert!(pairs.len() >= 9);
        for (frames, masks) in pairs {
            assert_eq!(frames.len(), masks.len());
            for (f, m) in frames.iter().zip(masks.iter()) {
                let fl: Vec<&str> = f.split('\n').collect();
                let ml: Vec<&str> = m.split('\n').collect();
                assert_eq!(fl.len(), ml.len());
                for (a, b) in fl.iter().zip(ml.iter()) {
                    assert_eq!(a.chars().count(), b.chars().count(), "mask misaligned");
                }
            }
        }
    }

    #[test]
    fn bird_has_two_flap_frames() {
        assert_eq!(BIRD.len(), 2);
        assert_eq!(BIRD_MASK.len(), 2);
    }

    #[test]
    fn flowers_non_empty() {
        assert!(!FLOWERS.is_empty());
        assert!(FLOWERS.iter().all(|f| !f.is_empty()));
    }

    #[test]
    fn moon_and_lightning_are_multiline() {
        assert!(MOON.contains('\n'));
        assert!(LIGHTNING.contains('\n'));
    }

    #[test]
    fn single_glyph_char_sets() {
        assert!(STAR_CHARS.len() >= 2);
        assert!(STAR_CHARS.iter().all(|c| c.chars().count() == 1));
        assert!(RAIN_CHARS.iter().all(|c| c.chars().count() == 1));
        assert!(WIND_CHARS.iter().all(|c| c.chars().count() == 1));
    }

    #[test]
    fn firefly_blinks_over_two_frames() {
        assert_eq!(FIREFLY.len(), 2);
    }

    #[test]
    fn ground_animals_are_multiline_and_masked() {
        for (frames, masks) in [(RABBIT, RABBIT_MASK), (FOX, FOX_MASK)] {
            assert_eq!(frames.len(), 2);
            assert_eq!(masks.len(), 2);
            assert!(frames.iter().all(|f| f.split('\n').count() >= 3));
        }
        for sprite in [HEDGEHOG, MOUSE] {
            assert!(sprite.split('\n').count() >= 3);
        }
    }
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test --lib art`
Expected: all art tests pass. If `every_mask_matches_its_frames` fails with "mask misaligned", a trailing space was trimmed — re-check that sprite against `art.py`.

- [ ] **Step 4: Commit**

```bash
git add src/art.rs
git commit -m "feat(art): transcribe ASCII sprites + color masks"
```

---

### Task 10: scene — geometry helpers + is_day/is_night + build_meadow

**Files:**
- Modify: `src/scene.rs`
- Test: `src/scene.rs`

**Interfaces:**
- Consumes: `art`, `engine::{Entity, Color, World, depth consts}`, `environment::Environment`, `rand`.
- Produces:
  - `pub const GROUND_ROWS: i32 = 4;`
  - `pub fn ground_top(world: &World) -> i32` (= `height - GROUND_ROWS`)
  - `pub fn tree_art(world: &World) -> (&'static str, &'static str)`
  - `pub fn tree_origin(world: &World) -> (i32, i32)`
  - `pub fn cloud_art(world: &World) -> (&'static str, &'static str)`
  - `pub fn ground_frames(width: usize) -> Vec<String>`
  - `pub fn is_day(world: &World) -> bool`, `pub fn is_night(world: &World) -> bool`
  - `pub fn build_meadow(world: &mut World, day_length: f64)`
- Note: helper `fn max_line_width(frame: &str) -> i32` (private) used by several factories; define it here.

- [ ] **Step 1: Write the failing tests** (append to `src/scene.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::art;
    use crate::engine::{self, World};
    use crate::environment::Environment;

    #[test]
    fn ground_top_leaves_band() {
        let w = World::new(40, 20);
        assert_eq!(ground_top(&w), 16);
    }

    #[test]
    fn tree_art_selects_variant_by_size() {
        let small = World::new(80, 24); // hauteur < 28 => petit
        let large = World::new(100, 35);
        let narrow = World::new(39, 35); // trop étroit => petit
        assert!(std::ptr::eq(tree_art(&small).0, art::TREE_SMALL));
        assert!(std::ptr::eq(tree_art(&large).0, art::TREE_LARGE));
        assert!(std::ptr::eq(tree_art(&narrow).0, art::TREE_SMALL));
    }

    #[test]
    fn tree_is_centered() {
        let w = World::new(60, 24);
        let (ox, _) = tree_origin(&w);
        let (frame, _) = tree_art(&w);
        let tree_w = max_line_width(frame);
        let center = ox as f64 + tree_w as f64 / 2.0;
        assert!((center - 30.0).abs() <= 1.0);
    }

    #[test]
    fn large_tree_fits_on_screen() {
        let w = World::new(100, 35);
        let (ox, oy) = tree_origin(&w);
        assert!(oy >= 0);
        assert!(ox >= 0);
    }

    #[test]
    fn ground_frames_fill_all_rows() {
        let frames = ground_frames(30);
        assert_eq!(frames.len(), 2);
        for f in &frames {
            let lines: Vec<&str> = f.split('\n').collect();
            assert_eq!(lines.len(), GROUND_ROWS as usize);
            for line in lines {
                assert_eq!(line.chars().count(), 30);
                assert!(!line.trim().is_empty());
            }
        }
    }

    #[test]
    fn build_meadow_adds_scenery() {
        let mut w = World::seeded(60, 24, 0);
        build_meadow(&mut w, 90.0);
        let names: std::collections::HashSet<&str> =
            w.entities.iter().filter_map(|e| e.name.as_deref()).collect();
        assert!(names.contains("tree"));
        assert!(names.contains("ground"));
    }

    #[test]
    fn build_meadow_installs_environment() {
        let mut w = World::seeded(60, 24, 0);
        build_meadow(&mut w, 90.0);
        assert!(w.env.is_some());
    }

    #[test]
    fn ground_spans_full_width() {
        let mut w = World::seeded(50, 20, 0);
        build_meadow(&mut w, 90.0);
        let ground = w.entities.iter().find(|e| e.name.as_deref() == Some("ground")).unwrap();
        assert_eq!(ground.width(), 50);
    }

    #[test]
    fn ground_band_at_grass_depth() {
        let mut w = World::seeded(50, 20, 0);
        build_meadow(&mut w, 90.0);
        let ground = w.entities.iter().find(|e| e.name.as_deref() == Some("ground")).unwrap();
        assert_eq!(ground.depth, engine::DEPTH_GRASS);
    }

    #[test]
    fn flowers_spread_across_band() {
        let mut w = World::seeded(60, 24, 5);
        build_meadow(&mut w, 90.0);
        let gy = ground_top(&w) as f64;
        let flowers: Vec<_> = w.entities.iter().filter(|e| e.name.as_deref() == Some("flower")).collect();
        assert!(!flowers.is_empty());
        assert!(flowers.iter().all(|e| gy <= e.y && e.y <= (w.height as f64 - 1.0)));
    }

    #[test]
    fn is_day_night_track_env() {
        let mut day = World::new(80, 24);
        day.env = Some(Environment::seeded(100.0, 0)); // phase 0 => jour
        assert!(is_day(&day));
        assert!(!is_night(&day));
        let mut night = World::new(80, 24);
        let mut env = Environment::seeded(100.0, 0);
        env.update(50.0); // phase 0.5 => nuit
        night.env = Some(env);
        assert!(is_night(&night));
        assert!(!is_day(&night));
        let no_env = World::new(80, 24);
        assert!(is_day(&no_env)); // pas d'env => jour
        assert!(!is_night(&no_env));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib scene`
Expected: FAIL to compile (functions missing).

- [ ] **Step 3: Implement** (top of `src/scene.rs`)

```rust
//! Construction de la prairie : géométrie, factories, spawners.

use crate::art;
use crate::engine::{self, Behavior, Color, Entity, World};
use crate::environment::Environment;
use rand::rngs::StdRng;
use rand::{Rng, RngCore, SeedableRng};

pub const GROUND_ROWS: i32 = 4;

/// Largeur max (en glyphes) parmi les lignes d'une frame.
fn max_line_width(frame: &str) -> i32 {
    frame.split('\n').map(|l| l.chars().count()).max().unwrap_or(0) as i32
}

pub fn ground_top(world: &World) -> i32 {
    world.height as i32 - GROUND_ROWS
}

/// Variante d'arbre selon la taille du monde (le resize resélectionne).
pub fn tree_art(world: &World) -> (&'static str, &'static str) {
    if world.height >= 28 && world.width >= 40 {
        (art::TREE_LARGE, art::TREE_LARGE_MASK)
    } else {
        (art::TREE_SMALL, art::TREE_SMALL_MASK)
    }
}

pub fn tree_origin(world: &World) -> (i32, i32) {
    let (frame, _) = tree_art(world);
    let tree_w = max_line_width(frame);
    let tree_h = frame.split('\n').count() as i32;
    let ox = world.width as i32 / 2 - tree_w / 2;
    let oy = ground_top(world) - tree_h;
    (ox, oy)
}

/// Variante de nuage selon la taille du monde (grand nuage si large).
pub fn cloud_art(world: &World) -> (&'static str, &'static str) {
    if world.height >= 24 && world.width >= 60 {
        (art::CLOUD_LARGE, art::CLOUD_LARGE_MASK)
    } else {
        (art::CLOUD_SMALL, art::CLOUD_SMALL_MASK)
    }
}

pub fn ground_frames(width: usize) -> Vec<String> {
    let rows: Vec<String> = art::GRASS_ROWS
        .iter()
        .map(|p| {
            let reps = width / p.chars().count() + 1;
            p.repeat(reps).chars().take(width).collect()
        })
        .collect();
    let shifted: Vec<String> = rows
        .iter()
        .map(|r| {
            let chars: Vec<char> = r.chars().collect();
            let mut s: String = chars[1..].iter().collect();
            s.push(chars[0]);
            s
        })
        .collect();
    vec![rows.join("\n"), shifted.join("\n")]
}

/// Pas d'env => on considère qu'il fait jour (compat : prairie diurne).
pub fn is_day(world: &World) -> bool {
    match &world.env {
        None => true,
        Some(env) => !env.is_night(),
    }
}
pub fn is_night(world: &World) -> bool {
    match &world.env {
        None => false,
        Some(env) => env.is_night(),
    }
}

pub fn build_meadow(world: &mut World, day_length: f64) {
    // Environnement global (horloge jour/nuit + météo), rng dérivé du rng maître.
    let env_seed = world.rng.next_u64();
    world.env = Some(Environment::new(day_length, StdRng::seed_from_u64(env_seed)));

    // Arbre — centré, base au sol, variante selon la taille du terminal.
    let (tf, tm) = tree_art(world);
    let (tox, toy) = tree_origin(world);
    world.add(
        Entity::new(vec![tf.to_string()])
            .with_mask(vec![tm.to_string()])
            .pos(tox as f64, toy as f64)
            .with_depth(engine::DEPTH_TREE)
            .with_color(Color::Green)
            .with_name("tree"),
    );

    // Sol — pleine largeur, herbe dense ondulante, derrière les animaux du sol.
    let gframes = ground_frames(world.width);
    world.add(
        Entity::new(gframes)
            .pos(0.0, ground_top(world) as f64)
            .with_depth(engine::DEPTH_GRASS)
            .with_frame_rate(2.0)
            .with_color(Color::Green)
            .with_name("ground"),
    );

    // Fleurs — dispersées sur toute la bande d'herbe.
    let gy = ground_top(world);
    let count = std::cmp::max(3, world.width / 12);
    for _ in 0..count {
        let fx = world.rng.gen_range(0..=world.width as i32 - 1) as f64;
        let fy = world.rng.gen_range(gy..=world.height as i32 - 1) as f64;
        let flower = art::FLOWERS[world.rng.gen_range(0..art::FLOWERS.len())];
        let palette = [Color::Red, Color::Yellow, Color::Magenta, Color::White];
        let color = palette[world.rng.gen_range(0..palette.len())];
        world.add(
            Entity::new(vec![flower.to_string()])
                .pos(fx, fy)
                .with_depth(engine::DEPTH_FOREGROUND)
                .with_color(color)
                .with_name("flower"),
        );
    }
}
```

Note: `Behavior` is imported now but first used in Task 11 — the compiler will warn "unused import" until then. That is acceptable mid-task; it is used by the end of Task 11. (Alternatively add the import in Task 11.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib scene`
Expected: the 11 geometry/build tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/scene.rs
git commit -m "feat(scene): geometry helpers, day/night predicates, build_meadow"
```

---

### Task 11: scene — cross_factory + sky/tree creature factories

**Files:**
- Modify: `src/scene.rs`
- Test: `src/scene.rs`

**Interfaces:**
- Consumes: geometry helpers (Task 10), `flip_horizontal`, `Behavior`.
- Produces (all `pub fn … (world: &mut World) -> Option<Entity>` unless noted):
  - `fn cross_factory(world, frames: &[String], depth: i32, speed: f64, y: f64, color: Color, name: &str, mask: Option<&[String]>, frame_rate: f64) -> Entity` (private)
  - `fn celestial_x(world: &World, frame: &str) -> f64` (private)
  - `spawn_bird, spawn_cloud, spawn_butterfly, spawn_owl, spawn_bee, spawn_sun, spawn_moon, spawn_star, spawn_firefly`

- [ ] **Step 1: Write the failing tests** (append inside scene `mod tests`)

```rust
    #[test]
    fn cross_factory_sets_direction_and_depth() {
        let mut w = World::seeded(60, 24, 1);
        let b = spawn_bird(&mut w).unwrap();
        assert_eq!(b.name.as_deref(), Some("bird"));
        assert!(b.dx != 0.0);
        assert_eq!(b.depth, engine::DEPTH_SKY_CREATURE);
        assert!(b.color_mask.is_some());
    }

    #[test]
    fn owl_is_resident_and_perched() {
        for (wd, ht) in [(60, 24), (100, 35)] {
            let mut w = World::seeded(wd, ht, 3);
            let o = spawn_owl(&mut w).unwrap();
            assert_eq!(o.dx, 0.0);
            assert_eq!(o.dy, 0.0);
            let (_, toy) = tree_origin(&w);
            let (frame, _) = tree_art(&w);
            let tree_h = frame.split('\n').count() as i32;
            assert!(o.y as i32 > toy && o.y as i32 <= toy + tree_h / 2 + 1);
            assert!(o.color_mask.is_some());
        }
    }

    #[test]
    fn owl_cull_kills_when_day_returns() {
        let mut w = World::new(80, 24);
        let mut env = Environment::seeded(100.0, 0);
        env.update(50.0); // nuit
        w.env = Some(env);
        let mut o = spawn_owl(&mut w).unwrap();
        o.advance(0.1, w.env_snapshot());
        assert!(o.alive); // la nuit : le hibou vit
        w.env.as_mut().unwrap().t = 0.0; // bascule jour
        o.advance(0.1, w.env_snapshot());
        assert!(!o.alive); // culled quand la nuit se ferme
    }

    #[test]
    fn butterfly_and_bee_are_day_gated_movers() {
        let mut w = World::seeded(80, 24, 2);
        let bf = spawn_butterfly(&mut w).unwrap();
        assert_eq!(bf.name.as_deref(), Some("butterfly"));
        assert_eq!(bf.behaviors.len(), 2); // zigzag + env cull
        let bee = spawn_bee(&mut w).unwrap();
        assert_eq!(bee.name.as_deref(), Some("bee"));
        assert_eq!(bee.behaviors.len(), 2); // orbit + env cull
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib scene`
Expected: FAIL to compile (factories missing).

- [ ] **Step 3: Implement** (append to `src/scene.rs`)

```rust
/// Crée une entité qui traverse l'écran depuis un côté aléatoire.
fn cross_factory(
    world: &mut World,
    frames: &[String],
    depth: i32,
    speed: f64,
    y: f64,
    color: Color,
    name: &str,
    mask: Option<&[String]>,
    frame_rate: f64,
) -> Entity {
    let from_left = world.rng.gen::<f64>() < 0.5;
    let (x, dx, used): (f64, f64, Vec<String>) = if from_left {
        let w0 = max_line_width(&frames[0]) as f64;
        (-w0, speed, frames.to_vec())
    } else {
        (
            world.width as f64,
            -speed,
            frames.iter().map(|f| engine::flip_horizontal(f)).collect(),
        )
    };
    let used_mask: Option<Vec<String>> = mask.map(|m| {
        if from_left {
            m.to_vec()
        } else {
            m.iter().map(|s| engine::flip_horizontal(s)).collect()
        }
    });
    let mut e = Entity::new(used)
        .pos(x, y)
        .vel(dx, 0.0)
        .with_depth(depth)
        .with_frame_rate(frame_rate)
        .with_color(color)
        .with_name(name);
    if let Some(m) = used_mask {
        e = e.with_mask(m);
    }
    e
}

fn celestial_x(world: &World, frame: &str) -> f64 {
    (world.width as i32 - max_line_width(frame) - 1) as f64
}

pub fn spawn_bird(world: &mut World) -> Option<Entity> {
    let y = world.rng.gen_range(1..=std::cmp::max(2, world.height as i32 / 3)) as f64;
    let speed = world.rng.gen_range(8.0..14.0);
    let frames: Vec<String> = art::BIRD.iter().map(|s| s.to_string()).collect();
    let masks: Vec<String> = art::BIRD_MASK.iter().map(|s| s.to_string()).collect();
    Some(cross_factory(
        world,
        &frames,
        engine::DEPTH_SKY_CREATURE,
        speed,
        y,
        Color::White,
        "bird",
        Some(&masks),
        4.0,
    ))
}

pub fn spawn_cloud(world: &mut World) -> Option<Entity> {
    let (frame, mask) = cloud_art(world);
    let y = world.rng.gen_range(0..=std::cmp::max(1, world.height as i32 / 4)) as f64;
    let speed = world.rng.gen_range(1.5..3.0);
    let frames = vec![frame.to_string()];
    let masks = vec![mask.to_string()];
    Some(cross_factory(
        world,
        &frames,
        engine::DEPTH_CLOUD,
        speed,
        y,
        Color::White,
        "cloud",
        Some(&masks),
        0.0,
    ))
}

pub fn spawn_butterfly(world: &mut World) -> Option<Entity> {
    let y = ground_top(world) as f64 - world.rng.gen_range(2..=6) as f64;
    let speed = world.rng.gen_range(3.0..6.0);
    let frames: Vec<String> = art::BUTTERFLY.iter().map(|s| s.to_string()).collect();
    let bottom = ground_top(world) as f64 - 1.0;
    let vy = world.rng.gen_range(3.0..6.0);
    let e = cross_factory(
        world,
        &frames,
        engine::DEPTH_SKY_CREATURE,
        speed,
        y,
        Color::Magenta,
        "butterfly",
        None,
        6.0,
    )
    .with_behavior(Behavior::Zigzag { top: 1.0, bottom, vy })
    .with_behavior(Behavior::EnvCull { day: true });
    Some(e)
}

pub fn spawn_owl(world: &mut World) -> Option<Entity> {
    let (tox, toy) = tree_origin(world);
    let (tf, _) = tree_art(world);
    let tree_w = max_line_width(tf);
    let tree_h = tf.split('\n').count() as i32;
    let owl_w = max_line_width(art::OWL[0]);
    let x = tox + tree_w / 2 - owl_w / 2;
    let y = toy + std::cmp::max(1, tree_h / 2 - 3); // centré dans la canopée
    let frames: Vec<String> = art::OWL.iter().map(|s| s.to_string()).collect();
    let masks: Vec<String> = art::OWL_MASK.iter().map(|s| s.to_string()).collect();
    Some(
        Entity::new(frames)
            .with_mask(masks)
            .pos(x as f64, y as f64)
            .with_depth(engine::DEPTH_TREE_CREATURE)
            .with_frame_rate(0.4)
            .with_color(Color::Brown)
            .with_name("owl")
            .with_behavior(Behavior::EnvCull { day: false }),
    )
}

pub fn spawn_bee(world: &mut World) -> Option<Entity> {
    let cx = (world.width / 2) as f64;
    let (_, toy) = tree_origin(world);
    let cy = toy as f64 + 1.0;
    let radius = world.rng.gen_range(3.0..6.0);
    let ang_speed = world.rng.gen_range(2.0..4.0);
    let frames: Vec<String> = art::BEE.iter().map(|s| s.to_string()).collect();
    Some(
        Entity::new(frames)
            .pos(cx, cy)
            .with_depth(engine::DEPTH_TREE_CREATURE)
            .with_frame_rate(10.0)
            .with_color(Color::Yellow)
            .with_name("bee")
            .with_behavior(Behavior::Orbit { cx, cy, radius, ang_speed, a: 0.0 })
            .with_behavior(Behavior::EnvCull { day: true }),
    )
}

pub fn spawn_sun(world: &mut World) -> Option<Entity> {
    Some(
        Entity::new(vec![art::SUN.to_string()])
            .with_mask(vec![art::SUN_MASK.to_string()])
            .pos(celestial_x(world, art::SUN), 0.0)
            .with_depth(engine::DEPTH_SUN)
            .with_color(Color::Yellow)
            .with_name("sun")
            .with_behavior(Behavior::EnvCull { day: true }),
    )
}

pub fn spawn_moon(world: &mut World) -> Option<Entity> {
    Some(
        Entity::new(vec![art::MOON.to_string()])
            .with_mask(vec![art::MOON_MASK.to_string()])
            .pos(celestial_x(world, art::MOON), 0.0)
            .with_depth(engine::DEPTH_SUN)
            .with_color(Color::White)
            .with_name("moon")
            .with_behavior(Behavior::EnvCull { day: false }),
    )
}

pub fn spawn_star(world: &mut World) -> Option<Entity> {
    let x = world.rng.gen_range(0..=world.width as i32 - 1) as f64;
    let y = world.rng.gen_range(0..=world.height as i32 / 3) as f64;
    let ch = art::STAR_CHARS[world.rng.gen_range(0..art::STAR_CHARS.len())];
    let color = if world.rng.gen::<bool>() { Color::White } else { Color::Yellow };
    Some(
        Entity::new(vec![ch.to_string()])
            .pos(x, y)
            .with_depth(engine::DEPTH_SUN)
            .with_color(color)
            .with_name("star")
            .with_behavior(Behavior::EnvCull { day: false }),
    )
}

pub fn spawn_firefly(world: &mut World) -> Option<Entity> {
    let (tox, toy) = tree_origin(world);
    let (tf, _) = tree_art(world);
    let tw = max_line_width(tf);
    let gy = ground_top(world);
    let lo = std::cmp::max(0, tox - 3);
    let hi = std::cmp::min(world.width as i32 - 1, tox + tw + 3);
    let x = world.rng.gen_range(lo..=hi) as f64;
    let y = world.rng.gen_range(toy + 2..=gy) as f64;
    let frame_rate = world.rng.gen_range(2.0..4.0);
    let vy = world.rng.gen_range(1.0..2.5);
    let frames: Vec<String> = art::FIREFLY.iter().map(|s| s.to_string()).collect();
    Some(
        Entity::new(frames)
            .pos(x, y)
            .with_depth(engine::DEPTH_TREE_CREATURE)
            .with_frame_rate(frame_rate)
            .with_color(Color::Yellow)
            .with_name("firefly")
            .with_behavior(Behavior::Zigzag { top: toy as f64, bottom: gy as f64, vy })
            .with_behavior(Behavior::EnvCull { day: false }),
    )
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib scene`
Expected: geometry + creature tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/scene.rs
git commit -m "feat(scene): cross_factory + sky/tree creature factories"
```

---

### Task 12: scene — falling, weather, and ground-animal factories

**Files:**
- Modify: `src/scene.rs`
- Test: `src/scene.rs`

**Interfaces:**
- Consumes: `cross_factory`, geometry helpers, `Behavior`, `env_snapshot`.
- Produces (all `pub fn … (world: &mut World) -> Option<Entity>`):
  - `spawn_apple, spawn_raindrop, spawn_wind_leaf, spawn_lightning`
  - `spawn_rabbit, spawn_fox, spawn_hedgehog, spawn_mouse, spawn_snail`
  - private helpers `ground_hopper(...) -> Entity`, `ground_walker(...) -> Entity`

- [ ] **Step 1: Write the failing tests** (append inside scene `mod tests`)

```rust
    #[test]
    fn apple_spawns_in_canopy_and_falls() {
        let mut w = World::seeded(60, 24, 0);
        let a = spawn_apple(&mut w).unwrap();
        assert_eq!(a.name.as_deref(), Some("apple"));
        assert_eq!(a.behaviors.len(), 1); // fall
        let (_, toy) = tree_origin(&w);
        let (frame, _) = tree_art(&w);
        let h = frame.split('\n').count() as f64;
        assert!(toy as f64 <= a.y && a.y <= toy as f64 + h);
    }

    #[test]
    fn rain_spawns_only_when_raining() {
        use crate::environment::Weather;
        let mut w = World::new(80, 24);
        w.env = Some(Environment::seeded(100.0, 0));
        w.env.as_mut().unwrap().weather = Weather::Clear;
        assert!(spawn_raindrop(&mut w).is_none());
        w.env.as_mut().unwrap().weather = Weather::Rain;
        let e = spawn_raindrop(&mut w).unwrap();
        assert_eq!(e.name.as_deref(), Some("rain"));
        assert!(e.dy > 0.0);
    }

    #[test]
    fn lightning_spawns_only_when_storming() {
        use crate::environment::Weather;
        let mut w = World::new(80, 24);
        w.env = Some(Environment::seeded(100.0, 0));
        w.env.as_mut().unwrap().weather = Weather::Rain;
        assert!(spawn_lightning(&mut w).is_none());
        w.env.as_mut().unwrap().weather = Weather::Storm;
        assert!(spawn_lightning(&mut w).is_some());
    }

    #[test]
    fn walkers_bottom_aligned_to_screen() {
        let mut w = World::seeded(60, 24, 3);
        for factory in [spawn_fox, spawn_hedgehog, spawn_mouse] {
            let e = factory(&mut w).unwrap();
            assert_eq!(e.y as i32 + e.height() as i32, w.height as i32);
        }
    }

    #[test]
    fn ground_animals_are_opaque_and_masked() {
        let mut w = World::seeded(60, 24, 3);
        for factory in [spawn_rabbit, spawn_fox, spawn_hedgehog, spawn_mouse] {
            let e = factory(&mut w).unwrap();
            assert!(e.opaque);
            assert!(e.color_mask.is_some());
        }
        assert!(spawn_snail(&mut w).unwrap().opaque); // snail opaque mais sans masque
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib scene`
Expected: FAIL to compile (factories missing).

- [ ] **Step 3: Implement** (append to `src/scene.rs`)

```rust
pub fn spawn_apple(world: &mut World) -> Option<Entity> {
    let (tox, toy) = tree_origin(world);
    let (tf, _) = tree_art(world);
    let tw = max_line_width(tf);
    let x = world.rng.gen_range(tox + 1..=tox + tw - 2) as f64;
    let is_apple = world.rng.gen::<f64>() < 0.6;
    let (frame, color) = if is_apple {
        (art::APPLE, Color::Red)
    } else {
        (art::LEAF, Color::Green)
    };
    Some(
        Entity::new(vec![frame.to_string()])
            .pos(x, toy as f64 + 1.0)
            .vel(0.0, 0.1)
            .with_depth(engine::DEPTH_TREE_CREATURE)
            .with_color(color)
            .with_name("apple")
            .with_behavior(Behavior::Fall {
                gravity: 12.0,
                ground_y: ground_top(world) as f64,
                first: true,
            }),
    )
}

pub fn spawn_raindrop(world: &mut World) -> Option<Entity> {
    let snap = world.env_snapshot();
    if !snap.raining {
        return None;
    }
    let dx = snap.wind_dx;
    let ch = if dx > 0.0 {
        art::RAIN_CHARS[2] // '\' penché à droite
    } else if dx < 0.0 {
        art::RAIN_CHARS[1] // '/' penché à gauche
    } else {
        art::RAIN_CHARS[0] // '|' vertical
    };
    let x = world.rng.gen_range(0..=world.width as i32 - 1) as f64;
    let dy = world.rng.gen_range(18.0..26.0);
    let color = if world.rng.gen::<bool>() { Color::Cyan } else { Color::Blue };
    Some(
        Entity::new(vec![ch.to_string()])
            .pos(x, 0.0)
            .vel(dx, dy)
            .with_depth(engine::DEPTH_FOREGROUND)
            .with_color(color)
            .with_name("rain"),
    )
}

pub fn spawn_wind_leaf(world: &mut World) -> Option<Entity> {
    let snap = world.env_snapshot();
    if !snap.windy || snap.raining {
        return None;
    }
    let dx = snap.wind_dx * world.rng.gen_range(1.5..2.5);
    let x = if dx >= 0.0 { -1.0 } else { world.width as f64 };
    let y = world.rng.gen_range(0..=std::cmp::max(0, ground_top(world) - 1)) as f64;
    let ch = art::WIND_CHARS[world.rng.gen_range(0..art::WIND_CHARS.len())];
    let dy = world.rng.gen_range(-1.0..1.0);
    let life = world.rng.gen_range(3.0..6.0);
    Some(
        Entity::new(vec![ch.to_string()])
            .pos(x, y)
            .vel(dx, dy)
            .with_depth(engine::DEPTH_SKY_CREATURE)
            .with_color(Color::Green)
            .with_name("wind_leaf")
            .with_behavior(Behavior::Lifespan { seconds: life, t: 0.0 }),
    )
}

pub fn spawn_lightning(world: &mut World) -> Option<Entity> {
    let snap = world.env_snapshot();
    if !snap.storming {
        return None;
    }
    let bolt_w = max_line_width(art::LIGHTNING);
    let x = world.rng.gen_range(0..=std::cmp::max(0, world.width as i32 - bolt_w)) as f64;
    let y = world.rng.gen_range(0..=world.height as i32 / 4) as f64;
    Some(
        Entity::new(vec![art::LIGHTNING.to_string()])
            .with_mask(vec![art::LIGHTNING_MASK.to_string()])
            .pos(x, y)
            .with_depth(engine::DEPTH_CLOUD)
            .with_color(Color::Yellow)
            .with_name("lightning")
            .with_behavior(Behavior::Lifespan { seconds: 0.3, t: 0.0 }),
    )
}

fn ground_hopper(
    world: &mut World,
    frames: &[String],
    color: Color,
    name: &str,
    amplitude: f64,
    speed: f64,
    masks: Option<&[String]>,
) -> Entity {
    let y = ground_top(world) as f64;
    let gy = ground_top(world);
    let mut e = cross_factory(
        world,
        frames,
        engine::DEPTH_GROUND_ANIMAL,
        speed,
        y,
        color,
        name,
        masks,
        6.0,
    );
    e.opaque = true; // le corps masque l'herbe derrière lui
    e.with_behavior(Behavior::Hop {
        ground_y: (gy + GROUND_ROWS - 1) as f64,
        amplitude,
        period: 0.4,
        t: 0.0,
    })
}

fn ground_walker(
    world: &mut World,
    frames: &[String],
    color: Color,
    name: &str,
    speed: f64,
    masks: Option<&[String]>,
    frame_rate: f64,
) -> Entity {
    let height = frames[0].split('\n').count() as i32;
    let y = (world.height as i32 - height) as f64;
    let mut e = cross_factory(
        world,
        frames,
        engine::DEPTH_GROUND_ANIMAL,
        speed,
        y,
        color,
        name,
        masks,
        frame_rate,
    );
    e.opaque = true;
    e
}

pub fn spawn_rabbit(world: &mut World) -> Option<Entity> {
    let frames: Vec<String> = art::RABBIT.iter().map(|s| s.to_string()).collect();
    let masks: Vec<String> = art::RABBIT_MASK.iter().map(|s| s.to_string()).collect();
    let speed = world.rng.gen_range(6.0..9.0);
    Some(ground_hopper(world, &frames, Color::White, "rabbit", 2.0, speed, Some(&masks)))
}

pub fn spawn_fox(world: &mut World) -> Option<Entity> {
    let frames: Vec<String> = art::FOX.iter().map(|s| s.to_string()).collect();
    let masks: Vec<String> = art::FOX_MASK.iter().map(|s| s.to_string()).collect();
    let speed = world.rng.gen_range(5.0..8.0);
    Some(ground_walker(world, &frames, Color::Red, "fox", speed, Some(&masks), 4.0))
}

pub fn spawn_hedgehog(world: &mut World) -> Option<Entity> {
    let frames = vec![art::HEDGEHOG.to_string()];
    let masks = vec![art::HEDGEHOG_MASK.to_string()];
    let speed = world.rng.gen_range(2.0..3.0);
    Some(ground_walker(world, &frames, Color::Brown, "hedgehog", speed, Some(&masks), 0.0))
}

pub fn spawn_mouse(world: &mut World) -> Option<Entity> {
    let frames = vec![art::MOUSE.to_string()];
    let masks = vec![art::MOUSE_MASK.to_string()];
    let speed = world.rng.gen_range(4.0..6.0);
    Some(ground_walker(world, &frames, Color::White, "mouse", speed, Some(&masks), 0.0))
}

pub fn spawn_snail(world: &mut World) -> Option<Entity> {
    let frames = vec![art::SNAIL.to_string()];
    let speed = world.rng.gen_range(0.8..1.5);
    Some(ground_walker(world, &frames, Color::Yellow, "snail", speed, None, 0.0))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib scene`
Expected: all scene factory tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/scene.rs
git commit -m "feat(scene): falling, weather, and ground-animal factories"
```

---

### Task 13: scene — targets + register_spawners

**Files:**
- Modify: `src/scene.rs`
- Test: `src/scene.rs`

**Interfaces:**
- Consumes: all `spawn_*` (Tasks 11–12), `Spawner`, `Target`, `is_day`, `is_night`, `env_snapshot`.
- Produces:
  - private target fns: `day_target_1, day_target_3, night_target_1, night_target_6, star_target, rain_target, wind_leaf_target, lightning_target` (each `fn(&World) -> i32`)
  - `pub fn register_spawners(spawner: &mut Spawner)`

- [ ] **Step 1: Write the failing tests** (append inside scene `mod tests`)

```rust
    use crate::spawn::{Spawner, step};

    fn spec_target(sp: &Spawner, name: &str, w: &World) -> i32 {
        sp.specs.iter().find(|s| s.name == name).unwrap().target.resolve(w)
    }

    fn day_world() -> World {
        let mut w = World::seeded(80, 24, 0);
        w.env = Some(Environment::seeded(100.0, 0)); // phase 0 => jour
        w
    }
    fn night_world() -> World {
        let mut w = World::seeded(80, 24, 0);
        let mut env = Environment::seeded(100.0, 0);
        env.update(50.0); // phase 0.5 => nuit
        w.env = Some(env);
        w
    }

    #[test]
    fn owl_target_night_only() {
        let mut sp = Spawner::new();
        register_spawners(&mut sp);
        assert_eq!(spec_target(&sp, "owl", &day_world()), 0);
        assert_eq!(spec_target(&sp, "owl", &night_world()), 1);
    }

    #[test]
    fn day_creatures_target_zero_at_night() {
        let mut sp = Spawner::new();
        register_spawners(&mut sp);
        let night = night_world();
        assert_eq!(spec_target(&sp, "bee", &night), 0);
        assert_eq!(spec_target(&sp, "butterfly", &night), 0);
    }

    #[test]
    fn moon_target_night_sun_target_day() {
        let mut sp = Spawner::new();
        register_spawners(&mut sp);
        let (day, night) = (day_world(), night_world());
        assert_eq!(spec_target(&sp, "sun", &day), 1);
        assert_eq!(spec_target(&sp, "sun", &night), 0);
        assert_eq!(spec_target(&sp, "moon", &night), 1);
        assert_eq!(spec_target(&sp, "moon", &day), 0);
    }

    #[test]
    fn register_spawners_populates_day_world() {
        let mut w = World::seeded(80, 24, 0);
        build_meadow(&mut w, 90.0); // env de jour (phase 0)
        let mut sp = Spawner::new();
        register_spawners(&mut sp);
        for _ in 0..200 {
            sp.tick(&mut w, 0.1);
        }
        let names: std::collections::HashSet<&str> =
            w.entities.iter().filter_map(|e| e.name.as_deref()).collect();
        assert!(names.contains("bee")); // créature de jour
        assert!(names.contains("sun")); // soleil day-gated
        assert!(!names.contains("owl")); // nocturne absent de jour
    }

    #[test]
    fn moon_replaces_sun_at_night_in_full_step() {
        let mut w = World::seeded(80, 24, 0);
        build_meadow(&mut w, 90.0);
        let dl = w.env.as_ref().unwrap().day_length;
        w.env.as_mut().unwrap().update(dl * 0.5); // bascule nuit
        let mut sp = Spawner::new();
        register_spawners(&mut sp);
        for _ in 0..60 {
            sp.tick(&mut w, 0.1); // tick seul : ne fait pas avancer l'horloge
        }
        let names: std::collections::HashSet<&str> =
            w.entities.iter().filter_map(|e| e.name.as_deref()).collect();
        assert!(names.contains("moon"));
        assert!(!names.contains("sun"));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib scene`
Expected: FAIL to compile (`register_spawners` missing) plus unused `step` import warning is fine.

- [ ] **Step 3: Implement** (append to `src/scene.rs`; add `use crate::spawn::{Spawner, Target};` to the imports near the top)

```rust
fn day_target_1(w: &World) -> i32 {
    if is_day(w) { 1 } else { 0 }
}
fn day_target_3(w: &World) -> i32 {
    if is_day(w) { 3 } else { 0 }
}
fn night_target_1(w: &World) -> i32 {
    if is_night(w) { 1 } else { 0 }
}
fn night_target_6(w: &World) -> i32 {
    if is_night(w) { 6 } else { 0 }
}
fn star_target(w: &World) -> i32 {
    if is_night(w) {
        std::cmp::max(3, w.width as i32 / 8)
    } else {
        0
    }
}
fn rain_target(w: &World) -> i32 {
    let snap = w.env_snapshot();
    if !snap.raining {
        return 0;
    }
    let base = std::cmp::max(6, w.width as i32 / 3);
    if snap.storming {
        base * 2 // l'orage intensifie la pluie
    } else {
        base
    }
}
fn wind_leaf_target(w: &World) -> i32 {
    let snap = w.env_snapshot();
    if snap.windy && !snap.raining {
        4
    } else {
        0
    }
}
fn lightning_target(w: &World) -> i32 {
    if w.env_snapshot().storming {
        1
    } else {
        0
    }
}

pub fn register_spawners(spawner: &mut Spawner) {
    // Corps célestes (gatés jour/nuit)
    spawner.register("sun", spawn_sun, Target::Dynamic(day_target_1), 1.0);
    spawner.register("moon", spawn_moon, Target::Dynamic(night_target_1), 1.0);
    spawner.register("star", spawn_star, Target::Dynamic(star_target), 2.0);
    // Résidents de l'arbre / créatures gatées
    spawner.register("owl", spawn_owl, Target::Dynamic(night_target_1), 1.0);
    spawner.register("bee", spawn_bee, Target::Dynamic(day_target_3), 0.8);
    spawner.register("firefly", spawn_firefly, Target::Dynamic(night_target_6), 1.5);
    // Ciel
    spawner.register("cloud", spawn_cloud, Target::Fixed(3), 0.3);
    spawner.register("bird", spawn_bird, Target::Fixed(5), 0.5);
    spawner.register("butterfly", spawn_butterfly, Target::Dynamic(day_target_3), 0.4);
    // Objets qui tombent
    spawner.register("apple", spawn_apple, Target::Fixed(2), 0.2);
    // Sol
    spawner.register("rabbit", spawn_rabbit, Target::Fixed(2), 0.3);
    spawner.register("fox", spawn_fox, Target::Fixed(1), 0.1);
    spawner.register("hedgehog", spawn_hedgehog, Target::Fixed(1), 0.15);
    spawner.register("mouse", spawn_mouse, Target::Fixed(2), 0.3);
    spawner.register("snail", spawn_snail, Target::Fixed(1), 0.1);
    // Météo
    spawner.register("rain", spawn_raindrop, Target::Dynamic(rain_target), 25.0);
    spawner.register("wind_leaf", spawn_wind_leaf, Target::Dynamic(wind_leaf_target), 3.0);
    spawner.register("lightning", spawn_lightning, Target::Dynamic(lightning_target), 1.5);
}
```

- [ ] **Step 4: Run the full library test suite**

Run: `cargo test --lib`
Expected: all library tests pass (engine, environment, spawn, art, scene).

- [ ] **Step 5: Commit**

```bash
git add src/scene.rs
git commit -m "feat(scene): population targets + register_spawners"
```

---

### Task 14: main — argument parsing

**Files:**
- Modify: `src/main.rs`
- Test: `src/main.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Produces:
  - `pub struct Args { pub seed: Option<u64>, pub fps: u32, pub day_length: f64 }`
  - `fn parse_args(args: &[String]) -> Result<Args, String>`
  - `const FPS: u32 = 20;`

- [ ] **Step 1: Write the failing tests** (append to `src/main.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn day_length_defaults_to_90() {
        let a = parse_args(&args(&[])).unwrap();
        assert_eq!(a.day_length, 90.0);
        assert_eq!(a.fps, 20);
        assert_eq!(a.seed, None);
    }

    #[test]
    fn day_length_parsed() {
        let a = parse_args(&args(&["--day-length", "30"])).unwrap();
        assert_eq!(a.day_length, 30.0);
    }

    #[test]
    fn seed_and_fps_parsed() {
        let a = parse_args(&args(&["--seed", "7", "--fps", "15"])).unwrap();
        assert_eq!(a.seed, Some(7));
        assert_eq!(a.fps, 15);
    }

    #[test]
    fn rejects_bad_fps_and_day_length() {
        assert!(parse_args(&args(&["--fps", "0"])).is_err());
        assert!(parse_args(&args(&["--day-length", "0"])).is_err());
        assert!(parse_args(&args(&["--seed", "x"])).is_err());
        assert!(parse_args(&args(&["--unknown"])).is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --bin asciimeadow`
Expected: FAIL to compile (`parse_args`, `Args` missing).

- [ ] **Step 3: Implement** (replace `src/main.rs` body; keep the `mod tests` block from Step 1)

```rust
//! Point d'entrée : coquille crossterm + boucle principale (seul module terminal-aware).

const FPS: u32 = 20;

pub struct Args {
    pub seed: Option<u64>,
    pub fps: u32,
    pub day_length: f64,
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut seed: Option<u64> = None;
    let mut fps: u32 = FPS;
    let mut day_length: f64 = 90.0;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" => {
                i += 1;
                let v = args.get(i).ok_or("--seed requires a value")?;
                seed = Some(v.parse::<u64>().map_err(|_| "--seed must be an integer")?);
            }
            "--fps" => {
                i += 1;
                let v = args.get(i).ok_or("--fps requires a value")?;
                fps = v.parse::<u32>().map_err(|_| "--fps must be an integer")?;
            }
            "--day-length" => {
                i += 1;
                let v = args.get(i).ok_or("--day-length requires a value")?;
                day_length = v.parse::<f64>().map_err(|_| "--day-length must be a number")?;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }
    if fps < 1 {
        return Err("--fps must be >= 1".into());
    }
    if day_length <= 0.0 {
        return Err("--day-length must be > 0".into());
    }
    Ok(Args { seed, fps, day_length })
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("asciimeadow: {e}");
            std::process::exit(2);
        }
    };
    if let Err(e) = run(args) {
        eprintln!("asciimeadow: {e}");
        std::process::exit(1);
    }
}
```

Note: `run` is defined in Task 15. Until then the binary will not compile. **Implement Tasks 14 and 15 together; run tests after Task 15.**

- [ ] **Step 4: Defer verification** — verify at Task 15.

- [ ] **Step 5: Commit** (after Task 15 green).

---

### Task 15: main — crossterm display + main loop

**Files:**
- Modify: `src/main.rs`
- Test: `src/main.rs`

**Interfaces:**
- Consumes: `asciimeadow::engine::{World, Buffer, Color, COLOR_NAMES}`, `asciimeadow::scene`, `asciimeadow::spawn::{Spawner, step}`, `Args` (Task 14), `crossterm`, `rand`.
- Produces:
  - `fn term_color(c: Color) -> crossterm::style::Color`
  - `struct TerminalGuard;` (RAII: restore terminal on drop)
  - `struct Display { out: std::io::Stdout, prev: Option<Buffer> }` with `draw(&mut self, buf: &Buffer)` and `force_repaint(&mut self)`
  - `fn run(args: Args) -> std::io::Result<()>`

- [ ] **Step 1: Write the failing test** (append inside main's `mod tests`)

```rust
    #[test]
    fn term_color_covers_all_palette() {
        use asciimeadow::engine::COLOR_NAMES;
        // Exhaustif par construction : chaque variante mappe sans paniquer.
        for c in COLOR_NAMES {
            let _ = super::term_color(c);
        }
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --bin asciimeadow`
Expected: FAIL to compile (`term_color`, `run` missing).

- [ ] **Step 3: Implement** (insert above `fn main` in `src/main.rs`; add imports at top)

Add at the very top of `src/main.rs` (below the doc comment):

```rust
use std::io::{Stdout, Write};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::style::{Color as TColor, Print, SetForegroundColor};
use crossterm::{cursor, execute, queue, terminal};

use asciimeadow::engine::{Buffer, Color, World};
use asciimeadow::scene;
use asciimeadow::spawn::{step, Spawner};
use rand::rngs::StdRng;
use rand::SeedableRng;
```

Insert before `fn main`:

```rust
/// Couleur logique -> couleur crossterm. Pas de brun natif : DarkYellow.
fn term_color(c: Color) -> TColor {
    match c {
        Color::White => TColor::White,
        Color::Green => TColor::Green,
        Color::Brown => TColor::DarkYellow,
        Color::Yellow => TColor::Yellow,
        Color::Red => TColor::Red,
        Color::Cyan => TColor::Cyan,
        Color::Blue => TColor::Blue,
        Color::Magenta => TColor::Magenta,
        Color::Black => TColor::Black,
    }
}

/// Rétablit le terminal quoi qu'il arrive (sortie normale, `q`, Ctrl+C, panic).
struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut out = std::io::stdout();
        let _ = execute!(out, cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

/// Affichage double-buffer : ne réécrit que les cellules changées.
struct Display {
    out: Stdout,
    prev: Option<Buffer>,
}

impl Display {
    fn new() -> Self {
        Display { out: std::io::stdout(), prev: None }
    }

    fn force_repaint(&mut self) {
        self.prev = None;
    }

    fn draw(&mut self, buf: &Buffer) -> std::io::Result<()> {
        let same_size = matches!(&self.prev, Some(p) if p.width == buf.width && p.height == buf.height);
        for y in 0..buf.height {
            for x in 0..buf.width {
                let ch = buf.chars[y][x];
                let col = buf.colors[y][x];
                let changed = !same_size
                    || self.prev.as_ref().map_or(true, |p| {
                        p.chars[y][x] != ch || p.colors[y][x] != col
                    });
                if changed {
                    queue!(
                        self.out,
                        cursor::MoveTo(x as u16, y as u16),
                        SetForegroundColor(term_color(col)),
                        Print(ch)
                    )?;
                }
            }
        }
        self.out.flush()?;
        self.prev = Some(buf.clone());
        Ok(())
    }
}

fn build_world(width: usize, height: usize, seed: Option<u64>, day_length: f64) -> (World, Spawner) {
    let rng = match seed {
        Some(s) => StdRng::seed_from_u64(s),
        None => StdRng::from_entropy(),
    };
    let mut world = World::with_rng(width, height, rng);
    scene::build_meadow(&mut world, day_length);
    let mut spawner = Spawner::new();
    scene::register_spawners(&mut spawner);
    (world, spawner)
}

fn run(args: Args) -> std::io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut out = std::io::stdout();
    execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;
    let _guard = TerminalGuard; // restaure à la sortie (y compris panic)

    let (mut cols, mut rows) = terminal::size()?;
    let (mut world, mut spawner) =
        build_world(cols as usize, rows as usize, args.seed, args.day_length);

    let mut disp = Display::new();
    let dt = 1.0 / args.fps as f64;
    let frame = std::time::Duration::from_secs_f64(dt);
    let mut paused = false;

    loop {
        // La fenêtre de poll fait office de cadence (comme le timeout curses).
        if event::poll(frame)? {
            match event::read()? {
                Event::Key(k) if k.kind != KeyEventKind::Release => match k.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char('p') | KeyCode::Char('P') => paused = !paused,
                    KeyCode::Char('r') | KeyCode::Char('R') => disp.force_repaint(),
                    _ => {}
                },
                Event::Resize(w, h) => {
                    cols = w;
                    rows = h;
                    let rebuilt = build_world(cols as usize, rows as usize, args.seed, args.day_length);
                    world = rebuilt.0;
                    spawner = rebuilt.1;
                    disp.force_repaint();
                }
                _ => {}
            }
        }
        if !paused {
            step(&mut world, &spawner, dt);
        }
        disp.draw(&world.render())?;
    }
    Ok(())
}
```

Note: `COLOR_NAMES` import is only used by the test; keep the test's `use asciimeadow::engine::COLOR_NAMES;` local to the test fn (as written in Step 1) to avoid an unused import in non-test builds.

- [ ] **Step 4: Verify build, tests, and a real run**

Run: `cargo build`
Expected: compiles clean (no warnings ideally; `cargo build 2>&1 | grep warning` should be empty).
Run: `cargo test`
Expected: entire suite (lib + bin) passes.
Run: `cargo run -- --seed 42` in a real terminal.
Expected: animated meadow renders; `p` pauses, `r` repaints, `q` and `Ctrl+C` quit cleanly leaving the terminal usable.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat(main): crossterm display, main loop, arg parsing"
```

---

### Task 16: cleanup — delete Python, update .gitignore + CLAUDE.md

**Files:**
- Delete: `asciimeadow/` (the Python package dir and all `.py` + `__pycache__`), `tests/` (Python tests + `__pycache__`), `pyproject.toml`, `.pytest_cache/`
- Modify: `.gitignore`
- Modify: `CLAUDE.md`

**Interfaces:** none (repo hygiene + docs).

- [ ] **Step 1: Confirm the Rust suite is green before deleting Python**

Run: `cargo test`
Expected: full pass. (Do not delete Python until this is green — `art.rs` transcription is validated against `art.py`.)

- [ ] **Step 2: Delete the Python code and caches**

```bash
git rm -r asciimeadow tests pyproject.toml
rm -rf .pytest_cache
```

- [ ] **Step 3: Update `.gitignore`** — replace Python-specific ignores with Rust

```gitignore
/target
Cargo.lock
**/*.rs.bk
```

Note: for a binary crate, committing `Cargo.lock` is conventional; if you prefer to commit it, drop that line. Either choice is fine — pick one and keep it.

- [ ] **Step 4: Rewrite `CLAUDE.md` for the Rust codebase**

Replace the whole file with:

```markdown
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`asciimeadow` is a terminal screensaver: an animated ASCII meadow (tree, animals, weather, day/night cycle) rendered with `crossterm`. Dependencies are limited to `crossterm` and `rand`.

The codebase and its comments are written in **French** — match that when adding comments or docstrings.

## Commands

\`\`\`bash
cargo run                                  # run the animation (needs a real TTY)
cargo run -- --seed 42                     # deterministic run
cargo run -- --fps 30 --day-length 60
cargo test                                 # run all tests (lib + bin)
cargo test --lib scene                     # run one module's tests
\`\`\`

Runtime keys: `q` quit, `p` pause, `r` redraw; `Ctrl+C` quits cleanly; terminal resize rebuilds the world.

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
```

(The `\`\`\`` fences above are escaped for this plan; write them as plain triple backticks in the actual `CLAUDE.md`.)

- [ ] **Step 5: Final verification**

Run: `cargo test`
Expected: full pass.
Run: `cargo build --release`
Expected: clean release build.
Run: `git status`
Expected: no stray `.py`/`__pycache__`/`.pytest_cache` tracked.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: remove Python implementation, update .gitignore and CLAUDE.md"
```

---

## Self-Review (plan author's checklist — completed)

**Spec coverage** (each spec section → task):
- Crate shape / deps / lean args → Task 1, 14.
- Module map (pure core / shell split) → Tasks 2–15; split enforced by lib-vs-bin (Task 1) and CLAUDE.md (Task 16).
- Frame pipeline (env → advance → tick) → `step` (Task 8), verified by `step_ticks_env_before_spawn`.
- Rendering / depth / masks / opaque / flip → Tasks 2, 4; tests cover mask override, opaque silhouette, depth order.
- Behavior enum model A + `EnvSnapshot` by value → Task 3.
- Determinism (`StdRng`, injectable env rng) → Tasks 6, 7; `seeded_rng_gives_deterministic_weather_sequence`.
- Environment model → Task 7.
- Terminal shell (raw mode, Drop guard, Ctrl+C, resize, diff draw) → Task 15.
- Tests ported (all 6 Python files) → engine (2,3,4,6), environment (7), spawn (8), art (9), scene (10–13), main (14,15).
- Python deletion + docs → Task 16.

**Placeholder scan:** no "TBD"/"implement later"; art transcription (Task 9) points at the in-repo `art.py` with exact type rules + a verifying alignment test, not a vague instruction.

**Type consistency:** setter names (`pos, vel, with_depth, with_color, with_frame_rate, with_mask, with_name, opaque, with_behavior, on_death`), `Target::{Fixed,Dynamic}` + `resolve`, factory signature `fn(&mut World)->Option<Entity>`, `Behavior` variant fields, and `EnvSnapshot` fields are used identically across Tasks 3–15.

**Known cross-task compile ordering** (called out inline): engine Tasks 3/4/6 + environment Task 7 first compile together (first green run at end of Task 7); main Tasks 14/15 compile together (first green run at end of Task 15). Subagent/inline executors must treat those pairs as a unit.
