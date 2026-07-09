# Release

Les releases sont **automatiques**. Un push sur `main` déclenche la CI
« Publish » (`.github/workflows/publish.yml`), qui, via cocogitto :

1. calcule la prochaine version depuis les *conventional commits* depuis le
   dernier tag (`feat` → mineur, `fix` → correctif, `BREAKING CHANGE` →
   majeur ; aucun commit releasable → aucune release) ;
2. bumpe `Cargo.toml` (+ `Cargo.lock`) et `packaging/asciimeadow.spec`
   (`Version:` + entrée `%changelog`), met à jour `CHANGELOG.md` ;
3. crée le commit `chore(version): X.Y.Z [skip ci]` et le tag `vX.Y.Z`, puis
   pousse les deux ensemble (`git push --atomic origin main vX.Y.Z` ; cog crée
   un tag *léger*, poussé explicitement) ;
4. déclenche un build COPR via `curl` sur l'URL de webhook (secret
   `COPR_WEBHOOK_URL`).

Le push du bot utilise le `GITHUB_TOKEN` : il ne re-déclenche pas Actions
(pas de boucle ; le `[skip ci]` du message de commit, ajouté par
`cog bump --skip-ci`, sert de ceinture-bretelles). COPR ne build donc que des
commits de release, où `Cargo.toml`, le `.spec` et le tag sont alignés —
c'est pourquoi le `.copr/Makefile` ne réécrit plus ces fichiers.

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

## Reprise en cas d'échec

Le job échoue **avant** tout bump si le secret `COPR_WEBHOOK_URL` manque, et
le push commit + tag est atomique : on ne se retrouve donc pas avec un commit
de version sans son tag. Le seul état partiel possible est *tag poussé mais
`curl` COPR en échec* (COPR indisponible, webhook temporairement KO). Dans ce
cas la release git est complète (re-run du workflow → aucun nouveau bump, le
tag existe) mais COPR n'a pas buildé : **re-déclencher le build manuellement**,
soit depuis l'interface COPR (*Rebuild*), soit en re-POSTant le webhook :
`curl -X POST <COPR_WEBHOOK_URL>`.
