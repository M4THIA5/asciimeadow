# Publish CI — synchronisation version / tag / COPR

Date : 2026-07-08

> **Errata (implémentation).** cocogitto crée un tag **léger** par défaut :
> les mentions « tag annoté » et le `git push --follow-tags` unique de ce
> document sont **caducs**. La CI pousse commit + tag ensemble via
> `git push --atomic origin main vX.Y.Z` et déclenche `cog bump` avec
> `--skip-ci`. Source de vérité : `cog.toml`, `.github/workflows/publish.yml`
> et `docs/RELEASING.md`. Ne pas réintroduire `--follow-tags`.

## Contexte

Aujourd'hui la version « source de vérité » est le **tag git**. Le
`.copr/Makefile` la dérive dans cet ordre : tag exact sur HEAD → `git
describe --always` (snapshot) → fallback `Cargo.toml`. Il réécrit ensuite
`Cargo.toml` et le `Version:` du `.spec` par `sed` au moment du build, pour
les aligner sur cette version.

Conséquences : `Cargo.toml` (`0.1.0`) n'est jamais bumpé à la main, le
`.spec` a un `Version:` figé, et rien n'automatise la création des tags de
release.

Objectif : une CI **Publish** qui, au push sur `main`, calcule la prochaine
version (conventional commits), bumpe `Cargo.toml` + `.spec`, met à jour
`CHANGELOG.md`, commit, tag, push, puis déclenche COPR. Une fois en place,
les deux `sed` du `.copr/Makefile` deviennent inutiles et sont retirés.

## Décisions

- **Calcul de version** : semver automatique sur *conventional commits*
  depuis le dernier tag. `feat` → minor, `fix` → patch, `BREAKING CHANGE` →
  major. Aucun commit releasable → aucune release.
- **Modèle** : release **directe sur `main`** (pas de release PR). Le push
  sur `main` déclenche le bump + tag + push automatiquement.
- **Outil** : [cocogitto](https://docs.cocogitto.io/) (`cog`), natif
  conventional-commits pour Rust.
- **Changelog** : `CHANGELOG.md` généré/maj par `cog` et committé dans le
  commit de release.
- **`.spec`** : le `Version:` **et** une entrée `%changelog` sont bumpés par
  la CI (via hooks `cog`). Les **deux** `sed` du `.copr/Makefile` (Cargo.toml
  et `Version:`) sont retirés.
- **Déclenchement COPR** : la CI appelle **explicitement** COPR (POST sur une
  URL de webhook COPR stockée en secret GitHub) après un bump réussi.
  L'auto-rebuild COPR sur push est désactivé → chaque build COPR correspond à
  un commit de release aligné.

## Architecture

Un seul workflow : `.github/workflows/publish.yml`, `on: push: branches:
[main]`.

```
permissions:
  contents: write        # push commit + tag sur main
concurrency:
  group: publish         # sérialise les runs, pas de bump concurrent
  cancel-in-progress: false
```

### Anti-boucle

Le commit de release est poussé avec le `GITHUB_TOKEN` par défaut. Les pushes
faits avec ce token **ne re-déclenchent pas** GitHub Actions → pas de boucle
infinie. Ceinture-bretelles : le message du commit de release contient
`[skip ci]`. (Le webhook externe COPR, lui, n'est pas déclenché par ce push
puisque l'auto-rebuild est désactivé ; c'est la CI qui appelle COPR.)

### Étapes du job

1. `actions/checkout` avec `fetch-depth: 0` et récupération des tags (`cog` a
   besoin de tout l'historique et des tags depuis `v0.1.0`).
2. Installer `cog` (cocogitto) et `cargo-edit` (fournit `cargo set-version`).
3. `git config` d'un utilisateur bot (nom + email).
4. `cog bump --auto` :
   - calcule le prochain semver depuis le dernier tag ;
   - **rien à releaser → sortie no-op gérée, le job se termine proprement
     sans release ni appel COPR** ;
   - exécute les `pre_bump_hooks` (voir `cog.toml`) ;
   - met à jour `CHANGELOG.md` ;
   - crée le commit de release `chore(version): {{version}} [skip ci]`
     incluant `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`,
     `packaging/asciimeadow.spec` ;
   - crée le tag annoté `v{{version}}` ;
   - exécute les `post_bump_hooks`.
5. Déclenchement COPR : `curl -X POST "$COPR_WEBHOOK_URL"` où
   `COPR_WEBHOOK_URL` est un secret GitHub. **Ne s'exécute que si un bump a eu
   lieu** (pas sur no-op). Implémenté de préférence en `post_bump_hook` de
   `cog` (garantit l'auto-gating : ne tourne que quand un bump se produit),
   avec le secret exposé en variable d'environnement de l'étape.

### `cog.toml` (nouveau, racine)

Configuration attendue :

- `tag_prefix = "v"`
- `bump_commit_message = "chore(version): {{version}} [skip ci]"`
- `pre_bump_hooks` :
  - `cargo set-version {{version}}` (bumpe `Cargo.toml` + relock `Cargo.lock`)
  - `sed -i -E 's/^Version:.*/Version:        {{version}}/'
    packaging/asciimeadow.spec`
  - insertion d'une nouvelle entrée en tête de `%changelog` du `.spec`
    (format `* <date> Mathias Collas <mathias.collas@gmail.com> -
    {{version}}-1` + ligne de description), la plus récente en premier.
- `post_bump_hooks` :
  - `git push origin main --follow-tags` (commit + tag annoté en **un seul
    push**)
  - `curl -X POST "$COPR_WEBHOOK_URL"` (déclenchement COPR)

La syntaxe exacte des hooks (échappement `sed`/`awk` pour l'entrée
`%changelog`, détection du no-op) est à finaliser dans le plan
d'implémentation.

### Côté COPR (`.copr/Makefile`)

- Retirer le `sed ... Cargo.toml` (redondant : la CI a déjà bumpé
  `Cargo.toml`, committé sur le commit de release que COPR va cloner).
- Retirer le `sed Version: ... $(spec)` (redondant : la CI a déjà bumpé le
  `.spec`).
- **Conserver** la dérivation de `VERSION` depuis le tag : elle sert encore à
  nommer les tarballs et au `--transform` de `tar`. Sur un commit de release,
  `git describe --exact-match --tags HEAD` renvoie `v{{version}}`, donc
  `VERSION` == version `Cargo.toml` == `Version:` du `.spec`. Tout est
  aligné.

### Côté COPR (configuration web, hors dépôt)

- **Désactiver l'auto-rebuild sur push**. La CI est le seul déclencheur.
- Récupérer l'URL de webhook COPR du package et la stocker en secret GitHub
  `COPR_WEBHOOK_URL`. Le committish COPR reste `main` : au moment où COPR
  fetch, `main` HEAD == le commit de release taggé.

## Flux nominal

```
dev push feat(...) sur main
  -> publish.yml
     cog bump --auto : 0.1.0 -> 0.2.0
       set-version Cargo.toml/Cargo.lock
       sed .spec Version: + %changelog
       maj CHANGELOG.md
       commit "chore(version): 0.2.0 [skip ci]"
       tag v0.2.0
       git push origin main --follow-tags   (1 push, commit+tag)
       curl POST COPR_WEBHOOK_URL
  -> COPR fetch main HEAD (= commit v0.2.0)
       git describe --exact-match -> v0.2.0
       Makefile : VERSION=0.2.0, aucun sed
       SRPM build -> build RPM
```

## Cas limites

- **Rien à releaser** (ex. push `docs:` seul) : `cog bump --auto` ne fait
  rien, le job se termine sans commit/tag/push ni appel COPR. Le YAML doit
  gérer proprement le code de sortie no-op de `cog`.
- **Runs concurrents** : le `concurrency.group` sérialise ; deux pushes
  rapprochés ne produisent pas deux bumps en course.
- **Push bloqué par branch protection** : si `main` est protégé et refuse le
  push du `GITHUB_TOKEN`, il faudra un PAT dédié ou autoriser le bot. À
  vérifier (aucune protection détectée à ce jour).
- **Cohérence des sources** : les builds COPR étant désormais exclusivement
  déclenchés par la CI sur commits de release, il n'y a plus de build
  « snapshot » où `.spec`/`Cargo.toml`/tag divergeraient. Le risque
  d'échec `Source0` par nom de tarball non concordant est éliminé.

## Tests / validation

Pas de tests Rust (infra CI/YAML). Validation :

- `cog bump --auto --dry-run` en local pour vérifier le calcul de version et
  l'enchaînement des hooks sans pousser.
- Exécution réelle sur une branche jetable (ou un fork) pour vérifier
  bump + commit + tag + push + `[skip ci]`.
- Vérifier que `.copr/Makefile` produit un SRPM correct sur un commit taggé
  **sans** les `sed` (version tarball == `Version:` du `.spec`).
- `rpmlint` sur le `.spec` pour confirmer que l'entrée `%changelog` générée
  est valide.

## Prérequis hors dépôt (action utilisateur)

1. Créer le secret GitHub `COPR_WEBHOOK_URL` (URL de webhook du package
   COPR).
2. Désactiver l'auto-rebuild-on-push côté COPR.
3. Confirmer qu'aucune branch protection ne bloque le push du `GITHUB_TOKEN`
   sur `main`.

## Hors périmètre (YAGNI)

- Release PR / validation humaine du bump.
- Publication sur crates.io.
- Signature des tags/commits.
- Matrice multi-plateforme de build dans la CI (COPR gère le build RPM).
