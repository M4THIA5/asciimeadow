# Bande terrestre épaisse + animaux à hauteurs variées — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Épaissir la bande terrestre (proportionnelle au terminal) et répartir les animaux terrestres à des hauteurs variées dans la bande avec un effet de perspective.

**Architecture:** Tout se passe dans `src/scene.rs` (cœur pur). On remplace la constante `GROUND_ROWS` par une fonction `ground_rows(world)`, on généralise `ground_frames` pour N rangs, puis on ajoute un helper `ground_slot` qui tire une rangée-plancher dans la bande et calcule un `depth` de perspective, câblé dans `ground_walker` et `ground_hopper`.

**Tech Stack:** Rust, `rand` (`world.rng`). Tests via `cargo test`. Aucune dépendance nouvelle, aucun `crossterm`.

## Global Constraints

- Cœur pur : modifications limitées à `src/scene.rs`, aucun `crossterm`, aucune I/O.
- Aucune dépendance nouvelle.
- Déterminisme : toute hauteur/tirage vient de `world.rng`.
- Commentaires et docstrings en **français** (règle du repo).
- Bande : `ground_rows = min(max(6, height/4), max(1, height/2))`.
- Perspective : plus haut dans la bande ⇒ `depth` plus grand ⇒ derrière ; borné à `DEPTH_GRASS - 1` (44), base `DEPTH_GROUND_ANIMAL` (40).
- **Commits en pause (demande utilisateur du 2026-07-08).** Les étapes « Commit » ci-dessous décrivent le commit standard, mais **ne pas exécuter `git commit` sans confirmation explicite** de l'utilisateur ; se contenter de `git add` et signaler que le commit est prêt.
- Vérif finale de chaque tâche : `cargo test` et `cargo clippy --all-targets` verts.

---

### Task 1: Bande proportionnelle + herbe sur N rangs

**Files:**
- Modify: `src/scene.rs` (`GROUND_ROWS` → `ground_rows`, `ground_top`, `ground_frames`, `build_meadow`, `ground_hopper`)
- Test: `src/scene.rs` (module `tests`)

**Interfaces:**
- Consumes: `World` (`world.height: usize`, `world.width: usize`), `art::GRASS_ROWS: [&str; 4]`.
- Produces:
  - `pub fn ground_rows(world: &World) -> i32`
  - `pub fn ground_top(world: &World) -> i32` (signature inchangée, impl mise à jour)
  - `pub fn ground_frames(width: usize, rows: usize) -> Vec<String>` (nouvelle signature : ajout de `rows`)

- [ ] **Step 1: Écrire le test qui échoue pour `ground_rows`**

Dans le module `#[cfg(test)] mod tests` de `src/scene.rs`, ajouter :

```rust
    #[test]
    fn ground_rows_scales_and_clamps() {
        assert_eq!(ground_rows(&World::new(40, 20)), 6); // max(6, 20/4=5) => 6
        assert_eq!(ground_rows(&World::new(80, 40)), 10); // 40/4 = 10
        assert_eq!(ground_rows(&World::new(20, 8)), 4); // plafond height/2 = 4
    }
```

- [ ] **Step 2: Lancer le test, vérifier qu'il échoue à la compilation**

Run: `cargo test --lib scene::tests::ground_rows_scales_and_clamps`
Expected: FAIL — `cannot find function ground_rows in this scope`.

- [ ] **Step 3: Remplacer la constante par la fonction `ground_rows` et mettre à jour `ground_top`**

Dans `src/scene.rs`, remplacer :

```rust
pub const GROUND_ROWS: i32 = 4;
```

par :

```rust
/// Nombre de rangs de la bande terrestre : ~¼ de l'écran, min 6, plafonné à la
/// moitié de la hauteur (protège les petits terminaux, laisse la place au ciel).
pub fn ground_rows(world: &World) -> i32 {
    let h = world.height as i32;
    std::cmp::min(std::cmp::max(6, h / 4), std::cmp::max(1, h / 2))
}
```

Puis remplacer le corps de `ground_top` :

```rust
pub fn ground_top(world: &World) -> i32 {
    world.height as i32 - GROUND_ROWS
}
```

par :

```rust
pub fn ground_top(world: &World) -> i32 {
    world.height as i32 - ground_rows(world)
}
```

- [ ] **Step 4: Corriger la référence restante à `GROUND_ROWS` dans `ground_hopper`**

Dans `ground_hopper`, remplacer la ligne :

```rust
        ground_y: (gy + GROUND_ROWS - 1) as f64,
```

par :

```rust
        ground_y: (gy + ground_rows(world) - 1) as f64,
```

(La logique du lapin est réécrite en Task 2 ; ici on garde juste la compilation et le comportement bas-de-bande.)

- [ ] **Step 5: Lancer le test `ground_rows`, vérifier qu'il passe**

Run: `cargo test --lib scene::tests::ground_rows_scales_and_clamps`
Expected: PASS.

- [ ] **Step 6: Écrire/ajuster le test qui échoue pour `ground_frames(width, rows)`**

Dans le module `tests`, remplacer le test existant `ground_frames_fill_all_rows` :

```rust
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
```

par :

```rust
    #[test]
    fn ground_frames_fill_all_rows() {
        let frames = ground_frames(30, 7);
        assert_eq!(frames.len(), 2);
        for f in &frames {
            let lines: Vec<&str> = f.split('\n').collect();
            assert_eq!(lines.len(), 7);
            for line in lines {
                assert_eq!(line.chars().count(), 30);
                assert!(!line.trim().is_empty());
            }
        }
    }
```

- [ ] **Step 7: Lancer le test, vérifier qu'il échoue à la compilation**

Run: `cargo test --lib scene::tests::ground_frames_fill_all_rows`
Expected: FAIL — `this function takes 1 argument but 2 arguments were supplied`.

- [ ] **Step 8: Généraliser `ground_frames` à N rangs**

Remplacer entièrement la fonction `ground_frames` :

```rust
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
```

par :

```rust
pub fn ground_frames(width: usize, rows: usize) -> Vec<String> {
    // Chaque rang cycle un motif de GRASS_ROWS, décalé horizontalement de `i`
    // glyphes pour éviter une répétition verticale visible quand la bande dépasse
    // 4 lignes.
    let base: Vec<String> = (0..rows)
        .map(|i| {
            let p = art::GRASS_ROWS[i % art::GRASS_ROWS.len()];
            let plen = p.chars().count();
            let reps = width / plen + 2;
            let repeated: Vec<char> = p.repeat(reps).chars().collect();
            let start = i % plen; // décalage horizontal par rang
            repeated[start..start + width].iter().collect()
        })
        .collect();
    let shifted: Vec<String> = base
        .iter()
        .map(|r| {
            let chars: Vec<char> = r.chars().collect();
            let mut s: String = chars[1..].iter().collect();
            s.push(chars[0]);
            s
        })
        .collect();
    vec![base.join("\n"), shifted.join("\n")]
}
```

- [ ] **Step 9: Mettre à jour l'appel dans `build_meadow`**

Dans `build_meadow`, remplacer :

```rust
    let gframes = ground_frames(world.width);
```

par :

```rust
    let gframes = ground_frames(world.width, ground_rows(world) as usize);
```

- [ ] **Step 10: Corriger le test `ground_top_leaves_band`**

Remplacer :

```rust
    #[test]
    fn ground_top_leaves_band() {
        let w = World::new(40, 20);
        assert_eq!(ground_top(&w), 16);
    }
```

par :

```rust
    #[test]
    fn ground_top_leaves_band() {
        let w = World::new(40, 20);
        assert_eq!(ground_top(&w), 14); // bande = ground_rows = 6, 20 - 6
    }
```

- [ ] **Step 11: Lancer toute la suite du module, vérifier vert**

Run: `cargo test --lib scene`
Expected: PASS (tous les tests du module `scene`, y compris `ground_frames_fill_all_rows`, `ground_top_leaves_band`, `ground_rows_scales_and_clamps`).

- [ ] **Step 12: Clippy**

Run: `cargo clippy --all-targets`
Expected: aucune erreur, aucun warning nouveau.

- [ ] **Step 13: Commit (⚠ commits en pause — voir Global Constraints)**

```bash
git add src/scene.rs
git commit -m "feat(scene): bande terrestre proportionnelle + herbe N rangs"
```

Ne pas exécuter `git commit` sans confirmation utilisateur ; laisser les changements stagés et signaler.

---

### Task 2: Hauteur variée + perspective des animaux terrestres

**Files:**
- Modify: `src/scene.rs` (nouveau helper `ground_slot`, `ground_walker`, `ground_hopper`)
- Test: `src/scene.rs` (module `tests`)

**Interfaces:**
- Consumes: `ground_top`, `ground_rows` (Task 1), `engine::DEPTH_GROUND_ANIMAL`, `engine::DEPTH_GRASS`, `World.rng`, `cross_factory` (signature inchangée : prend déjà `depth: i32` et `y: f64`).
- Produces: `fn ground_slot(world: &mut World, sprite_h: i32) -> (i32, i32)` — retourne `(feet_row, depth)`, `feet_row` = rangée-écran des pieds au repos, `depth` = profondeur de perspective.

- [ ] **Step 1: Écrire le test qui échoue pour `ground_slot` (bornes + perspective)**

Dans le module `tests`, ajouter :

```rust
    #[test]
    fn ground_slot_keeps_body_in_band_and_orders_depth() {
        let mut w = World::seeded(60, 24, 7);
        let top = ground_top(&w);
        let bottom = w.height as i32 - 1;
        let sprite_h = 2;
        for _ in 0..50 {
            let (feet, depth) = ground_slot(&mut w, sprite_h);
            // pieds dans la bande, corps entier dans la bande
            assert!(feet <= bottom, "pieds sous l'écran");
            assert!(feet - (sprite_h - 1) >= top, "corps au-dessus de la bande");
            // depth dans la fenêtre de perspective, toujours devant l'herbe
            assert!(depth >= engine::DEPTH_GROUND_ANIMAL);
            assert!(depth < engine::DEPTH_GRASS);
            // plus haut (feet plus petit) => plus loin => depth plus grand
            let rank = bottom - feet;
            let expected = std::cmp::min(engine::DEPTH_GROUND_ANIMAL + rank, engine::DEPTH_GRASS - 1);
            assert_eq!(depth, expected);
        }
    }
```

- [ ] **Step 2: Lancer le test, vérifier qu'il échoue à la compilation**

Run: `cargo test --lib scene::tests::ground_slot_keeps_body_in_band_and_orders_depth`
Expected: FAIL — `cannot find function ground_slot in this scope`.

- [ ] **Step 3: Ajouter le helper `ground_slot`**

Dans `src/scene.rs`, juste au-dessus de `fn ground_hopper`, ajouter :

```rust
/// Tire la rangée-plancher (les « pieds ») d'un animal terrestre dans la bande,
/// pour un sprite de hauteur `sprite_h`, et le `depth` de perspective associé.
///
/// L'animal tient entièrement dans la bande (le haut du sprite reste ≥ `ground_top`).
/// Plus haut dans la bande ⇒ plus loin ⇒ `depth` plus grand ⇒ peint avant ⇒ derrière.
/// `depth` est borné à `DEPTH_GRASS - 1` : les animaux restent toujours devant l'herbe.
fn ground_slot(world: &mut World, sprite_h: i32) -> (i32, i32) {
    let top = ground_top(world);
    let bottom = world.height as i32 - 1;
    // Borne basse des pieds : garde le corps entier dans la bande. Si le sprite est
    // plus haut que la bande, on retombe sur `bottom` (plage non vide).
    let lo = std::cmp::min(top + sprite_h - 1, bottom);
    let feet = world.rng.gen_range(lo..=bottom);
    let rank = bottom - feet; // 0 = tout en bas (devant), croît vers le haut (derrière)
    let depth = std::cmp::min(engine::DEPTH_GROUND_ANIMAL + rank, engine::DEPTH_GRASS - 1);
    (feet, depth)
}
```

- [ ] **Step 4: Lancer le test `ground_slot`, vérifier qu'il passe**

Run: `cargo test --lib scene::tests::ground_slot_keeps_body_in_band_and_orders_depth`
Expected: PASS.

- [ ] **Step 5: Câbler `ground_walker` sur `ground_slot`**

Remplacer le corps de `ground_walker` :

```rust
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
```

par :

```rust
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
    let (feet, depth) = ground_slot(world, height);
    let y = (feet - (height - 1)) as f64; // bas du sprite posé sur `feet`
    let mut e = cross_factory(
        world, frames, depth, speed, y, color, name, masks, frame_rate,
    );
    e.opaque = true;
    e
}
```

- [ ] **Step 6: Câbler `ground_hopper` sur `ground_slot`**

Remplacer le corps de `ground_hopper` :

```rust
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
        ground_y: (gy + ground_rows(world) - 1) as f64,
        amplitude,
        period: 0.4,
        t: 0.0,
    })
}
```

par :

```rust
fn ground_hopper(
    world: &mut World,
    frames: &[String],
    color: Color,
    name: &str,
    amplitude: f64,
    speed: f64,
    masks: Option<&[String]>,
) -> Entity {
    let height = frames[0].split('\n').count() as i32;
    let (feet, depth) = ground_slot(world, height);
    let y = (feet - (height - 1)) as f64;
    let mut e = cross_factory(
        world, frames, depth, speed, y, color, name, masks, 6.0,
    );
    e.opaque = true; // le corps masque l'herbe derrière lui
    // Hop pose le bas du sprite à `ground_y - 1` ; `feet + 1` place les pieds au
    // repos exactement sur `feet`.
    e.with_behavior(Behavior::Hop {
        ground_y: (feet + 1) as f64,
        amplitude,
        period: 0.4,
        t: 0.0,
    })
}
```

- [ ] **Step 7: Écrire le test qui échoue pour la nouvelle position des walkers**

Remplacer le test existant `walkers_bottom_aligned_to_screen` :

```rust
    #[test]
    fn walkers_bottom_aligned_to_screen() {
        let mut w = World::seeded(60, 24, 3);
        for factory in [spawn_fox, spawn_hedgehog, spawn_mouse] {
            let e = factory(&mut w).unwrap();
            assert_eq!(e.y as i32 + e.height() as i32, w.height as i32);
        }
    }
```

par :

```rust
    #[test]
    fn walkers_feet_within_band() {
        let mut w = World::seeded(60, 24, 3);
        let top = ground_top(&w);
        for factory in [spawn_fox, spawn_hedgehog, spawn_mouse] {
            let e = factory(&mut w).unwrap();
            let feet = e.y as i32 + e.height() as i32 - 1; // rangée du bas du sprite
            assert!(top <= e.y as i32, "corps au-dessus de la bande");
            assert!(feet <= w.height as i32 - 1, "pieds sous l'écran");
            assert!(e.depth >= engine::DEPTH_GROUND_ANIMAL && e.depth < engine::DEPTH_GRASS);
        }
    }
```

- [ ] **Step 8: Lancer le test, vérifier qu'il passe (implémentation déjà faite aux steps 5-6)**

Run: `cargo test --lib scene::tests::walkers_feet_within_band`
Expected: PASS.

- [ ] **Step 9: Vérifier que les tests existants d'opacité/masque tiennent toujours**

Run: `cargo test --lib scene`
Expected: PASS — notamment `ground_animals_are_opaque_and_masked` (opacité/masque inchangés) et tous les tests de spawners.

- [ ] **Step 10: Suite complète + clippy**

Run: `cargo test`
Expected: PASS (lib + bin).

Run: `cargo clippy --all-targets`
Expected: aucune erreur, aucun warning nouveau.

- [ ] **Step 11: Vérification visuelle manuelle (TTY réel)**

Run: `cargo run -- --seed 42`
Attendu à l'œil : bande d'herbe nettement plus épaisse ; renard, hérisson, souris, escargot, lapin apparaissent à des hauteurs différentes ; les animaux plus haut passent derrière ceux plus bas ; tous restent devant l'herbe. Quitter avec `q`.

- [ ] **Step 12: Commit (⚠ commits en pause — voir Global Constraints)**

```bash
git add src/scene.rs
git commit -m "feat(scene): animaux terrestres à hauteurs variées avec perspective"
```

Ne pas exécuter `git commit` sans confirmation utilisateur ; laisser les changements stagés et signaler.

---

## Notes de revue (self-review du plan)

- **Couverture du spec :** bande proportionnelle (Task 1, `ground_rows`), herbe N rangs (Task 1, `ground_frames`), hauteur aléatoire dans la bande (Task 2, `ground_slot` + walker/hopper), perspective bornée devant l'herbe (Task 2, `depth`), corps entier dans la bande (Task 2, borne `lo`). Tests impactés listés dans le spec tous traités (Task 1 steps 6/10, Task 2 steps 7 + nouveau step 1).
- **Cohérence des types :** `ground_slot(world: &mut World, sprite_h: i32) -> (i32, i32)` utilisé identiquement dans walker et hopper. `ground_frames(width: usize, rows: usize)` — appel dans `build_meadow` passe `ground_rows(world) as usize`. `ground_rows(&World) -> i32`, `ground_top(&World) -> i32`.
- **Fenêtre de perspective :** base `DEPTH_GROUND_ANIMAL = 40`, plafond `DEPTH_GRASS - 1 = 44` → 5 couches, conforme à la décision.
