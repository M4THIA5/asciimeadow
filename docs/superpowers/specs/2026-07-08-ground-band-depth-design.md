# Design — bande terrestre épaisse + animaux à hauteurs variées

Date : 2026-07-08

## Contexte

La bande d'herbe fait aujourd'hui 4 rangs fixes (`GROUND_ROWS: i32 = 4`), remplie
par les 4 motifs `art::GRASS_ROWS`. Tous les animaux terrestres arrivent au même
niveau : les *walkers* (renard, hérisson, souris, escargot) sont collés au bas de
l'écran (`y = height - hauteur_sprite`), le lapin part du haut de la bande et
saute. Résultat : aucune profondeur, la prairie paraît plate.

## Objectif

1. Épaissir la bande terrestre, proportionnellement à la taille du terminal.
2. Répartir les animaux terrestres à des hauteurs variées dans cette bande, avec
   un effet de perspective (les animaux plus hauts passent derrière les plus bas).

Contrainte retenue : **l'animal reste entièrement dans la bande** — pas de corps
qui dépasse au-dessus de l'herbe.

## Décisions

- **Bande proportionnelle**, min 6 rangs (~¼ de la hauteur), plafonnée à la
  moitié de l'écran.
- **Hauteur aléatoire à chaque spawn** (pas de couloir fixe par espèce) : un même
  type peut apparaître haut ou bas.
- **Perspective** : plus haut dans la bande = plus loin = rendu derrière, mais
  tous les animaux restent devant l'herbe (pas d'occlusion par l'herbe).

## Conception

### 1. Bande proportionnelle (`scene.rs`)

Remplacer la constante par une fonction dépendant du monde :

```rust
pub fn ground_rows(world: &World) -> i32 {
    let h = world.height as i32;
    std::cmp::min(std::cmp::max(6, h / 4), std::cmp::max(1, h / 2))
}
```

- `max(6, h/4)` : au moins 6 rangs, environ un quart de l'écran.
- `min(…, max(1, h/2))` : jamais plus de la moitié de l'écran → protège les
  petits terminaux (build_world clampe déjà width/height à ≥ 1) et laisse la place
  au ciel + arbre.

`ground_top` en découle :

```rust
pub fn ground_top(world: &World) -> i32 {
    world.height as i32 - ground_rows(world)
}
```

Le resize reconstruit déjà tout le monde (`build_world`), donc la bande se
recalcule automatiquement.

### 2. Herbe sur N rangs (`scene.rs`)

`ground_frames` prend désormais le nombre de rangs et cycle les 4 motifs
`art::GRASS_ROWS` avec un décalage horizontal croissant par rang, pour casser la
répétition visible quand la bande dépasse 4 lignes :

```rust
pub fn ground_frames(width: usize, rows: i32) -> Vec<String> {
    // rangée i : motif GRASS_ROWS[i % 4] répété sur `width`, décalé de `i` glyphes.
    // Deux frames (pattern + version décalée d'un cran) pour l'ondulation existante.
}
```

Sortie : toujours 2 frames (animation d'ondulation), chaque frame = `rows` lignes
de `width` glyphes, aucune ligne vide.

### 3. Hauteur + profondeur des animaux (`scene.rs`)

Helper commun, tiré une fois par spawn :

```rust
/// Rangée-plancher (les « pieds ») tirée dans la bande pour un sprite de hauteur
/// `sprite_h`, et depth associé. L'animal tient entièrement dans la bande.
/// Plus haut dans la bande ⇒ depth plus grand ⇒ peint avant ⇒ derrière.
fn ground_slot(world: &mut World, sprite_h: i32) -> (i32, i32) {
    let top = ground_top(world);
    let bottom = world.height as i32 - 1;
    let lo = std::cmp::min(top + sprite_h - 1, bottom); // garde le corps dans la bande
    let feet = world.rng.gen_range(lo..=bottom);
    let rank = bottom - feet;                            // 0 = tout en bas (devant)
    let depth = std::cmp::min(engine::DEPTH_GROUND_ANIMAL + rank, engine::DEPTH_GRASS - 1);
    (feet, depth)
}
```

- `feet ∈ [top + h - 1, bottom]` garantit que le haut du sprite (`feet - (h-1)`)
  reste ≥ `top` : corps entièrement dans la bande.
- `lo = min(top + h - 1, bottom)` : si le sprite est plus haut que la bande, on
  retombe sur `bottom` (comportement bas-de-bande, pas de panique sur la plage).
- `depth` borné à `DEPTH_GRASS - 1` (44) : les animaux restent toujours devant
  l'herbe (45) ; la fenêtre de perspective va de `DEPTH_GROUND_ANIMAL` (40, le plus
  proche) à 44 (le plus loin). Bornée quel que soit l'écran.

Câblage :

- `ground_walker` : `let (feet, depth) = ground_slot(world, h);` puis
  `y = (feet - (h - 1)) as f64`, et passe `depth` à `cross_factory` (au lieu de
  `DEPTH_GROUND_ANIMAL` en dur).
- `ground_hopper` : idem pour `y`/`depth`, et `Behavior::Hop { ground_y: (feet + 1) as f64, … }`
  (le Hop pose le bas du sprite à `ground_y - 1`, donc `feet + 1` place les pieds
  au repos sur `feet`).

`cross_factory` prend déjà `depth` en paramètre — pas de changement de signature.

## Impact sur les tests (`scene.rs`)

- `ground_top_leaves_band` : `World::new(40, 20)` → `ground_rows = min(max(6,5), 10) = 6`
  → `ground_top` attendu = **14** (au lieu de 16).
- `ground_frames_fill_all_rows` : appelle `ground_frames(30, N)` et vérifie `N`
  lignes (paramétrer le test sur `N`, ex. 6).
- `walkers_bottom_aligned_to_screen` → renommé `walkers_feet_within_band` :
  vérifie que les pieds (`y + hauteur`) tombent dans `[ground_top, height]` au lieu
  d'être exactement `height`.
- Nouveau test `ground_animals_get_perspective_depth` : sur plusieurs spawns
  seedés, un animal placé plus haut (feet plus petit) a un `depth` ≥ à celui d'un
  animal placé plus bas, et tous `< DEPTH_GRASS`.

Les autres tests (opacité, masques, gating jour/nuit) sont inchangés.

## Hors scope (YAGNI)

- Herbe plus clairsemée vers le haut de la bande (renfort de perspective).
- Parallaxe de vitesse (animaux du fond plus lents).
- Occlusion des animaux par l'herbe (nécessiterait une herbe multi-depth).

## Contraintes respectées

- Cœur pur : tout dans `scene.rs`, aucun `crossterm`, aucune I/O.
- Aucune dépendance nouvelle.
- Déterminisme conservé : les hauteurs viennent de `world.rng` (seed reproductible).
- Commentaires et docs en français.
