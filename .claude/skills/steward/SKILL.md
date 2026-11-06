---
name: steward
description: Conventions du dépôt pour mener une PR jusqu'à la fusion — branche, commits, régénération des fichiers générés, conflits, CI rouge, retours de relecture.
---

# Mener une PR

- **Avant chaque push** : `mise run check` vert (plus `mise run openapi` / `mise run deny` selon le changement).
  Un push qui casse la CI coûte un cycle : vérifie d'abord.
- **Commits** : français, `type(module): résumé`, corps qui explique le pourquoi ; aucun nom de modèle d'IA.
- **Branche de quelqu'un d'autre** : jamais de rebase, d'amend ni de force-push ; on fusionne `main` dedans. Sur sa
  propre branche, une branche dont la PR est déjà fusionnée repart de `origin/main`.
- **Conflits** : fichiers générés (`ui/openapi.json`, `ui/src/routeTree.gen.ts`, `Cargo.lock`, `ui/package-lock.json`)
  → prendre une version puis **régénérer** (`mise run openapi`, `npm run build` dans `ui/`, `cargo check`,
  `npm install`), jamais de résolution à la main. Migrations : deux branches qui créent le même numéro → renumérote
  la tienne (si elle n'est pas encore sur `main`).
- **CI rouge** : reproduis en local avec la même tâche (`mise run check:rust` / `check:ui`), corrige la cause. Pas de
  test désactivé, pas de relance « pour voir » d'un échec réel.
- **Relecture** : petites demandes appliquées et poussées ; demandes larges (refonte, changement d'API) → proposer
  avant de faire. Répondre à chaque fil traité.
- **Description de PR** : sections « Ce qui change », « Tests », et ce qui n'a pas pu être vérifié (Discord réel, API
  externe…). Captures pour un changement visible du site.
