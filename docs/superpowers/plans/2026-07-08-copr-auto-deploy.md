# Auto-déploiement COPR sur tag — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publier automatiquement une nouvelle version RPM sur COPR à chaque tag git poussé, sans geste manuel de packaging.

**Architecture:** Un `.copr/Makefile` (méthode COPR `make_srpm`) génère le SRPM vendored côté serveur COPR : il dérive la version du tag, réaligne `Cargo.toml`/`Cargo.lock` (éphémère), `cargo vendor`, crée les tarballs source + vendor, injecte la version dans le spec, puis `rpmbuild -bs`. Un webhook GitHub (event *Branch or tag creation*) déclenche le build COPR.

**Tech Stack:** Fedora RPM packaging, `cargo-rpm-macros`, `cargo vendor`, GNU make, COPR SCM/webhook, `mock`.

## Global Constraints

- Le cœur reste pur (lib sans I/O) ; ce plan ne touche **aucun** code Rust de `src/`.
- Build RPM **vendored** (offline) : les deps sont embarquées dans un tarball vendor produit pendant la phase *srpm* (qui, elle, a le réseau).
- Chemin COPR = **`m4thia5/asciimeadow`** (minuscule ; le username est sensible à la casse).
- Licence du paquet : `MIT OR Apache-2.0`.
- Source unique de vérité pour la version = **le tag git** (`vX.Y.Z` → RPM `X.Y.Z`). Jamais de write-back dans le repo.
- Commentaires/docs en **français** (convention du repo).

---

### Task 1 : Finaliser le spec + corriger le chemin COPR du README

Groundwork packaging : committer le correctif `%install` déjà présent dans l'arbre de travail (installe le seul binaire) et corriger la casse du chemin COPR dans le README.

**Files:**
- Modify: `packaging/asciimeadow.spec` (déjà modifié dans l'arbre, non committé — bloc `%install`)
- Modify: `README.md:27-33` (bloc d'installation COPR)

**Interfaces:**
- Consumes: rien.
- Produces: un `packaging/asciimeadow.spec` propre, avec `Version: 0.1.0` comme valeur par défaut (écrasée au build par la Task 2) et un `%install` qui installe uniquement le binaire.

- [ ] **Step 1 : Vérifier le diff en attente du spec**

Run : `git diff -- packaging/asciimeadow.spec`
Attendu : le `%install` utilise `install -Dpm 0755 -t %{buildroot}%{_bindir} $(find target -type f -name %{crate} -perm -u+x)` (pas `%cargo_install`).

- [ ] **Step 2 : Corriger la casse du chemin COPR dans le README**

Remplacer dans `README.md` la ligne :

```
sudo dnf copr enable M4THIA5/asciimeadow
```

par :

```
sudo dnf copr enable m4thia5/asciimeadow
```

- [ ] **Step 3 : Sanity-check du spec (parse)**

Run : `rpmspec -q --qf '%{name} %{version}\n' packaging/asciimeadow.spec`
Attendu : `asciimeadow 0.1.0` (aucune erreur de parse).

Si `rpmspec` absent : `sudo dnf -y install rpm-build`.

- [ ] **Step 4 : Commit**

```bash
git add packaging/asciimeadow.spec README.md
git commit -m "fix(packaging): install binaire seul + chemin COPR minuscule"
```

---

### Task 2 : `.copr/Makefile` (génération du SRPM)

Le fichier que COPR exécute pour produire le SRPM. Testé localement dans un clone jetable (pour ne pas salir l'arbre de travail), puis validé par un build `mock` offline.

**Files:**
- Create: `.copr/Makefile`

**Interfaces:**
- Consumes: variables `outdir` et `spec` passées par COPR ; le clone git positionné sur le tag. `packaging/asciimeadow.spec` (Task 1).
- Produces: cible make `srpm` qui dépose `asciimeadow-<version>-1.*.src.rpm` dans `$(outdir)`. Invocation COPR : `make -f .copr/Makefile srpm outdir="<dir>" spec="packaging/asciimeadow.spec"`.

- [ ] **Step 1 : Écrire `.copr/Makefile`**

```makefile
# .copr/Makefile — génération du SRPM pour COPR (méthode « make_srpm ».
# COPR invoque :  make -f .copr/Makefile srpm outdir="<dir>" spec="<spec>"
# S'exécute en root dans un chroot mock AVEC réseau (phase srpm) : c'est
# ici que `cargo vendor` télécharge les deps pour un build RPM ensuite offline.

CRATE  := asciimeadow
spec   ?= packaging/asciimeadow.spec
outdir ?= $(CURDIR)

# Version = tag courant sans le préfixe « v ».
# Fallbacks : dernier tag accessible, puis version de Cargo.toml (jamais vide).
VERSION := $(shell git describe --exact-match --tags HEAD 2>/dev/null | sed 's/^v//')
ifeq ($(VERSION),)
VERSION := $(shell git describe --tags --always 2>/dev/null | sed 's/^v//')
endif
ifeq ($(VERSION),)
VERSION := $(shell grep -m1 '^version' Cargo.toml | sed -E 's/.*"(.*)".*/\1/')
endif

srpm:
	# Toolchain Rust (déjà présente en local → no-op ; installée dans le chroot COPR).
	command -v cargo >/dev/null 2>&1 || dnf -y install cargo
	# Aligner la version du paquet sur le tag (éphémère : dans ce clone uniquement).
	sed -i -E 's/^version = ".*"/version = "$(VERSION)"/' Cargo.toml
	# Télécharger les deps (réseau) ; re-locke la version racine dans Cargo.lock.
	cargo vendor vendor/
	# Tarball source, préfixe « asciimeadow-<version>/ », sans .git ni vendor.
	tar caf "$(outdir)/$(CRATE)-$(VERSION).tar.gz" \
		--exclude=./.git --exclude=./vendor --exclude='./*.tar.*' \
		--transform 's,^\.,$(CRATE)-$(VERSION),' .
	# Tarball des deps vendored.
	tar caf "$(outdir)/$(CRATE)-$(VERSION)-vendor.tar.xz" vendor/
	# Injecter la version dans le spec.
	sed -i -E 's/^Version:.*/Version:        $(VERSION)/' "$(spec)"
	# Construire le SRPM dans outdir.
	rpmbuild -bs "$(spec)" \
		--define "_sourcedir $(outdir)" \
		--define "_srcrpmdir $(outdir)"

.PHONY: srpm
```

- [ ] **Step 2 : Générer un SRPM dans un clone jetable (test local)**

Le Makefile modifie `Cargo.toml`/`spec` : on l'exécute dans un clone temporaire pour garder l'arbre de travail propre.

```bash
rm -rf /tmp/am-srpm && git clone --tags . /tmp/am-srpm
git -C /tmp/am-srpm checkout v0.1.0
mkdir -p /tmp/am-out
make -C /tmp/am-srpm -f /tmp/am-srpm/.copr/Makefile srpm \
     outdir=/tmp/am-out spec=packaging/asciimeadow.spec
```

Attendu : commande OK, se termine sur `Wrote: /tmp/am-out/asciimeadow-0.1.0-1.*.src.rpm`.

- [ ] **Step 3 : Vérifier la version et le contenu du SRPM**

```bash
rpm -qp --qf '%{NAME} %{VERSION}-%{RELEASE}\n' /tmp/am-out/asciimeadow-0.1.0-1.*.src.rpm
rpm -qlp /tmp/am-out/asciimeadow-0.1.0-1.*.src.rpm
```

Attendu : `asciimeadow 0.1.0-1.fc*` ; la liste contient `asciimeadow-0.1.0.tar.gz`, `asciimeadow-0.1.0-vendor.tar.xz` et `asciimeadow.spec`.

- [ ] **Step 4 : Build RPM offline via mock (prouve que le vendoring est complet)**

```bash
mock -r fedora-44-x86_64 /tmp/am-out/asciimeadow-0.1.0-1.*.src.rpm
```

Attendu : `Finish: run` / `INFO: Done`, un `asciimeadow-0.1.0-1.fc44.x86_64.rpm` dans `/var/lib/mock/fedora-44-x86_64/result/`. Le build ne fait aucun accès réseau (deps vendored).

Prérequis : appartenance au groupe `mock` (`sudo usermod -aG mock $USER` puis `newgrp mock`).

- [ ] **Step 5 : Commit**

```bash
git add .copr/Makefile
git commit -m "feat(packaging): .copr/Makefile pour build SRPM vendored (COPR make_srpm)"
```

---

### Task 3 : Câblage COPR + webhook + doc du process de release

Configuration manuelle COPR/GitHub (UI, une seule fois) et documentation du geste de release. La partie code de ce plan est terminée ; cette tâche relie les pièces et laisse une trace dans le repo.

**Files:**
- Modify: `README.md` (ajout d'une section « Publier une release »)

**Interfaces:**
- Consumes: `.copr/Makefile` (Task 2), le projet COPR `m4thia5/asciimeadow` déjà existant.
- Produces: un pipeline tag→publication opérationnel + une section README décrivant le geste de release.

- [ ] **Step 1 : Configurer le package SCM sur COPR**

Sur `https://copr.fedorainfracloud.org/coprs/m4thia5/asciimeadow/` → onglet **Packages** → **New package** :
- Provider : **SCM**
- Clone url : `https://github.com/M4THIA5/asciimeadow.git`
- Committish : (vide)
- Subdirectory : (vide)
- Spec File : `packaging/asciimeadow.spec`
- Build method (Type) : **make_srpm**
- Cocher **Auto-rebuild**
- **Save**

- [ ] **Step 2 : Récupérer l'URL du webhook COPR**

Projet COPR → **Settings** → **Integrations** → copier l'URL webhook (section GitHub) et le token/secret associé si affiché.

- [ ] **Step 3 : Ajouter le webhook côté GitHub (tag-only)**

Repo GitHub → **Settings** → **Webhooks** → **Add webhook** :
- Payload URL : l'URL de l'étape précédente
- Content type : `application/json`
- **Let me select individual events** → décocher *Pushes* → cocher **Branch or tag creation**
- **Add webhook**

Attendu : après ajout, GitHub affiche un ping avec réponse `200`.

- [ ] **Step 4 : Documenter le process de release dans le README**

Insérer après la section « Installation (Fedora / COPR) » de `README.md` :

```markdown
## Publier une release

Le déploiement COPR est automatique : pousser un tag `vX.Y.Z` déclenche
un webhook qui reconstruit et publie le paquet.

```bash
git tag v0.2.0
git push origin v0.2.0
```

La version RPM est dérivée du tag (`v0.2.0` → `0.2.0`) ; `Cargo.toml` n'a
pas besoin d'être édité (il est réaligné au build). Le build apparaît sur
https://copr.fedorainfracloud.org/coprs/m4thia5/asciimeadow/builds/ ;
côté utilisateur, `sudo dnf upgrade asciimeadow` récupère la nouvelle version.
```
```

- [ ] **Step 5 : Commit**

```bash
git add README.md
git commit -m "docs(packaging): documenter le process de release par tag"
```

- [ ] **Step 6 : Test bout-en-bout (tag jetable)**

```bash
git tag v0.1.1
git push origin v0.1.1
```

Attendu : sous 1–2 min, un nouveau build `asciimeadow 0.1.1-1` apparaît automatiquement dans la liste des builds COPR et réussit sur `fedora-44-x86_64`. Puis :

```bash
sudo dnf upgrade asciimeadow   # récupère 0.1.1
```

Si le build n'apparaît pas : vérifier dans GitHub → Webhooks → *Recent Deliveries* que la livraison est `200` ; côté COPR, que le package a bien *Auto-rebuild* activé.

---

## Notes d'implémentation

- **Version vide impossible :** la triple cascade `--exact-match` → `--always` → `Cargo.toml` garantit un `$(VERSION)` non vide même si le clone COPR est peu profond.
- **`rpmbuild -bs` seulement** : la phase srpm ne lance pas `%build`/`%check`/`%install`, donc rapide et sans TTY. Les tests (`%cargo_test`) tournent dans la phase build RPM (mock/COPR), sur le cœur pur sans TTY.
- **Idempotence version/Cargo.lock** : `cargo vendor` re-résout après le `sed` de `Cargo.toml`, donc `Cargo.lock` embarqué dans le tarball source porte la même version → build `%cargo_build` cohérent offline.
