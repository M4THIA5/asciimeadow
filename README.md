# asciimeadow

Économiseur d'écran pour terminal : une prairie ASCII animée — arbre, animaux,
météo et cycle jour/nuit — rendue avec [`crossterm`](https://crates.io/crates/crossterm).

## Lancer

Nécessite un vrai terminal (TTY).

```bash
cargo run                                  # animation par défaut
cargo run -- --seed 42                     # rendu déterministe
cargo run -- --fps 30 --day-length 60      # 30 img/s, journée de 60 s
```

## Touches

| Touche   | Action          |
|----------|-----------------|
| `q`      | quitter         |
| `p`      | pause           |
| `r`      | redessiner      |
| `Ctrl+C` | quitter propre  |

Un redimensionnement du terminal reconstruit le monde.

## Installation (Fedora / COPR)

```bash
sudo dnf copr enable m4thia5/asciimeadow
sudo dnf install asciimeadow
asciimeadow
```

## Développement

```bash
cargo test                # tous les tests (lib + bin)
cargo test --lib scene    # les tests d'un module
```

L'architecture sépare un cœur pur (bibliothèque, sans I/O, testable) d'une
coquille terminal (`src/main.rs`, seul module qui connaît `crossterm`).
Voir [`CLAUDE.md`](CLAUDE.md) pour les détails.

## Licence

Sous double licence [MIT](LICENSE-MIT) **OU** [Apache-2.0](LICENSE-APACHE), au choix.
