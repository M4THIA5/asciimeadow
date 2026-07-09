# Auto-déploiement COPR à chaque release (tag)

Date : 2026-07-08

## Contexte

`asciimeadow` est packagé pour Fedora via COPR (`m4thia5/asciimeadow`). Le premier
build a été poussé à la main : générer le SRPM localement (`cargo vendor` +
`rpmbuild -bs`) puis `copr-cli build`. Objectif : **à chaque nouvelle version,
le paquet est reconstruit et publié automatiquement**, sans geste manuel de
packaging.

Décisions arrêtées en amont :

- **Déclencheur = tag git** (`v*`). Une release = un tag ; les commits de travail
  sur `main` ne déclenchent rien.
- **Mécanisme = webhook COPR + `.copr/Makefile`** (méthode `make_srpm`). Tout se
  passe côté serveur COPR ; aucun secret stocké dans GitHub, aucune minute CI.
- **Version = dérivée du tag**, injectée au build. `Cargo.toml` et `Cargo.lock`
  sont réalignés sur cette version **au moment du build uniquement** (dans le clone
  éphémère de COPR) — jamais de commit retour dans le repo.

## Flux

```
git tag v0.2.0 && git push origin v0.2.0
        │
        ▼
GitHub webhook  (event « Branch or tag creation »)
        │
        ▼
COPR  →  clone du repo au tag  →  make -f .copr/Makefile srpm outdir=… spec=…
        │        (chroot mock, root, réseau disponible)
        ▼
SRPM vendored, versionné depuis le tag
        │
        ▼
build RPM sur les chroots (fedora-44-x86_64, …)  →  publié dans le repo COPR
        │
        ▼
côté utilisateur :  dnf upgrade asciimeadow   →  nouvelle version
```

La phase *srpm* de COPR dispose du réseau (contrairement à la phase *build* du RPM
qui est offline) : c'est là que `cargo vendor` télécharge les dépendances, ce qui
permet ensuite un build RPM 100 % hors-ligne à partir du tarball vendored.

## Composants

### 1. `.copr/Makefile` (nouveau)

Cible `srpm` invoquée par COPR : `make -f .copr/Makefile srpm outdir="<dir>" spec="<spec>"`.
Étapes :

1. `dnf -y install cargo` — toolchain Rust dans le chroot srpm.
2. Déterminer la version depuis le tag :
   `VERSION = git describe --exact-match --tags HEAD | sed 's/^v//'`
   (fallback `git describe --tags --always | sed 's/^v//'` pour robustesse hors-tag).
3. Réécrire la version dans `Cargo.toml` (`sed` de la ligne `version = …`).
4. `cargo vendor vendor/` — télécharge les deps (online) et **re-locke la version
   du paquet racine dans `Cargo.lock`**, gardant `Cargo.toml`/`Cargo.lock` cohérents.
5. Créer le tarball source `asciimeadow-$VERSION.tar.gz`, préfixe `asciimeadow-$VERSION/`,
   en excluant `.git/`, `vendor/` et les tarballs — il embarque les `Cargo.toml`/`Cargo.lock`
   modifiés.
6. Créer `asciimeadow-$VERSION-vendor.tar.xz` depuis `vendor/`.
7. Réécrire la ligne `Version:` du spec = `$VERSION` (`sed`).
8. `rpmbuild -bs "$spec"` avec `_sourcedir` pointant sur les tarballs et
   `_srcrpmdir=$(outdir)` → SRPM déposé dans `$(outdir)` où COPR l'attend.

**Interface :** entrée = variables `outdir` et `spec` passées par COPR + le clone git
au tag. Sortie = un `.src.rpm` dans `$(outdir)`. Le Makefile ne dépend de rien d'autre
que d'un chroot Fedora avec réseau (COPR le garantit).

### 2. `packaging/asciimeadow.spec` (existant, ajustements mineurs)

- `Version: 0.1.0` reste comme **valeur par défaut** — permet un `rpmbuild` manuel
  hors COPR. Le `.copr/Makefile` l'écrase à chaque build automatique.
- Inclut le commit du correctif `%install` déjà présent dans l'arbre de travail
  (installation du seul binaire, pas de `%cargo_install` qui enregistrerait les
  sources de la lib dans le registry cargo).
- Le reste (vendored, `%cargo_prep -v vendor`, `%cargo_build`, `%cargo_test`) est
  inchangé.

### 3. Configuration COPR (manuel, une seule fois)

Dans le projet `m4thia5/asciimeadow`, ajouter un **package SCM** :

- Clone URL : `https://github.com/M4THIA5/asciimeadow.git`
- Build method : **`make_srpm`**
- Spec File : `packaging/asciimeadow.spec`
- Subdirectory : (racine)
- Auto-rebuild : **activé**

### 4. Webhook GitHub (manuel, une seule fois)

Repo GitHub → Settings → Webhooks → Add webhook :

- Payload URL : l'URL webhook du projet COPR (COPR → Settings → Integrations).
- Content type : `application/json`
- Events : **« Let me select individual events »** → cocher **« Branch or tag creation »**
  (tag-only ; les push de commits sur `main` ne déclenchent pas).

## Versioning

- **Source unique de vérité = le tag git.** `v0.2.0` → RPM `0.2.0-1`.
- `Cargo.toml` et `Cargo.lock` alignés sur la version au build (éphémère, dans le
  clone COPR ; jamais committé). Le binaire porte donc la bonne version dans ses
  métadonnées.
- `Release` reste toujours `1%{?dist}` par version. Re-tagger une version déjà
  publiée entre en collision (cas limite assumé : on ne re-tague pas).

## Vérification

1. **Local** — sur un checkout du tag :
   `make -f .copr/Makefile srpm outdir=/tmp/out spec=packaging/asciimeadow.spec`
   produit un SRPM à la bonne version ; `mock -r fedora-44-x86_64 …src.rpm` le
   build offline jusqu'au bout.
2. **Bout-en-bout** — `git tag v0.1.1 && git push origin v0.1.1` → un build apparaît
   automatiquement sur COPR, réussit sur `fedora-44-x86_64`, et
   `sudo dnf upgrade asciimeadow` récupère `0.1.1`.

## Hors périmètre

- Pas de write-back de `Cargo.toml` dans le repo (nécessiterait un token push).
- Pas de génération automatique de `%changelog` (le décalage version/changelog est
  un simple avertissement rpmlint, le build passe).
- Pas de GitHub Actions (le webhook COPR suffit et évite tout secret).
- Migration vers Fedora officiel (unbundled, package review) : hors sujet, cf. le
  doc de packaging initial.
