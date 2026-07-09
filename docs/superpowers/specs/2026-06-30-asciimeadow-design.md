# asciimeadow — Design

Date : 2026-06-30

## Objectif

Recréer l'esprit d'`asciiquarium` mais avec un décor de **prairie** : un arbre
au centre, un ciel au-dessus, un sol en bas. Des créatures animées passent en
continu dans trois zones — le ciel, l'arbre et le sol — avec spawn aléatoire,
couches de profondeur et couleur, fidèlement au modèle d'asciiquarium.

Programme **terminal en Python**, basé sur `curses` (bibliothèque standard,
zéro dépendance), lancé via `python -m asciimeadow`.

## Layout de la scène

La scène s'adapte à la taille du terminal (largeur `W`, hauteur `H`) et est
reconstruite à chaque redimensionnement.

- **Ciel** — bande haute, environ 60 % des lignes.
  - Soleil fixe dans le coin haut-droit.
  - Nuages qui dérivent horizontalement (lents).
  - Vols d'oiseaux qui traversent l'écran (groupes de 3-5).
  - Avion / montgolfière occasionnels.
  - Papillons volant en zigzag, plutôt dans le bas du ciel près du sol.
- **Arbre** — centré horizontalement, art statique (tronc + canopée).
  - Écureuil qui grimpe le tronc puis longe une branche, en boucle.
  - Hibou perché (statique, cligne occasionnellement).
  - Abeilles qui orbitent la canopée (petits cercles / zigzag).
  - Pommes et feuilles qui tombent de la canopée vers le sol (gravité).
- **Sol** — bande basse, environ 4-5 lignes.
  - Ligne d'herbe ondulante (animation sur place) + fleurs dispersées (statiques).
  - Lapins qui sautillent (oscillation verticale en se déplaçant).
  - Renard qui trotte en traversant.
  - Hérisson, souris, escargot qui rampent (escargot le plus lent).

## Profondeur (z-order)

Comme asciiquarium, chaque entité a une profondeur ; le compositor dessine du
fond vers l'avant. Convention : **plus grand = plus loin** (dessiné en premier).

Ordre, du fond vers l'avant :

1. Soleil (le plus loin)
2. Nuages
3. Oiseaux / papillons / avion / montgolfière (créatures du ciel)
4. Arbre (tronc + canopée, statique)
5. Créatures de l'arbre (écureuil, hibou, abeilles, objets qui tombent)
6. Animaux du sol
7. Herbe / fleurs de premier plan (le plus proche)

Les valeurs exactes seront des constantes nommées dans `engine.py` (ex.
`DEPTH_SUN`, `DEPTH_CLOUD`, …) pour rester lisibles et ajustables.

## Modèle d'entité

L'`Entity` est l'unité de base animée. Champs :

- `frames` — liste d'images ASCII multi-lignes (les images de l'animation).
- `color_mask` — optionnel, même forme que la frame, mappe un caractère de
  couleur à chaque cellule (style asciiquarium) ; sinon couleur unie.
- `x`, `y` — position flottante du coin haut-gauche du sprite.
- `dx`, `dy` — vélocité en cellules par seconde.
- `depth` — entier de z-order.
- `frame_rate` — vitesse d'animation (images par seconde).
- `on_death` — callback appelé quand l'entité sort de l'écran (ou meurt) ;
  sert à respawn ou retirer l'entité.
- Caractère espace `' '` = transparent (laisse voir ce qui est derrière).
- `default_color` — couleur par défaut si pas de masque.

Comportements dérivés (sautillement du lapin, orbite des abeilles, gravité des
pommes, zigzag des papillons) sont implémentés via une fonction de mise à jour
par type qui ajuste `dx`/`dy`/`y` au fil du temps — pas un champ supplémentaire
sur `Entity`.

## Système de spawn

Un gestionnaire de population (`spawn.py`) tient un registre : pour chaque type
de créature, une fonction de fabrication + une population cible (ou une
probabilité d'apparition par tick). Quand une entité sort de l'écran, son
`on_death` la retire et le gestionnaire en recrée pour maintenir la population.

Le côté d'entrée (gauche/droite) est tiré au hasard ; le sprite est miroité
selon la direction de déplacement.

## Boucle principale

- Timestep fixe, environ 20 images/seconde.
- Entrées clavier : `q` quitter, `p` pause, `r` forcer le redraw.
- `KEY_RESIZE` → reconstruit le décor statique (arbre, sol, soleil) aux
  nouvelles dimensions.
- Chaque tick : update (déplacer, animer, retirer les entités hors-écran) →
  spawn → render.
- Render : composition par profondeur, l'espace est transparent, couleurs via
  les paires de couleurs curses.

## Couleur

`curses.init_pair` pour les couleurs nécessaires : vert (herbe, canopée), brun
(tronc), jaune (soleil), blanc (nuages), plus les couleurs des animaux. Chaque
entité a une couleur unie par défaut et, optionnellement, un masque de couleur
par caractère.

## Structure des fichiers

```
asciimeadow/
  engine.py     wrapper Screen, Entity, compositor z-order, primitives de boucle
  art.py        tout l'art ASCII + masques couleur (décor + créatures)
  scene.py      construit la prairie : place le décor, enregistre les spawners,
                définit les comportements par créature
  spawn.py      gestionnaire de population
  __main__.py   entrée CLI, curses.wrapper, boucle principale
```

Chaque module a une responsabilité claire : `engine` ne connaît pas les
créatures concrètes ; `art` ne contient que des données ; `scene` et `spawn`
assemblent le tout ; `__main__` pilote curses.

## Tests

La logique du moteur est testable sans terminal (headless) via un faux buffer
d'écran :

- déplacement d'`Entity` dans le temps (intégration de `dx`/`dy`) ;
- retrait des entités hors-écran (culling) ;
- ordre de dessin du compositor (z-order, transparence de l'espace) ;
- maintien de la population par le spawner.

TDD sur cette logique pure. Le rendu curses lui-même est validé manuellement
(lancement réel), pas en test unitaire.

## Hors scope (YAGNI)

Notés comme évolutions futures possibles, non implémentés au départ :

- cycle jour/nuit (étoiles, lune, dégradé de couleur) ;
- météo (pluie, vent) ;
- son ;
- easter eggs / interactivité avancée.
