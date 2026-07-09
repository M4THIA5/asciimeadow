# Refonte visuelle de la prairie — arbre, herbe, animaux

Date : 2026-07-03
Statut : validé (choix arbre + herbe confirmés par Mathias ; gabarit animaux
« compact 2-3 lignes » = option recommandée, à confirmer à la relecture)

## Objectif

Trois améliorations visuelles, sans changement de comportement du moteur :

1. **Arbre plus grand**, adapté à la taille du terminal.
2. **Herbe dense sur toute la bande basse** (4 lignes), pas une seule ligne.
3. **Sprites animaux reconnaissables** : bird, squirrel, owl, rabbit, fox,
   hedgehog, mouse. Papillon, abeille et escargot restent inchangés.

## 1. Arbre adaptatif

Deux variantes dans `art.py`, chacune avec son masque couleur :

- `TREE_SMALL` : l'arbre actuel (11×7), inchangé.
- `TREE_LARGE` : ~25×14 — canopée arrondie sur ~8 lignes, tronc à double
  colonne avec branches et évasement racinaire. Croquis (à affiner
  visuellement à l'implémentation) :

  ```
          .@@@@@@.
       @@@@@@@@@@@@@@
     @@@@@@@@@@@@@@@@@@
    @@@@@@@@@@@@@@@@@@@@
   @@@@@@@@@@@@@@@@@@@@@@
    @@@@@@@@@@@@@@@@@@@@
      @@@@@@@@@@@@@@@@
        '@@@@@@@@@@'
           \|  |/
            |  |
           /|  |\
          / |  | \
            |  |
         ___|  |___
  ```

  Masque : canopée `g` (vert), tronc/branches/racines `n` (marron).

Sélection dans `scene.py` : nouvelle fonction `tree_art(world)` qui renvoie
`(frames, mask)` — `TREE_LARGE` si `world.height >= 28` **et**
`world.width >= 40`, sinon `TREE_SMALL`. `tree_origin(world)` et tous les
spawners liés à l'arbre (`spawn_squirrel`, `spawn_owl`, `spawn_apple`,
`spawn_bee`) passent par `tree_art`/`tree_origin` au lieu de lire `art.TREE`
en dur, pour que la géométrie (largeur de canopée, hauteur de tronc, perchoir
du hibou) suive la variante. Le resize reconstruisant déjà toute la scène,
le changement de variante est automatique.

## 2. Herbe dense (bande de 4 lignes)

`GROUND_ROWS` reste à 4. `_ground_frames(width)` génère 4 lignes **toutes
remplies** de touffes variées (`v w W , ' . "`), motifs différents par ligne
(plus dense en haut, plus clairsemé vers le bas), répétés jusqu'à `width`.
Deux frames : la seconde décale chaque ligne d'un caractère (ondulation
existante, étendue à toute la bande). Couleur unie verte.

**Correction de profondeur** : la bande dessinée à `DEPTH_FOREGROUND` (30)
écraserait les animaux (`DEPTH_GROUND_ANIMAL` = 40, dessinés avant). Nouvelle
constante dans `engine.py` : `DEPTH_GRASS = 45` (entre créatures d'arbre 50 et
animaux du sol 40). La bande d'herbe passe à `DEPTH_GRASS` : les animaux
marchent *devant* l'herbe.

Les fleurs sont dispersées sur **toute la bande** (`y` aléatoire entre
`ground_top` et `height - 1`) au lieu de la seule ligne du haut, et restent à
`DEPTH_FOREGROUND` (petits accents d'un caractère devant les animaux,
acceptable, style asciiquarium).

## 3. Sprites animaux détaillés

Gabarit **compact : 2 à 3 lignes** (4 pour le hibou), pour rester proportionné
à l'arbre et à la bande d'herbe. Chaque sprite reçoit un **masque couleur**
multi-tons (liste parallèle aux frames). Tous dessinés orientés **droite** ;
`_cross_factory` miroite déjà sprite + masque via `flip_horizontal`.

Traits distinctifs obligatoires par animal (les croquis ci-dessous sont des
brouillons ; l'apparence finale se règle à l'implémentation en lançant l'app
et en vérifiant visuellement) :

| Animal | Traits clés | Croquis (frame A, lignes séparées par `/`) |
|---|---|---|
| Oiseau (2 frames, battement) | bec, ailes hautes/basses | ` \\,` / `(o \__` / ` \__/` |
| Écureuil (2 frames, grimpe) | queue en panache, pose verticale | ` @,` / `(o )` / ` (,,&` |
| Hibou (2 frames, clignement) | aigrettes, grands yeux `O.O`/`-.-` | ` ^v^` / `(O.O)` / `({.})` / ` " "` |
| Lapin (2 frames, saut) | longues oreilles, queue pompon | ` (\_/)` / ` (o.o)` / `(")_(")` |
| Renard | museau pointu, queue touffue | ` /\,_` / `(o \____,` / ` \_" "  \_~` |
| Hérisson | dos de piquants `;`, museau | ` ,;;;;;;,` / `';;;;;;;(o>` / `  " " " "` |
| Souris | oreille ronde, longue queue traînante | `  __` / `~~(o \` / `   " "` |

Ajustements dans `scene.py` :

- `_ground_walker` : `y = world.height - hauteur_du_sprite` (au lieu de
  `ground_top + GROUND_ROWS - 1`, qui ferait déborder un sprite multi-lignes
  sous l'écran).
- `make_hop` utilise déjà `e.height()` — rien à changer.
- Les factories sol passent désormais `color_mask`.
- Perchoir du hibou : calculé depuis les dimensions de la variante d'arbre
  (centré dans la canopée) au lieu de `tox + 2, toy + 1` en dur.
- L'écureuil grimpe sur la colonne du tronc de la variante courante
  (via `tree_origin`, déjà le cas).

Note `flip_horizontal` : la table ne traduit que `<>[](){}/\` ; les lettres
des masques (`g`, `n`, `w`…) sont seulement inversées ligne à ligne, ce qui
est correct.

## Hors périmètre

Papillon, abeille, escargot, nuages, soleil : inchangés. Aucune nouvelle
créature, aucun nouveau comportement de déplacement.

## Erreurs et cas limites

- Terminal petit : seuil `TREE_LARGE` garantit l'arbre grand seulement quand
  il tient (14 + 4 lignes de sol ≤ 28 laisse ≥ 10 lignes de ciel).
- Sprites multi-lignes près des bords : `Buffer.draw_entity` clippe déjà
  hors-écran ; le culling directionnel de `World._offscreen` fonctionne avec
  n'importe quelle taille de sprite.
- Masques : longueur de chaque ligne de masque ≤ ligne de frame acceptée par
  `draw_entity` (indices vérifiés) ; on impose néanmoins des masques de même
  grille exacte, validés par test.

## Tests (headless, TDD)

Nouveaux tests dans `tests/` :

1. **Intégrité de l'art** : pour chaque paire (frames, masque) exportée par
   `art.py` : même nombre de frames, même nombre de lignes et mêmes longueurs
   ligne à ligne.
2. **Sélection d'arbre** : monde 80×24 → `TREE_SMALL` ; monde 100×35 →
   `TREE_LARGE` ; `tree_origin` cohérent avec la variante.
3. **Herbe** : `_ground_frames` renvoie 2 frames de `GROUND_ROWS` lignes de
   largeur exacte, chaque ligne contenant des caractères non-espace.
4. **Profondeur** : la bande d'herbe est à `DEPTH_GRASS` et
   `DEPTH_GROUND_ANIMAL < DEPTH_GRASS`.
5. **Placement des marcheurs** : un walker multi-lignes a ses pieds sur
   `world.height - 1`.

La qualité visuelle des sprites (reconnaissabilité) se vérifie en lançant
l'app (`run-asciimeadow`), pas en test unitaire — même approche que le design
initial.
