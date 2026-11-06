---
name: verifier
description: Vérifier une modification du bot ou du site avant de terminer, de commiter ou d'ouvrir une PR — quelles commandes lancer selon ce qui a changé, et comment lire les échecs courants.
---

# Vérifier avant de terminer

1. Lance la vérification qui correspond au changement :

   | Changement | Commande |
   |---|---|
   | N'importe quoi, avant commit/PR | `mise run check` (Rust et interface en parallèle) |
   | Rust seulement | `mise run check:rust` |
   | Interface seulement | `mise run check:ui` |
   | Route ou forme JSON de l'API | `mise run openapi` puis `mise run check` |
   | Dépendance Rust ajoutée ou mise à jour | `mise run deny` |

2. Échecs fréquents et leur cause :
   - `openapi_spec_is_up_to_date` : la spécification a changé → `mise run openapi`, commite `ui/openapi.json`.
   - `check:api` (tsc sur le client généré) : un type de `ui/src/api/types*.ts` ne correspond plus → aligne le type
     sur la spécification, pas l'inverse.
   - `command_definitions_are_valid` : limite Discord dépassée (25 options, description > 100 caractères, nom invalide).
   - Clippy `unwrap_used` : remplace par `?` ou `expect("raison de l'invariant")`.
   - Oxlint `max-lines` : découpe le fichier (nouveau composant ou `*-groupe.ts`), ne désactive pas la règle.
   - `check:compiler` : un composant fait quelque chose que le React Compiler refuse (mutation pendant le rendu,
     lecture de ref) → corrige le composant.

3. Ne contourne jamais : pas de `#[allow]`, `// oxlint-disable`, test ignoré ou `--no-verify` pour passer au vert. Si
   un contournement est vraiment justifié, il porte un commentaire qui dit pourquoi, et tu le signales.

4. Relis ton diff (`git diff`) avant de conclure, ou lance le sous-agent `relecteur`.
