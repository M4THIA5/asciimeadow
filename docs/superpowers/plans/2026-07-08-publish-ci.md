# Publish CI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Au push sur `main`, une CI calcule la prochaine version (conventional commits), bumpe `Cargo.toml` + le `.spec` RPM, met à jour `CHANGELOG.md`, commit + tag + push, puis déclenche un build COPR.

**Architecture :** Un workflow GitHub Actions unique pilote [cocogitto](https://docs.cocogitto.io/) (`cog bump --auto`). Un gate `--dry-run` décide s'il y a une release ; le vrai bump applique des hooks (`cargo set-version`, un script de bump du `.spec`), pousse commit+tag en un seul push, puis un post-bump hook `curl` déclenche COPR. Les deux `sed` du `.copr/Makefile` deviennent inutiles et sont retirés.

**Tech Stack :** GitHub Actions, cocogitto (`cog`), cargo-edit (`cargo set-version`), bash/sed (GNU), Makefile COPR, spec RPM.

## Global Constraints

- Commentaires et docs en **français** (règle du dépôt, CLAUDE.md).
- Les dépendances *du crate* restent limitées à `crossterm` + `rand` : ce plan n'ajoute **aucune** dépendance Rust (les outils `cog`/`cargo-edit` sont installés dans la CI uniquement).
- Préfixe de tag : `v` (ex. `v0.2.0`). Source de vérité de version = le tag git.
- Conventional commits : `feat` → minor, `fix` → patch, `BREAKING CHANGE` → major.
- Le repo est en `0.y.z` : `cog bump --auto` ne passera jamais tout seul à `1.0.0` (règle cocogitto), un `feat` bumpe le mineur.
- Ne jamais réintroduire un `sed` sur `Cargo.toml`/`Version:` dans le `.copr/Makefile` : c'est désormais la CI qui bumpe ces fichiers.

---

### Task 1: Script de bump du `.spec` RPM (`bump-spec.sh`)

Un petit script bash qui met à jour le champ `Version:` et préfixe une entrée `%changelog` (la plus récente en premier). Paramétrable sur le chemin du `.spec` pour être testable sur une fixture.

**Files:**
- Create: `packaging/bump-spec.sh`
- Test: `packaging/tests/bump-spec.test.sh`

**Interfaces:**
- Consumes: rien.
- Produces: exécutable `bash packaging/bump-spec.sh <version> [spec-path]`. Défaut `spec-path` = `packaging/asciimeadow.spec`. Effets : `Version:` réécrit à `<version>` ; nouvelle entrée `%changelog` `* <date C-locale> Mathias Collas <mathias.collas@gmail.com> - <version>-1` + `- Release <version>` insérée juste après la ligne `%changelog`. Appelé par `cog.toml` (Task 2).

- [ ] **Step 1: Écrire le test qui échoue**

Create `packaging/tests/bump-spec.test.sh` :

```bash
#!/usr/bin/env bash
# Test de packaging/bump-spec.sh sur une fixture .spec jetable.
set -euo pipefail

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
spec="$tmp/test.spec"

cat > "$spec" <<'EOF'
Name:           asciimeadow
Version:        0.1.0
Release:        1%{?dist}

%changelog
* Wed Jul 08 2026 Mathias Collas <mathias.collas@gmail.com> - 0.1.0-1
- Paquet initial
EOF

bash packaging/bump-spec.sh 0.2.0 "$spec"

fail() { echo "FAIL: $1"; exit 1; }

if ! grep -qE '^Version:        0\.2\.0$' "$spec"; then fail "Version non bumpée"; fi
if grep -qE '^Version:        0\.1\.0$' "$spec"; then fail "ancienne Version encore présente"; fi
if ! grep -q '^- Release 0.2.0$' "$spec"; then fail "ligne de release manquante"; fi

new_line="$(grep -n -- '- 0\.2\.0-1$' "$spec" | head -1 | cut -d: -f1)"
old_line="$(grep -n -- '- 0\.1\.0-1$' "$spec" | head -1 | cut -d: -f1)"
if [ -z "$new_line" ] || [ -z "$old_line" ]; then fail "entrées changelog introuvables"; fi
if [ "$new_line" -ge "$old_line" ]; then fail "nouvelle entrée pas en premier ($new_line >= $old_line)"; fi

echo "PASS"
```

- [ ] **Step 2: Lancer le test, vérifier qu'il échoue**

Run: `bash packaging/tests/bump-spec.test.sh`
Expected: FAIL — `bash: packaging/bump-spec.sh: No such file or directory` (exit non-zéro), car le script n'existe pas encore.

- [ ] **Step 3: Écrire le script**

Create `packaging/bump-spec.sh` :

```bash
#!/usr/bin/env bash
# Aligne un .spec RPM sur une nouvelle version : réécrit « Version: » et
# préfixe une entrée %changelog (la plus récente en premier). Appelé par un
# pre_bump_hook de cog (voir cog.toml). LC_ALL=C pour une date au format RPM
# anglophone stable.
set -euo pipefail

version="${1:?usage: bump-spec.sh <version> [spec-path]}"
spec="${2:-packaging/asciimeadow.spec}"
author="Mathias Collas <mathias.collas@gmail.com>"
date_str="$(LC_ALL=C date '+%a %b %d %Y')"

# Champ Version:
sed -i -E "s/^Version:.*/Version:        ${version}/" "$spec"

# Nouvelle entrée %changelog, insérée juste après la ligne « %changelog »
# (GNU sed, commande « a » une-ligne, \n interprété comme saut de ligne).
sed -i "/^%changelog$/a * ${date_str} ${author} - ${version}-1\n- Release ${version}\n" "$spec"
```

Then: `chmod +x packaging/bump-spec.sh`

- [ ] **Step 4: Lancer le test, vérifier qu'il passe**

Run: `bash packaging/tests/bump-spec.test.sh`
Expected: `PASS`

- [ ] **Step 5: Commit**

```bash
git add packaging/bump-spec.sh packaging/tests/bump-spec.test.sh
git commit -m "feat(ci): script de bump du .spec RPM (Version + %changelog)"
```

---

### Task 2: Configuration cocogitto (`cog.toml`)

**Files:**
- Create: `cog.toml`

**Interfaces:**
- Consumes: `packaging/bump-spec.sh` (Task 1) via un pre-bump hook ; la variable d'environnement `COPR_WEBHOOK_URL` (fournie par la CI, Task 3) via un post-bump hook.
- Produces: comportement de `cog bump --auto` : bump `Cargo.toml`+`Cargo.lock`+`.spec`, génère `CHANGELOG.md`, commit `chore(version): <v> [skip ci]` (le `[skip ci]` est ajouté automatiquement par cog), tag annoté `v<version>`, push `main`+tag, `curl` COPR.

- [ ] **Step 1: Écrire `cog.toml`**

Create `cog.toml` :

```toml
# Configuration cocogitto : versioning automatique par conventional commits.
# La CI « Publish » (.github/workflows/publish.yml) lance `cog bump --auto`
# au push sur main. Détails : docs/RELEASING.md.

tag_prefix = "v"
from_latest_tag = true
ignore_merge_commits = true
branch_whitelist = ["main"]

# Avant le commit de version : aligner Cargo.toml (+ Cargo.lock via cargo-edit)
# et le .spec RPM sur la nouvelle version. {{version}} = version cible.
pre_bump_hooks = [
    "cargo set-version {{version}}",
    "bash packaging/bump-spec.sh {{version}}",
]

# Après le tag : pousser commit + tag annoté en un seul push, puis déclencher
# COPR. COPR_WEBHOOK_URL est injecté par la CI (secret GitHub) ; -f fait
# échouer le hook si le webhook renvoie une erreur HTTP.
post_bump_hooks = [
    "git push origin main --follow-tags",
    "curl -fsS -X POST \"$COPR_WEBHOOK_URL\"",
]

# Changelog avec liens vers GitHub.
[changelog]
path = "CHANGELOG.md"
template = "remote"
remote = "github.com"
owner = "M4THIA5"
repository = "asciimeadow"
```

- [ ] **Step 2: Valider la syntaxe TOML**

Run: `python3 -c "import tomllib; tomllib.load(open('cog.toml','rb')); print('OK')"`
Expected: `OK`

- [ ] **Step 3: (validation complète, optionnelle) dry-run cocogitto**

Nécessite `cog` en local. Si absent : `cargo install cocogitto --locked`.
Run: `cog bump --dry-run --auto`
Expected: `0.2.0` (un `feat` existe depuis `v0.1.0` → bump mineur). Aucun fichier n'est modifié (dry-run).

Si `cog` n'est pas installable ici, sauter ce step : la validation réelle se fera au premier run CI (Task 3).

- [ ] **Step 4: Commit**

```bash
git add cog.toml
git commit -m "feat(ci): configuration cocogitto (bump auto + hooks Cargo/.spec/COPR)"
```

---

### Task 3: Workflow GitHub Actions (`publish.yml`)

**Files:**
- Create: `.github/workflows/publish.yml`

**Interfaces:**
- Consumes: `cog.toml` (Task 2) ; le secret GitHub `COPR_WEBHOOK_URL` (créé hors dépôt, Task 5).
- Produces: sur push `main`, exécute le gate dry-run puis `cog bump --auto` avec `COPR_WEBHOOK_URL` en env. Push via `GITHUB_TOKEN` (ne re-déclenche pas Actions → pas de boucle).

- [ ] **Step 1: Écrire le workflow**

Create `.github/workflows/publish.yml` :

```yaml
name: Publish

on:
  push:
    branches: [main]

# Push du commit + tag de release sur main.
permissions:
  contents: write

# Sérialise les runs : pas de bump concurrent.
concurrency:
  group: publish
  cancel-in-progress: false

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout (historique complet + tags)
        uses: actions/checkout@v4
        with:
          fetch-depth: 0
          fetch-tags: true

      - name: Toolchain Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Installer cocogitto + cargo-edit
        run: cargo install cocogitto cargo-edit --locked

      - name: Configurer l'identité git du bot
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "41898282+github-actions[bot]@users.noreply.github.com"

      - name: Déterminer s'il y a une release à faire
        id: check
        run: |
          if version="$(cog bump --dry-run --auto 2>/dev/null)"; then
            echo "release=true" >> "$GITHUB_OUTPUT"
            echo "version=$version" >> "$GITHUB_OUTPUT"
            echo "Prochaine version : $version"
          else
            echo "release=false" >> "$GITHUB_OUTPUT"
            echo "Aucun commit releasable depuis le dernier tag."
          fi

      - name: Bump + tag + push + déclenchement COPR
        if: steps.check.outputs.release == 'true'
        env:
          COPR_WEBHOOK_URL: ${{ secrets.COPR_WEBHOOK_URL }}
        run: cog bump --auto
```

- [ ] **Step 2: Valider la syntaxe YAML**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/publish.yml')); print('OK')"`
Expected: `OK`

(Si `actionlint` est disponible : `actionlint .github/workflows/publish.yml` — attendu : aucune erreur. Sinon sauter.)

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/publish.yml
git commit -m "feat(ci): workflow Publish (bump/tag/push + trigger COPR au push main)"
```

- [ ] **Step 4: Validation end-to-end (manuelle, après merge sur main)**

Non automatisable ici (nécessite le secret + un vrai push main). À faire une fois les prérequis de Task 5 en place :
1. Push un commit `fix:`/`feat:` sur `main`.
2. Vérifier dans l'onglet Actions que le job `release` bumpe, crée le tag `vX.Y.Z`, push le commit `chore(version): X.Y.Z [skip ci]`.
3. Vérifier qu'**aucun** second run Actions n'est déclenché par le push du bot (grâce au `GITHUB_TOKEN`).
4. Vérifier qu'un build COPR démarre.

---

### Task 4: Retirer les deux `sed` du `.copr/Makefile`

Sur les commits de release, `Cargo.toml` et le `.spec` sont déjà bumpés (Tasks 1-3) et le tag est sur HEAD ; `git describe --exact-match` donne la bonne `VERSION`. Les `sed` d'alignement deviennent inutiles.

**Files:**
- Modify: `.copr/Makefile`

**Interfaces:**
- Consumes: rien (dépend de l'invariant « COPR ne build que des commits de release », garanti par le déclenchement CI-only — voir Task 5).
- Produces: recette `srpm` sans réécriture de `Cargo.toml` ni de `Version:`. `VERSION` (dérivée du tag) sert toujours au nommage des tarballs.

- [ ] **Step 1: Lire le fichier**

Run: `Read .copr/Makefile` (repérer le bloc du `sed ... Cargo.toml` et la ligne `sed ... "$(spec)" && \`).

- [ ] **Step 2: Supprimer le `sed` sur `Cargo.toml` (et son commentaire)**

Edit — remplacer :

```make
	# Aligner la version du paquet sur le tag (éphémère : dans ce clone uniquement).
	# NB : non ancré sur [package] — OK tant que seul [package] a une clé « version = »
	# de tête (les deps sont inline). À revoir si une dep passe en table [dependencies.x].
	sed -i -E 's/^version = ".*"/version = "$(VERSION)"/' Cargo.toml
```

par :

```make
	# Version : Cargo.toml et le .spec sont déjà bumpés par la CI « Publish »
	# sur le commit de release (tag vX.Y.Z). On ne réécrit plus rien ici.
```

- [ ] **Step 3: Supprimer le `sed` sur `Version:` du `.spec` et corriger le commentaire du trap**

Edit — remplacer :

```make
	# Chaînage `&&` + trap : toute erreur (tar, sed, rpmbuild) fait échouer la cible,
	# et le staging est nettoyé même en cas d'échec.
	staging=$$(mktemp -d) && \
	trap 'rm -rf "$$staging"' EXIT && \
	tar caf "$$staging/$(CRATE)-$(VERSION).tar.gz" \
		--exclude=./.git --exclude=./vendor --exclude='./*.tar.*' \
		--transform 's,^\.,$(CRATE)-$(VERSION),' . && \
	tar caf "$$staging/$(CRATE)-$(VERSION)-vendor.tar.xz" vendor/ && \
	sed -i -E 's/^Version:.*/Version:        $(VERSION)/' "$(spec)" && \
	rpmbuild -bs "$(spec)" \
		--define "_sourcedir $$staging" \
		--define "_srcrpmdir $(outdir)"
```

par :

```make
	# Chaînage `&&` + trap : toute erreur (tar, rpmbuild) fait échouer la cible,
	# et le staging est nettoyé même en cas d'échec.
	staging=$$(mktemp -d) && \
	trap 'rm -rf "$$staging"' EXIT && \
	tar caf "$$staging/$(CRATE)-$(VERSION).tar.gz" \
		--exclude=./.git --exclude=./vendor --exclude='./*.tar.*' \
		--transform 's,^\.,$(CRATE)-$(VERSION),' . && \
	tar caf "$$staging/$(CRATE)-$(VERSION)-vendor.tar.xz" vendor/ && \
	rpmbuild -bs "$(spec)" \
		--define "_sourcedir $$staging" \
		--define "_srcrpmdir $(outdir)"
```

- [ ] **Step 4: Vérifier qu'il ne reste que les `sed` de dérivation de version**

Run: `grep -n 'sed' .copr/Makefile`
Expected: seulement les 3 lignes de dérivation de `VERSION` (les `sed 's/^v//'` sur `git describe` et le `sed` du fallback `Cargo.toml`). **Aucune** ligne réécrivant `Cargo.toml` ou `Version:`.

- [ ] **Step 5: (validation complète, optionnelle) build SRPM**

Nécessite `rpmbuild` + `cargo` + réseau (Fedora). Sur un commit taggé :
Run: `make -f .copr/Makefile srpm outdir="$(mktemp -d)"`
Expected: un `.src.rpm` est produit ; la version du SRPM == `Version:` du `.spec` == version `Cargo.toml`. Sauter si l'outillage RPM est absent.

- [ ] **Step 6: Commit**

```bash
git add .copr/Makefile
git commit -m "refac(packaging): retirer les sed Cargo.toml/.spec (bump géré par la CI)"
```

---

### Task 5: Documentation du process de release

**Files:**
- Create: `docs/RELEASING.md`
- Modify: `CLAUDE.md` (ajouter un pointeur `## Release`)

**Interfaces:**
- Consumes: rien.
- Produces: doc du flux auto + checklist des prérequis hors dépôt (secret `COPR_WEBHOOK_URL`, désactivation auto-rebuild COPR, branch protection).

- [ ] **Step 1: Écrire `docs/RELEASING.md`**

Create `docs/RELEASING.md` :

```markdown
# Release

Les releases sont **automatiques**. Un push sur `main` déclenche la CI
« Publish » (`.github/workflows/publish.yml`), qui, via cocogitto :

1. calcule la prochaine version depuis les *conventional commits* depuis le
   dernier tag (`feat` → mineur, `fix` → correctif, `BREAKING CHANGE` →
   majeur ; aucun commit releasable → aucune release) ;
2. bumpe `Cargo.toml` (+ `Cargo.lock`) et `packaging/asciimeadow.spec`
   (`Version:` + entrée `%changelog`), met à jour `CHANGELOG.md` ;
3. crée le commit `chore(version): X.Y.Z [skip ci]` et le tag annoté
   `vX.Y.Z`, poussés en un seul `git push --follow-tags` ;
4. déclenche un build COPR via `curl` sur l'URL de webhook (secret
   `COPR_WEBHOOK_URL`).

Le push du bot utilise le `GITHUB_TOKEN` : il ne re-déclenche pas Actions
(pas de boucle). COPR ne build donc que des commits de release, où
`Cargo.toml`, le `.spec` et le tag sont alignés — c'est pourquoi le
`.copr/Makefile` ne réécrit plus ces fichiers.

## Prérequis (configuration unique)

1. **Secret GitHub `COPR_WEBHOOK_URL`** : récupérer l'URL de webhook du
   package COPR (page du package → *Integrations* / webhook) et l'ajouter
   dans *Settings → Secrets and variables → Actions*.
2. **Désactiver l'auto-rebuild COPR sur push** : la CI est le seul
   déclencheur. Committish COPR laissé sur `main`.
3. **Branch protection** : s'assurer que `main` autorise le push du
   `GITHUB_TOKEN` (sinon prévoir un PAT dédié ou autoriser le bot).

## Faire une release

Rien de spécial : merger/pusher des commits *conventional* sur `main`. La
version et le tag sont gérés par la CI.
```

- [ ] **Step 2: Ajouter un pointeur dans `CLAUDE.md`**

Edit `CLAUDE.md` — insérer, juste avant la ligne `## Architecture`, la section :

```markdown
## Release

Releases automatiques au push sur `main` via la CI « Publish »
(cocogitto : bump `Cargo.toml`/`.spec`, tag `vX.Y.Z`, trigger COPR). Voir
`docs/RELEASING.md`. Ne jamais réintroduire de `sed` de version dans
`.copr/Makefile` : le bump est fait par la CI.

```

- [ ] **Step 3: Commit**

```bash
git add docs/RELEASING.md CLAUDE.md
git commit -m "docs(ci): documenter le process de release automatique"
```

---

## Notes de vérification (self-review)

- **Couverture spec** : trigger push main + permissions + concurrency (Task 3) ; anti-boucle via `GITHUB_TOKEN` + `[skip ci]` auto de cog (Tasks 2-3) ; bump semver conventional (Task 2) ; bump `Cargo.toml` (`cargo set-version`, Task 2) ; bump `.spec` Version + `%changelog` (Tasks 1-2) ; `CHANGELOG.md` (Task 2, `[changelog]`) ; commit+tag en un push `--follow-tags` (Task 2) ; trigger COPR explicite via secret (Tasks 2-3) ; retrait des deux `sed` du Makefile (Task 4) ; prérequis hors dépôt documentés (Task 5). No-op propre géré par le gate `--dry-run` (Task 3).
- **Invariant clé** : le retrait des `sed` (Task 4) n'est sûr que si COPR ne build que des commits de release — garanti par le déclenchement CI-only (auto-rebuild COPR désactivé, Task 5, prérequis 2). Si l'auto-rebuild COPR reste actif, un build sur commit non-taggé casserait (`Source0` introuvable) : ne pas retirer les `sed` sans avoir désactivé l'auto-rebuild.
- **Cohérence des noms** : `{{version}}` (hook cog) → `cargo set-version {{version}}` et `bash packaging/bump-spec.sh {{version}}` ; `bump-spec.sh <version> [spec-path]` appelé sans path → défaut `packaging/asciimeadow.spec` ; `COPR_WEBHOOK_URL` identique entre `cog.toml` (post-bump hook), `publish.yml` (env) et `docs/RELEASING.md` (secret).
```
